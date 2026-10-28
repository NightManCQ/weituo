use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::共享::*;

#[derive(Serialize)]
pub(super) struct 好友视图 {
    pub(super) 用户编号: Uuid,
    pub(super) 登录名: String,
    pub(super) 昵称: String,
    pub(super) 建立时间: DateTime<Utc>,
}

#[derive(Serialize)]
pub(super) struct 好友申请视图 {
    pub(super) 申请编号: Uuid,
    pub(super) 申请人编号: Uuid,
    pub(super) 登录名: String,
    pub(super) 昵称: String,
    pub(super) 申请附言: String,
    pub(super) 创建时间: DateTime<Utc>,
}

#[derive(Serialize)]
pub(super) struct 用户搜索视图 {
    pub(super) 用户编号: Uuid,
    pub(super) 登录名: String,
    pub(super) 昵称: String,
    pub(super) 好友状态: String,
}

#[derive(Serialize)]
pub(super) struct 私聊消息视图 {
    pub(super) 消息编号: Uuid,
    pub(super) 会话编号: Uuid,
    pub(super) 发送者编号: Uuid,
    pub(super) 发送者昵称: String,
    pub(super) 客户端消息编号: String,
    pub(super) 消息正文: String,
    pub(super) 创建时间: DateTime<Utc>,
}

#[derive(Serialize)]
pub(super) struct 好友聊天会话视图 {
    pub(super) 会话编号: Uuid,
    pub(super) 联系人编号: Uuid,
    pub(super) 联系人昵称: String,
    pub(super) 最近消息: Option<String>,
    pub(super) 最近消息时间: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub(super) struct 用户搜索条件 {
    #[serde(rename = "login_name")]
    登录名: String,
}

#[derive(Deserialize)]
pub(super) struct 好友申请请求 {
    #[serde(rename = "target_user_id")]
    目标用户编号: Uuid,
    #[serde(rename = "message", default)]
    申请附言: String,
}

#[derive(Deserialize)]
pub(super) struct 私聊消息请求 {
    #[serde(alias = "client_message_id")]
    客户端消息编号: String,
    #[serde(alias = "content")]
    消息正文: String,
}

#[derive(Deserialize)]
pub(super) struct 消息查询条件 {
    #[serde(rename = "limit")]
    数量上限: Option<i64>,
}

fn 当前用户编号(
    请求: &axum::http::Request<axum::body::Body>
) -> Result<Uuid, 应用错误> {
    请求
        .extensions()
        .get::<当前用户>()
        .map(|用户| 用户.编号)
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))
}

fn 好友编号对(用户甲: Uuid, 用户乙: Uuid) -> (Uuid, Uuid) {
    if 用户甲 < 用户乙 {
        (用户甲, 用户乙)
    } else {
        (用户乙, 用户甲)
    }
}

pub(super) async fn 查询我的好友(
    State(状态): State<应用状态>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<好友视图>>, 应用错误> {
    let 当前编号 = 当前用户编号(&请求)?;
    let 记录集 = sqlx::query(
        r#"SELECT u."用户编号", u."登录名", u."昵称", f."建立时间"
           FROM "好友关系" f
           JOIN "用户" u ON u."用户编号" = CASE
               WHEN f."用户甲编号" = $1 THEN f."用户乙编号"
               ELSE f."用户甲编号"
           END
           WHERE f."用户甲编号" = $1 OR f."用户乙编号" = $1
           ORDER BY u."昵称", u."用户编号""#,
    )
    .bind(当前编号)
    .fetch_all(&状态.数据库)
    .await?;
    Ok(Json(
        记录集
            .iter()
            .map(|记录| 好友视图 {
                用户编号: 记录.get("用户编号"),
                登录名: 记录.get("登录名"),
                昵称: 记录.get("昵称"),
                建立时间: 记录.get("建立时间"),
            })
            .collect(),
    ))
}

pub(super) async fn 查询好友聊天会话(
    State(状态): State<应用状态>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<好友聊天会话视图>>, 应用错误> {
    let 当前编号 = 当前用户编号(&请求)?;
    let 记录集 = sqlx::query(
        r#"SELECT s."会话编号",
                  contact."用户编号" AS "联系人编号",
                  contact."昵称" AS "联系人昵称",
                  latest."消息正文" AS "最近消息",
                  latest."创建时间" AS "最近消息时间"
           FROM "私聊会话" s
           JOIN "好友关系" f
             ON f."用户甲编号" = s."用户甲编号"
            AND f."用户乙编号" = s."用户乙编号"
           JOIN "用户" contact
             ON contact."用户编号" = CASE
                 WHEN s."用户甲编号" = $1 THEN s."用户乙编号"
                 ELSE s."用户甲编号"
             END
           LEFT JOIN LATERAL (
               SELECT "消息正文", "创建时间"
               FROM "私聊消息"
               WHERE "会话编号" = s."会话编号"
               ORDER BY "创建时间" DESC, "消息编号" DESC
               LIMIT 1
           ) latest ON TRUE
           WHERE s."用户甲编号" = $1 OR s."用户乙编号" = $1
           ORDER BY COALESCE(latest."创建时间", s."创建时间") DESC,
                    s."会话编号" DESC
           LIMIT 200"#,
    )
    .bind(当前编号)
    .fetch_all(&状态.数据库)
    .await?;
    Ok(Json(
        记录集
            .iter()
            .map(|记录| 好友聊天会话视图 {
                会话编号: 记录.get("会话编号"),
                联系人编号: 记录.get("联系人编号"),
                联系人昵称: 记录.get("联系人昵称"),
                最近消息: 记录.get("最近消息"),
                最近消息时间: 记录.get("最近消息时间"),
            })
            .collect(),
    ))
}

pub(super) async fn 搜索用户(
    State(状态): State<应用状态>,
    Query(条件): Query<用户搜索条件>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<用户搜索视图>>, 应用错误> {
    let 当前编号 = 当前用户编号(&请求)?;
    let 登录名 = 条件.登录名.trim();
    if !(3..=64).contains(&登录名.chars().count()) {
        return Err(应用错误::请求错误(
            "请输入完整登录名（3-64 个字符）".into(),
        ));
    }
    let 记录集 = sqlx::query(
        r#"SELECT u."用户编号", u."登录名", u."昵称",
                  CASE
                    WHEN EXISTS (
                        SELECT 1 FROM "好友关系" f
                        WHERE (f."用户甲编号" = $1 AND f."用户乙编号" = u."用户编号")
                           OR (f."用户乙编号" = $1 AND f."用户甲编号" = u."用户编号")
                    ) THEN '已是好友'
                    WHEN EXISTS (
                        SELECT 1 FROM "好友申请" a
                        WHERE a."申请状态" = '待处理'
                          AND a."申请人编号" = u."用户编号"
                          AND a."目标用户编号" = $1
                    ) THEN '待你处理'
                    WHEN EXISTS (
                        SELECT 1 FROM "好友申请" a
                        WHERE a."申请状态" = '待处理'
                          AND a."申请人编号" = $1
                          AND a."目标用户编号" = u."用户编号"
                    ) THEN '已发送申请'
                    ELSE '可添加'
                  END AS "好友状态"
           FROM "用户" u
           WHERE lower(u."登录名") = lower($2)
             AND u."账号状态" = '正常'
             AND u."用户编号" <> $1
           LIMIT 1"#,
    )
    .bind(当前编号)
    .bind(登录名)
    .fetch_all(&状态.数据库)
    .await?;
    Ok(Json(
        记录集
            .iter()
            .map(|记录| 用户搜索视图 {
                用户编号: 记录.get("用户编号"),
                登录名: 记录.get("登录名"),
                昵称: 记录.get("昵称"),
                好友状态: 记录.get("好友状态"),
            })
            .collect(),
    ))
}

pub(super) async fn 查询收到的好友申请(
    State(状态): State<应用状态>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<好友申请视图>>, 应用错误> {
    let 当前编号 = 当前用户编号(&请求)?;
    let 记录集 = sqlx::query(
        r#"SELECT a."申请编号", a."申请人编号", u."登录名", u."昵称",
                  a."申请附言", a."创建时间"
           FROM "好友申请" a
           JOIN "用户" u ON u."用户编号" = a."申请人编号"
           WHERE a."目标用户编号" = $1 AND a."申请状态" = '待处理'
           ORDER BY a."创建时间" DESC
           LIMIT 100"#,
    )
    .bind(当前编号)
    .fetch_all(&状态.数据库)
    .await?;
    Ok(Json(
        记录集
            .iter()
            .map(|记录| 好友申请视图 {
                申请编号: 记录.get("申请编号"),
                申请人编号: 记录.get("申请人编号"),
                登录名: 记录.get("登录名"),
                昵称: 记录.get("昵称"),
                申请附言: 记录.get("申请附言"),
                创建时间: 记录.get("创建时间"),
            })
            .collect(),
    ))
}

pub(super) async fn 创建好友申请(
    State(状态): State<应用状态>,
    用户: Option<Extension<当前用户>>,
    Json(申请): Json<好友申请请求>,
) -> Result<Json<serde_json::Value>, 应用错误> {
    let 当前编号 = 用户
        .map(|Extension(用户)| 用户.编号)
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;
    let 附言 = 申请.申请附言.trim();
    if 当前编号 == 申请.目标用户编号 {
        return Err(应用错误::请求错误("不能添加自己为好友".into()));
    }
    if 附言.chars().count() > 200 {
        return Err(应用错误::请求错误("申请附言最多 200 个字符".into()));
    }
    let 目标用户登录名: Option<String> = sqlx::query_scalar(
        r#"SELECT "登录名" FROM "用户"
           WHERE "用户编号" = $1 AND "账号状态" = '正常'"#,
    )
    .bind(申请.目标用户编号)
    .fetch_optional(&状态.数据库)
    .await?;
    let 目标用户登录名 =
        目标用户登录名.ok_or_else(|| 应用错误::未找到("用户不存在或已停用".into()))?;
    if !是安全的_mqtt账号主题段(&目标用户登录名) {
        return Err(应用错误::请求错误(
            "目标用户登录名包含 MQTT 主题不支持的字符".into(),
        ));
    }
    let (用户甲, 用户乙) = 好友编号对(当前编号, 申请.目标用户编号);
    let 已是好友 = sqlx::query(
        r#"SELECT 1 FROM "好友关系"
           WHERE "用户甲编号" = $1 AND "用户乙编号" = $2"#,
    )
    .bind(用户甲)
    .bind(用户乙)
    .fetch_optional(&状态.数据库)
    .await?;
    if 已是好友.is_some() {
        return Err(应用错误::冲突("你们已经是好友".into()));
    }
    let 对方申请 = sqlx::query(
        r#"SELECT 1 FROM "好友申请"
           WHERE "申请人编号" = $1 AND "目标用户编号" = $2
             AND "申请状态" = '待处理'"#,
    )
    .bind(申请.目标用户编号)
    .bind(当前编号)
    .fetch_optional(&状态.数据库)
    .await?;
    if 对方申请.is_some() {
        return Err(应用错误::冲突(
            "对方已向你发送好友申请，请在申请列表中处理".into(),
        ));
    }

    let 申请编号 = Uuid::new_v4();
    let mut 事务 = 状态.数据库.begin().await?;
    let 插入结果 = sqlx::query(
        r#"INSERT INTO "好友申请"
           ("申请编号", "申请人编号", "目标用户编号", "申请附言")
           VALUES ($1, $2, $3, $4)"#,
    )
    .bind(申请编号)
    .bind(当前编号)
    .bind(申请.目标用户编号)
    .bind(附言)
    .execute(&mut *事务)
    .await;
    if let Err(错误) = 插入结果 {
        if 错误
            .as_database_error()
            .is_some_and(|数据库| 数据库.code().as_deref() == Some("23505"))
        {
            return Err(应用错误::冲突("已有待处理的好友申请".into()));
        }
        return Err(错误.into());
    }

    let 事件编号 = Uuid::new_v4();
    let 事件 = serde_json::json!({
        "事件编号": 事件编号,
        "事件类型": "friend.request_created",
        "结构版本": 1,
        "发生时间": Utc::now(),
        "通知用户编号": 申请.目标用户编号,
        "数据": {
            "申请编号": 申请编号,
            "申请人编号": 当前编号,
            "目标用户编号": 申请.目标用户编号,
            "申请附言": 附言
        }
    });
    sqlx::query(
        r#"INSERT INTO "事务发件箱" ("事件编号", "事件类型", "主题", "事件内容")
           VALUES ($1, 'friend.request_created', $2, $3)"#,
    )
    .bind(事件编号)
    .bind(构造用户通知主题(申请.目标用户编号))
    .bind(事件)
    .execute(&mut *事务)
    .await?;
    事务.commit().await?;

    let 数据库 = 状态.数据库.clone();
    let 消息代理 = 状态.消息代理.clone();
    tokio::spawn(async move {
        if let Err(错误) =
            crate::工作者::发布单个发件箱事件(&数据库, &消息代理, 事件编号).await
        {
            tracing::error!(%事件编号, error = %错误, "immediate friend request MQTT delivery failed");
        }
    });

    Ok(Json(serde_json::json!({"消息": "好友申请已发送"})))
}

pub(super) async fn 同意好友申请(
    State(状态): State<应用状态>,
    Path(申请编号): Path<Uuid>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<serde_json::Value>, 应用错误> {
    处理好友申请(&状态.数据库, 当前用户编号(&请求)?, 申请编号, true).await?;
    Ok(Json(serde_json::json!({"消息": "已添加好友"})))
}

pub(super) async fn 拒绝好友申请(
    State(状态): State<应用状态>,
    Path(申请编号): Path<Uuid>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<serde_json::Value>, 应用错误> {
    处理好友申请(&状态.数据库, 当前用户编号(&请求)?, 申请编号, false).await?;
    Ok(Json(serde_json::json!({"消息": "已拒绝好友申请"})))
}

async fn 处理好友申请(
    数据库: &PgPool,
    当前编号: Uuid,
    申请编号: Uuid,
    同意: bool,
) -> Result<(), 应用错误> {
    let mut 事务 = 数据库.begin().await?;
    let 申请记录 = sqlx::query(
        r#"SELECT "申请人编号", "目标用户编号", "申请状态"
           FROM "好友申请"
           WHERE "申请编号" = $1 AND "目标用户编号" = $2
           FOR UPDATE"#,
    )
    .bind(申请编号)
    .bind(当前编号)
    .fetch_optional(&mut *事务)
    .await?
    .ok_or_else(|| 应用错误::未找到("好友申请不存在".into()))?;
    let 申请人编号: Uuid = 申请记录.get("申请人编号");
    let 状态: String = 申请记录.get("申请状态");
    if 状态 != "待处理" {
        return Err(应用错误::冲突("该好友申请已处理".into()));
    }

    if 同意 {
        let 用户对 = 好友编号对(当前编号, 申请人编号);
        sqlx::query(
            r#"INSERT INTO "好友关系" ("用户甲编号", "用户乙编号")
               VALUES ($1, $2) ON CONFLICT DO NOTHING"#,
        )
        .bind(用户对.0)
        .bind(用户对.1)
        .execute(&mut *事务)
        .await?;
        sqlx::query(
            r#"INSERT INTO "私聊会话" ("会话编号", "用户甲编号", "用户乙编号")
               VALUES ($1, $2, $3)
               ON CONFLICT ("用户甲编号", "用户乙编号") DO NOTHING"#,
        )
        .bind(Uuid::new_v4())
        .bind(用户对.0)
        .bind(用户对.1)
        .execute(&mut *事务)
        .await?;
    }
    sqlx::query(
        r#"UPDATE "好友申请"
           SET "申请状态" = $2, "处理时间" = now()
           WHERE "申请编号" = $1"#,
    )
    .bind(申请编号)
    .bind(if 同意 { "已同意" } else { "已拒绝" })
    .execute(&mut *事务)
    .await?;
    事务.commit().await?;
    Ok(())
}

pub(super) async fn 删除好友(
    State(状态): State<应用状态>,
    Path(好友编号): Path<Uuid>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<serde_json::Value>, 应用错误> {
    let 当前编号 = 当前用户编号(&请求)?;
    if 当前编号 == 好友编号 {
        return Err(应用错误::请求错误("好友编号无效".into()));
    }
    let 用户对 = 好友编号对(当前编号, 好友编号);
    let 结果 = sqlx::query(
        r#"DELETE FROM "好友关系"
           WHERE "用户甲编号" = $1 AND "用户乙编号" = $2"#,
    )
    .bind(用户对.0)
    .bind(用户对.1)
    .execute(&状态.数据库)
    .await?;
    if 结果.rows_affected() == 0 {
        return Err(应用错误::未找到("好友关系不存在".into()));
    }
    Ok(Json(serde_json::json!({"消息": "已删除好友"})))
}

pub(super) async fn 查询私聊消息(
    State(状态): State<应用状态>,
    Path(好友编号): Path<Uuid>,
    Query(条件): Query<消息查询条件>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<私聊消息视图>>, 应用错误> {
    let 当前编号 = 当前用户编号(&请求)?;
    if 当前编号 == 好友编号 {
        return Err(应用错误::请求错误("好友编号无效".into()));
    }
    let (用户甲, 用户乙) = 好友编号对(当前编号, 好友编号);
    let 会话 = sqlx::query(
        r#"SELECT "会话编号" FROM "私聊会话"
           WHERE "用户甲编号" = $1 AND "用户乙编号" = $2"#,
    )
    .bind(用户甲)
    .bind(用户乙)
    .fetch_optional(&状态.数据库)
    .await?;

    let 会话编号: Uuid = if let Some(会话) = 会话 {
        会话.get("会话编号")
    } else {
        let 是好友 = sqlx::query(
            r#"SELECT 1 FROM "好友关系"
               WHERE "用户甲编号" = $1 AND "用户乙编号" = $2"#,
        )
        .bind(用户甲)
        .bind(用户乙)
        .fetch_optional(&状态.数据库)
        .await?;
        if 是好友.is_none() {
            return Err(应用错误::禁止访问("你们还不是好友".into()));
        }
        sqlx::query(
            r#"INSERT INTO "私聊会话" ("会话编号", "用户甲编号", "用户乙编号")
               VALUES ($1, $2, $3)
               ON CONFLICT ("用户甲编号", "用户乙编号")
               DO UPDATE SET "用户甲编号" = EXCLUDED."用户甲编号"
               RETURNING "会话编号""#,
        )
        .bind(Uuid::new_v4())
        .bind(用户甲)
        .bind(用户乙)
        .fetch_one(&状态.数据库)
        .await?
        .get("会话编号")
    };
    let 限制 = 条件.数量上限.unwrap_or(100).clamp(1, 200);
    let 记录集 = sqlx::query(
        r#"SELECT m."消息编号", m."会话编号", m."发送者编号", u."昵称",
                  m."客户端消息编号", m."消息正文", m."创建时间"
           FROM (
               SELECT * FROM "私聊消息"
               WHERE "会话编号" = $1
               ORDER BY "创建时间" DESC, "消息编号" DESC
               LIMIT $2
           ) m
           JOIN "用户" u ON u."用户编号" = m."发送者编号"
           ORDER BY m."创建时间", m."消息编号""#,
    )
    .bind(会话编号)
    .bind(限制)
    .fetch_all(&状态.数据库)
    .await?;
    Ok(Json(
        记录集
            .iter()
            .map(|记录| 私聊消息视图 {
                消息编号: 记录.get("消息编号"),
                会话编号: 记录.get("会话编号"),
                发送者编号: 记录.get("发送者编号"),
                发送者昵称: 记录.get("昵称"),
                客户端消息编号: 记录.get("客户端消息编号"),
                消息正文: 记录.get("消息正文"),
                创建时间: 记录.get("创建时间"),
            })
            .collect(),
    ))
}

pub(super) async fn 发送私聊消息(
    State(状态): State<应用状态>,
    Path(好友编号): Path<Uuid>,
    用户: Option<Extension<当前用户>>,
    Json(内容): Json<私聊消息请求>,
) -> Result<Json<私聊消息视图>, 应用错误> {
    let 当前编号 = 用户
        .map(|Extension(用户)| 用户.编号)
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;

    let 正文 = 内容.消息正文.trim();
    if 当前编号 == 好友编号
        || !(1..=4000).contains(&正文.chars().count())
        || 内容.客户端消息编号.trim().is_empty()
        || 内容.客户端消息编号.len() > 100
    {
        return Err(应用错误::请求错误(
            "消息内容不能为空且最多 4000 个字符，客户端消息编号最多 100 字节".into(),
        ));
    }
    let (用户甲, 用户乙) = 好友编号对(当前编号, 好友编号);
    let mut 事务 = 状态.数据库.begin().await?;
    let 好友状态 = sqlx::query(
        r#"SELECT sender."昵称" AS "发送者昵称",
                  sender."登录名" AS "发送者登录名",
                  recipient."昵称" AS "好友昵称",
                  recipient."登录名" AS "好友登录名"
           FROM "好友关系" f
           JOIN "用户" sender ON sender."用户编号" = $1 AND sender."账号状态" = '正常'
           JOIN "用户" recipient ON recipient."用户编号" = $2 AND recipient."账号状态" = '正常'
           WHERE f."用户甲编号" = $3 AND f."用户乙编号" = $4
           FOR SHARE OF f"#,
    )
    .bind(当前编号)
    .bind(好友编号)
    .bind(用户甲)
    .bind(用户乙)
    .fetch_optional(&mut *事务)
    .await?
    .ok_or_else(|| 应用错误::禁止访问("只有正常账号之间的好友可以发送私聊".into()))?;
    let 发送者昵称: String = 好友状态.get("发送者昵称");
    let 发送者登录名: String = 好友状态.get("发送者登录名");
    let 好友昵称: String = 好友状态.get("好友昵称");
    let 好友登录名: String = 好友状态.get("好友登录名");
    if !是安全的_mqtt账号主题段(&发送者登录名) || !是安全的_mqtt账号主题段(&好友登录名)
    {
        return Err(应用错误::请求错误(
            "好友登录名包含 MQTT 主题不支持的字符".into(),
        ));
    }
    let mut 待发布事件编号 = Vec::with_capacity(2);
    let 会话编号: Uuid = sqlx::query(
        r#"INSERT INTO "私聊会话" ("会话编号", "用户甲编号", "用户乙编号")
           VALUES ($1, $2, $3)
           ON CONFLICT ("用户甲编号", "用户乙编号")
           DO UPDATE SET "用户甲编号" = EXCLUDED."用户甲编号"
           RETURNING "会话编号""#,
    )
    .bind(Uuid::new_v4())
    .bind(用户甲)
    .bind(用户乙)
    .fetch_one(&mut *事务)
    .await?
    .get("会话编号");

    let 新消息编号 = Uuid::new_v4();
    let 记录 = sqlx::query(
        r#"INSERT INTO "私聊消息"
           ("消息编号", "会话编号", "发送者编号", "客户端消息编号", "消息正文")
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT ("会话编号", "发送者编号", "客户端消息编号") DO UPDATE
             SET "客户端消息编号" = EXCLUDED."客户端消息编号"
           RETURNING "消息编号", "会话编号", "发送者编号", "客户端消息编号",
                     "消息正文", "创建时间""#,
    )
    .bind(新消息编号)
    .bind(会话编号)
    .bind(当前编号)
    .bind(内容.客户端消息编号.trim())
    .bind(正文)
    .fetch_one(&mut *事务)
    .await?;
    if 记录.get::<String, _>("消息正文") != 正文 {
        return Err(应用错误::冲突(
            "该客户端消息编号已用于其他消息内容".into(),
        ));
    }
    if 记录.get::<Uuid, _>("消息编号") == 新消息编号 {
        for 通知用户编号 in [当前编号, 好友编号] {
            let 事件编号 = Uuid::new_v4();
            let 联系人编号 = if 通知用户编号 == 当前编号 {
                好友编号
            } else {
                当前编号
            };
            let 联系人昵称 = if 通知用户编号 == 当前编号 {
                好友昵称.clone()
            } else {
                发送者昵称.clone()
            };
            let 通知账号 = if 通知用户编号 == 当前编号 {
                发送者登录名.clone()
            } else {
                好友登录名.clone()
            };
            let 事件 = serde_json::json!({
                "事件编号": 事件编号,
                "事件类型": "friend.message_created",
                "结构版本": 1,
                "发生时间": Utc::now(),
                "会话编号": 会话编号,
                "通知用户编号": 通知用户编号,
                "数据": {
                    "消息编号": 记录.get::<Uuid, _>("消息编号"),
                    "会话编号": 会话编号,
                    "联系人编号": 联系人编号,
                    "联系人昵称": 联系人昵称,
                    "发送者编号": 当前编号,
                    "发送者昵称": 发送者昵称,
                    "客户端消息编号": 记录.get::<String, _>("客户端消息编号"),
                    "消息正文": 记录.get::<String, _>("消息正文"),
                    "创建时间": 记录.get::<DateTime<Utc>, _>("创建时间")
                }
            });
            sqlx::query(
                r#"INSERT INTO "事务发件箱" ("事件编号", "事件类型", "主题", "事件内容")
                   VALUES ($1, 'friend.message_created', $2, $3)"#,
            )
            .bind(事件编号)
            .bind(format!("u/{通知账号}"))
            .bind(事件)
            .execute(&mut *事务)
            .await?;
            待发布事件编号.push(事件编号);
        }
    }
    事务.commit().await?;
    let 数据库 = 状态.数据库.clone();
    let 消息代理 = 状态.消息代理.clone();
    for 事件编号 in 待发布事件编号 {
        let 数据库 = 数据库.clone();
        let 消息代理 = 消息代理.clone();
        tokio::spawn(async move {
            if let Err(错误) =
                crate::工作者::发布单个发件箱事件(&数据库, &消息代理, 事件编号).await
            {
                tracing::error!(%事件编号, error = %错误, "immediate friend MQTT delivery failed");
            }
        });
    }
    Ok(Json(私聊消息视图 {
        消息编号: 记录.get("消息编号"),
        会话编号: 记录.get("会话编号"),
        发送者编号: 记录.get("发送者编号"),
        发送者昵称,
        客户端消息编号: 记录.get("客户端消息编号"),
        消息正文: 记录.get("消息正文"),
        创建时间: 记录.get("创建时间"),
    }))
}
