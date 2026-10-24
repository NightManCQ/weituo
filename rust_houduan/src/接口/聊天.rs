use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::共享::*;

pub(super) async fn 查询我的聊天会话(
    State(状态): State<应用状态>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<聊天会话视图>>, 应用错误> {
    let 用户 = 请求
        .extensions()
        .get::<当前用户>()
        .cloned()
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;
    let 记录集 = sqlx::query(
        r#"SELECT t."任务编号", t."标题", t."任务状态",
                  联系人."用户编号" AS "联系人编号", 联系人."昵称" AS "联系人昵称",
                  最近消息."消息正文" AS "最近消息",
                  最近消息."创建时间" AS "最近消息时间"
           FROM "任务" t
           JOIN "用户" 联系人
             ON 联系人."用户编号" = CASE
                 WHEN t."发布者编号" = $1 THEN t."接单者编号"
                 ELSE t."发布者编号"
             END
           JOIN LATERAL (
               SELECT "消息正文", "创建时间"
               FROM "聊天消息"
               WHERE "任务编号" = t."任务编号"
               ORDER BY "创建时间" DESC, "消息编号" DESC
               LIMIT 1
           ) 最近消息 ON TRUE
           WHERE (t."发布者编号" = $1 OR t."接单者编号" = $1)
           ORDER BY 最近消息."创建时间" DESC, t."任务编号" DESC
           LIMIT 200"#,
    )
    .bind(用户.编号)
    .fetch_all(&状态.数据库)
    .await?;
    Ok(Json(
        记录集
            .iter()
            .map(|记录| 聊天会话视图 {
                任务编号: 记录.get("任务编号"),
                任务标题: 记录.get("标题"),
                任务状态: 记录.get("任务状态"),
                联系人编号: 记录.get("联系人编号"),
                联系人昵称: 记录.get("联系人昵称"),
                最近消息: 记录.get("最近消息"),
                最近消息时间: 记录.get("最近消息时间"),
            })
            .collect(),
    ))
}

pub(crate) async fn 校验并保存聊天消息(
    数据库: &PgPool,
    任务编号: Uuid,
    用户编号: Uuid,
    请求: 聊天上行请求,
) -> Result<(聊天消息视图, Option<(Uuid, serde_json::Value)>), 应用错误> {
    let 正文 = 请求.消息正文.trim();
    if 正文.is_empty()
        || 正文.chars().count() > 4000
        || 请求.客户端消息编号.is_empty()
        || 请求.客户端消息编号.len() > 100
    {
        return Err(应用错误::请求错误(
            "消息不能为空且最多 4000 个字符，客户端消息编号最多 100 字节".into(),
        ));
    }

    let mut 事务 = 数据库.begin().await?;
    let 任务 = sqlx::query(
        r#"SELECT u."昵称"
           FROM "任务" t
           JOIN "用户" u ON u."用户编号" = $2 AND u."账号状态" = '正常'
           WHERE t."任务编号" = $1
             AND (t."发布者编号" = $2 OR t."接单者编号" = $2)
           FOR SHARE OF t"#,
    )
    .bind(任务编号)
    .bind(用户编号)
    .fetch_optional(&mut *事务)
    .await?
    .ok_or_else(|| 应用错误::禁止访问("账号不可用或不是任务参与者".into()))?;
    let 昵称: String = 任务.get("昵称");
    let 新消息编号 = Uuid::new_v4();
    let 记录 = sqlx::query(
        r#"INSERT INTO "聊天消息"
           ("消息编号", "任务编号", "发送者编号", "客户端消息编号", "消息正文")
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT ("任务编号", "发送者编号", "客户端消息编号") DO UPDATE
             SET "客户端消息编号" = EXCLUDED."客户端消息编号"
           RETURNING "消息编号", "任务编号", "发送者编号", "客户端消息编号",
                     "消息正文", "创建时间""#,
    )
    .bind(新消息编号)
    .bind(任务编号)
    .bind(用户编号)
    .bind(&请求.客户端消息编号)
    .bind(正文)
    .fetch_one(&mut *事务)
    .await?;
    let 消息编号: Uuid = 记录.get("消息编号");
    if 记录.get::<String, _>("消息正文") != 正文 {
        return Err(应用错误::冲突(
            "该客户端消息编号已用于其他消息内容".into(),
        ));
    }

    let 待发布事件 = if 消息编号 == 新消息编号 {
        let 事件编号 = Uuid::new_v4();
        let 事件 = json!({
            "事件编号": 事件编号,
            "事件类型": "chat.message_created",
            "结构版本": 2,
            "发生时间": Utc::now(),
            "任务编号": 任务编号,
            "数据": {
                "消息编号": 消息编号,
                "任务编号": 任务编号,
                "发送者编号": 用户编号,
                "发送者昵称": 昵称,
                "客户端消息编号": 记录.get::<String, _>("客户端消息编号"),
                "消息正文": 记录.get::<String, _>("消息正文"),
                "创建时间": 记录.get::<DateTime<Utc>, _>("创建时间")
            }
        });
        sqlx::query(
            r#"INSERT INTO "事务发件箱" ("事件编号", "事件类型", "主题", "事件内容")
               VALUES ($1, 'chat.message_created', $2, $3)"#,
        )
        .bind(事件编号)
        .bind(format!("renwu/{任务编号}/messages"))
        .bind(事件.clone())
        .execute(&mut *事务)
        .await?;
        Some((事件编号, 事件))
    } else {
        let 待投递事件 = sqlx::query(
            r#"SELECT "事件编号", "事件内容"
               FROM "事务发件箱"
               WHERE "投递状态" = '待投递'
                 AND "事件内容" #>> '{数据,消息编号}' = $1
               LIMIT 1"#,
        )
        .bind(消息编号.to_string())
        .fetch_optional(&mut *事务)
        .await?;
        待投递事件.map(|记录| (记录.get("事件编号"), 记录.get("事件内容")))
    };
    事务.commit().await?;
    Ok((
        聊天消息视图 {
            消息编号,
            任务编号: 记录.get("任务编号"),
            发送者编号: 记录.get("发送者编号"),
            发送者昵称: 昵称,
            客户端消息编号: 记录.get("客户端消息编号"),
            消息正文: 记录.get("消息正文"),
            创建时间: 记录.get("创建时间"),
        },
        待发布事件,
    ))
}

pub(super) async fn 查询聊天消息(
    State(状态): State<应用状态>,
    Path(任务编号): Path<Uuid>,
    Query(查询条件): Query<聊天消息查询>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<聊天消息视图>>, 应用错误> {
    let 用户 = 请求
        .extensions()
        .get::<当前用户>()
        .cloned()
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;
    let 参与关系 = sqlx::query(
        r#"SELECT 1 FROM "任务" WHERE "任务编号" = $1
           AND ("发布者编号" = $2 OR "接单者编号" = $2)"#,
    )
    .bind(任务编号)
    .bind(用户.编号)
    .fetch_optional(&状态.数据库)
    .await?;
    if 参与关系.is_none() {
        return Err(应用错误::禁止访问(
            "只有任务参与者可以查看聊天".into(),
        ));
    }
    let 数量上限 = 查询条件.数量上限.unwrap_or(100).clamp(1, 200);
    let 记录集 = sqlx::query(
        r#"SELECT m."消息编号", m."任务编号", m."发送者编号", u."昵称",
                  m."客户端消息编号", m."消息正文", m."创建时间"
           FROM (
               SELECT "消息编号", "任务编号", "发送者编号", "客户端消息编号",
                      "消息正文", "创建时间"
               FROM "聊天消息"
               WHERE "任务编号" = $1
                 AND ($2::timestamptz IS NULL OR "创建时间" > $2)
               ORDER BY "创建时间" DESC, "消息编号" DESC
               LIMIT $3
           ) m
           JOIN "用户" u ON u."用户编号" = m."发送者编号"
           ORDER BY m."创建时间", m."消息编号""#,
    )
    .bind(任务编号)
    .bind(查询条件.起始时间)
    .bind(数量上限)
    .fetch_all(&状态.数据库)
    .await?;
    Ok(Json(
        记录集
            .into_iter()
            .map(|记录| 聊天消息视图 {
                消息编号: 记录.get("消息编号"),
                任务编号: 记录.get("任务编号"),
                发送者编号: 记录.get("发送者编号"),
                发送者昵称: 记录.get("昵称"),
                客户端消息编号: 记录.get("客户端消息编号"),
                消息正文: 记录.get("消息正文"),
                创建时间: 记录.get("创建时间"),
            })
            .collect(),
    ))
}
