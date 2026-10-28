use anyhow::{Context, Result};
use apalis::prelude::*;
use apalis_postgres::PostgresStorage;
use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header, encode};
use rumqttc::{AsyncClient, Event, MqttOptions, Outgoing, Packet, QoS, Transport};
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use std::{
    collections::{HashMap, VecDeque},
    env,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    sync::oneshot,
    time::{sleep, timeout},
};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::共享::*;

async fn 发布待投递事件(状态: &工作者状态) -> Result<()> {
    let 记录集 = sqlx::query(
        r#"SELECT "事件编号", "主题", "事件内容"
           FROM "事务发件箱"
           WHERE "投递状态" = '待投递' AND "下次尝试时间" <= now()
           ORDER BY "创建时间"
           LIMIT 50"#,
    )
    .fetch_all(&状态.数据库)
    .await?;
    for 记录 in 记录集 {
        let 编号: Uuid = 记录.get("事件编号");
        发布单个发件箱事件(&状态.数据库, &状态.消息代理, 编号).await?;
    }
    Ok(())
}

pub(crate) async fn 发布单个发件箱事件(
    数据库: &PgPool,
    消息代理: &消息发布器,
    事件编号: Uuid,
) -> Result<bool> {
    let mut 事务 = 数据库.begin().await?;
    let 已领取记录 = sqlx::query(
        r#"UPDATE "事务发件箱" SET "重试次数" = "重试次数" + 1,
                  "下次尝试时间" = now() + interval '30 seconds'
           WHERE "事件编号" = $1 AND "投递状态" = '待投递'
             AND "下次尝试时间" <= now()
           RETURNING "主题", "事件内容""#,
    )
    .bind(事件编号)
    .fetch_optional(&mut *事务)
    .await?;
    let Some(已领取记录) = 已领取记录 else {
        事务.rollback().await?;
        return Ok(false);
    };
    let 主题: String = 已领取记录.get("主题");
    let 事件内容: Value = 已领取记录.get("事件内容");
    事务.commit().await?;
    match 消息代理.确认发布消息(事件编号, &主题, 事件内容).await {
        Ok(()) => {
            sqlx::query(
                r#"UPDATE "事务发件箱" SET "投递状态" = '已投递', "投递时间" = now()
                   WHERE "事件编号" = $1"#,
            )
            .bind(事件编号)
            .execute(数据库)
            .await?;
            info!(%事件编号, %主题, "MQTT outbox event delivered");
            Ok(true)
        }
        Err(err) => {
            warn!(%事件编号, %主题, error = %err, "MQTT publish failed; event will retry");
            sqlx::query(
                r#"UPDATE "事务发件箱" SET "下次尝试时间" = now() + interval '5 seconds'
                   WHERE "事件编号" = $1 AND "投递状态" = '待投递'"#,
            )
            .bind(事件编号)
            .execute(数据库)
            .await?;
            Ok(false)
        }
    }
}

async fn 处理过期任务(数据库: &PgPool) -> Result<()> {
    loop {
        let mut 事务 = 数据库.begin().await?;
        let 记录集 = sqlx::query(
            r#"SELECT t."任务编号", t."发布者编号",
                      (SELECT g."群组编号" FROM "群_群组" g
                       WHERE g."关联任务编号" = t."任务编号") AS "群组编号"
               FROM "任务" t
               WHERE t."任务状态" = '待接单' AND t."截止时间" <= now()
               ORDER BY t."截止时间"
               LIMIT 50
               FOR UPDATE OF t SKIP LOCKED"#,
        )
        .fetch_all(&mut *事务)
        .await?;
        if 记录集.is_empty() {
            事务.commit().await?;
            return Ok(());
        }

        for 记录 in &记录集 {
            let 任务编号: Uuid = 记录.get("任务编号");
            let 发布者编号: Uuid = 记录.get("发布者编号");
            let 群组编号: Option<Uuid> = 记录.get("群组编号");
            let 主题 = 构造任务事件主题(群组编号, 发布者编号);
            crate::接口::积分::退款委托报酬(&mut 事务, 任务编号, None, None).await?;
            let 事件编号 = Uuid::new_v4();
            let 更新结果 = sqlx::query(
                r#"UPDATE "任务"
                   SET "任务状态" = '已过期', "版本号" = "版本号" + 1, "更新时间" = now()
                   WHERE "任务编号" = $1 AND "任务状态" = '待接单'"#,
            )
            .bind(任务编号)
            .execute(&mut *事务)
            .await?;
            if 更新结果.rows_affected() == 0 {
                continue;
            }
            sqlx::query(
                r#"INSERT INTO "任务事件"
                   ("事件编号", "任务编号", "操作者编号", "事件类型", "原状态", "新状态")
                   VALUES ($1, $2, NULL, 'task.expired', '待接单', '已过期')"#,
            )
            .bind(事件编号)
            .bind(任务编号)
            .execute(&mut *事务)
            .await?;
            let 事件 = json!({
                "事件编号": 事件编号,
                "事件类型": "task.expired",
                "结构版本": 2,
                "发生时间": Utc::now(),
                "任务编号": 任务编号,
                "数据": {"任务状态": "已过期"}
            });
            sqlx::query(
                r#"INSERT INTO "事务发件箱" ("事件编号", "事件类型", "主题", "事件内容")
                   VALUES ($1, 'task.expired', $2, $3)"#,
            )
            .bind(事件编号)
            .bind(主题)
            .bind(事件)
            .execute(&mut *事务)
            .await?;
            info!(%任务编号, %发布者编号, "task expired");
        }
        事务.commit().await?;
    }
}

async fn 处理发件箱任务(
    后台任务: 发件箱任务,
    状态: Data<工作者状态>,
) -> Result<(), BoxDynError> {
    let _ = 后台任务.事件编号;
    处理过期任务(&状态.数据库)
        .await
        .map_err(|e| Box::new(std::io::Error::other(e.to_string())) as BoxDynError)?;
    发布待投递事件(&状态)
        .await
        .map_err(|e| Box::new(std::io::Error::other(e.to_string())) as BoxDynError)
}

pub(crate) async fn 连接消息代理() -> Result<消息发布器> {
    let 消息主机 = env::var("MQTT_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
    let 消息端口 = env::var("MQTT_PORT")
        .unwrap_or_else(|_| "1883".to_owned())
        .parse::<u16>()
        .context("MQTT_PORT 必须为有效端口")?;
    let 消息用户名 = env::var("MQTT_USERNAME").unwrap_or_else(|_| "renwu_worker".to_owned());
    let 消息签名密钥 = env::var("MQTT_JWT_SECRET").context("必须配置 MQTT_JWT_SECRET")?;
    anyhow::ensure!(消息签名密钥.len() >= 32, "MQTT_JWT_SECRET 至少需要 32 字节");
    let 消息令牌 = encode(
        &Header::default(),
        &消息令牌声明 {
            过期时间: None,
            用户名: 消息用户名.clone(),
            访问控制: 消息访问控制 {
                发布权限: vec![
                    "renwu/+/events".to_owned(),
                    "q/+".to_owned(),
                    "u/+".to_owned(),
                ],
                订阅权限: Vec::new(),
            },
        },
        &EncodingKey::from_secret(消息签名密钥.as_bytes()),
    )
    .context("生成 MQTT worker JWT 失败")?;
    let mut 消息连接选项 = MqttOptions::new(
        env::var("MQTT_CLIENT_ID").unwrap_or_else(|_| "renwu-backend".to_owned()),
        消息主机,
        消息端口,
    );
    消息连接选项.set_keep_alive(Duration::from_secs(30));
    消息连接选项.set_credentials(消息用户名, 消息令牌);
    if env::var("MQTT_TLS").is_ok_and(|值| 值.eq_ignore_ascii_case("true")) {
        消息连接选项.set_transport(Transport::tls_with_default_config());
    }
    let (客户端, mut 事件循环) = AsyncClient::new(消息连接选项, 100);
    let 待发送队列: 待发布队列 = Arc::new(Mutex::new(VecDeque::new()));
    let 等待确认表: 发布确认表 = Arc::new(Mutex::new(HashMap::new()));
    let 待处理事件队列 = Arc::clone(&待发送队列);
    let 事件确认表 = Arc::clone(&等待确认表);
    tokio::spawn(async move {
        loop {
            match 事件循环.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => info!("MQTT connected"),
                Ok(Event::Incoming(Packet::PubAck(ack))) => {
                    if let Some((事件编号, 发送者)) = 事件确认表
                        .lock()
                        .expect("MQTT acknowledgement lock poisoned")
                        .remove(&ack.pkid)
                    {
                        info!(%事件编号, packet_id = ack.pkid, "MQTT QoS 1 event delivery acknowledged");
                        let _ = 发送者.send(());
                    }
                }
                Ok(Event::Outgoing(Outgoing::Publish(packet_id))) => {
                    if let Some(待确认项) = 待处理事件队列
                        .lock()
                        .expect("MQTT pending queue lock poisoned")
                        .pop_front()
                    {
                        事件确认表
                            .lock()
                            .expect("MQTT acknowledgement lock poisoned")
                            .insert(packet_id, 待确认项);
                    }
                }
                Ok(_) => {}
                Err(err) => {
                    warn!(error = %err, "MQTT disconnected; retrying");
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }
    });
    Ok(消息发布器 {
        客户端,
        待发送队列,
        发布锁: Arc::new(tokio::sync::Mutex::new(())),
    })
}

impl 消息发布器 {
    pub(crate) async fn 确认发布消息(
        &self,
        事件编号: Uuid,
        主题: &str,
        事件内容: Value,
    ) -> Result<()> {
        let _guard = self.发布锁.lock().await;
        let (发送者, receiver) = oneshot::channel();
        self.待发送队列
            .lock()
            .expect("MQTT pending queue lock poisoned")
            .push_back((事件编号, 发送者));

        if let Err(err) = self
            .客户端
            .publish(
                主题,
                QoS::AtLeastOnce,
                false,
                serde_json::to_vec(&事件内容)?,
            )
            .await
        {
            self.待发送队列
                .lock()
                .expect("MQTT pending queue lock poisoned")
                .retain(|(pending_id, _)| *pending_id != 事件编号);
            return Err(err.into());
        }

        match timeout(Duration::from_secs(15), receiver).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) => Err(anyhow::anyhow!("MQTT QoS 1 acknowledgement channel closed")),
            Err(_) => Err(anyhow::anyhow!("MQTT QoS 1 acknowledgement timed out")),
        }
    }
}

async fn 运行工作循环(数据库: PgPool, 消息代理: 消息发布器) -> Result<()> {
    PostgresStorage::<()>::setup(&数据库)
        .await
        .context("Apalis storage setup failed")?;
    let 存储后端 = PostgresStorage::<发件箱任务>::new(&数据库)
        .with_config(apalis_postgres::Config::default().queue("renwu-outbox"));
    let 状态 = 工作者状态 {
        数据库: 数据库.clone(),
        消息代理,
    };
    let 工作者编号 = format!("renwu-outbox-worker-{}", Uuid::new_v4());
    let 工作者 = WorkerBuilder::new(工作者编号)
        .backend(存储后端)
        .data(状态.clone())
        .build(处理发件箱任务);
    let mut 入队存储 = PostgresStorage::<发件箱任务>::new(&数据库)
        .with_config(apalis_postgres::Config::default().queue("renwu-outbox"));
    tokio::spawn(async move {
        loop {
            let 后台任务 = TaskBuilder::new(发件箱任务 {
                事件编号: Uuid::nil(),
            })
            .build();
            if let Err(err) = 入队存储.push_task(后台任务).await {
                error!(error = %err, "failed to enqueue outbox scan");
            }
            sleep(Duration::from_secs(2)).await;
        }
    });
    工作者.run().await.context("Apalis worker stopped")?;
    Ok(())
}

pub(crate) async fn 启动(数据库: PgPool, 消息代理: 消息发布器) -> Result<()> {
    运行工作循环(数据库, 消息代理).await
}
