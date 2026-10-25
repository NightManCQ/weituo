use std::env;

use axum::{Json, extract::State};
use chrono::{Duration as ChronoDuration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use sqlx::Row;
use uuid::Uuid;

use crate::共享::*;

pub(super) async fn 获取实时凭据(
    State(状态): State<应用状态>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<实时连接信息>, 应用错误> {
    let 用户 = 请求
        .extensions()
        .get::<当前用户>()
        .cloned()
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;
    let 主机地址 =
        env::var("MQTT_PUBLIC_URL").unwrap_or_else(|_| "ws://localhost:9001/mqtt".into());
    let 密钥 = env::var("MQTT_JWT_SECRET")
        .map_err(|_| 应用错误::内部错误(anyhow::anyhow!("必须配置 MQTT_JWT_SECRET")))?;
    if 密钥.len() < 32 {
        return Err(应用错误::内部错误(anyhow::anyhow!(
            "MQTT_JWT_SECRET 至少需要 32 字节"
        )));
    }
    let 任务记录集 = sqlx::query(
        r#"SELECT "任务编号" FROM "任务"
           WHERE ("发布者编号" = $1 OR "接单者编号" = $1)
           ORDER BY "更新时间" DESC LIMIT 100"#,
    )
    .bind(用户.编号)
    .fetch_all(&状态.数据库)
    .await?;
    let 主题列表: Vec<String> = 任务记录集
        .iter()
        .flat_map(|记录| {
            let 编号: Uuid = 记录.get("任务编号");
            [
                format!("renwu/{编号}/events"),
                format!("renwu/{编号}/messages"),
            ]
        })
        .collect();
    let mut 主题列表: Vec<String> = 主题列表;
    主题列表.push(format!("renwu/users/{}/notifications", 用户.编号));
    let 过期时间 = (Utc::now() + ChronoDuration::minutes(5)).timestamp() as usize;
    let 消息令牌 = encode(
        &Header::default(),
        &消息令牌声明 {
            过期时间,
            用户名: 用户.编号.to_string(),
            访问控制: 消息访问控制 {
                发布权限: Vec::new(),
                订阅权限: 主题列表.clone(),
            },
        },
        &EncodingKey::from_secret(密钥.as_bytes()),
    )
    .map_err(|e| 应用错误::内部错误(anyhow::anyhow!(e.to_string())))?;
    Ok(Json(实时连接信息 {
        消息地址: 主机地址,
        消息令牌,
        主题列表,
        有效秒数: 300,
        说明: "EMQX 必须配置 MQTT_JWT_SECRET 对应的 JWT 认证器并启用 ACL claim；仅订阅当前用户参与的任务主题及个人通知主题",
    }))
}
