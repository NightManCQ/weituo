use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use crate::共享::*;

const 群组人数上限: i64 = 100;

pub(super) async fn 创建委托群聊(
    事务: &mut Transaction<'_, Postgres>,
    任务编号: Uuid,
    发布者编号: Uuid,
    任务标题: &str,
) -> Result<Uuid, 应用错误> {
    let 群组编号 = Uuid::new_v4();
    let 群组名称: String = 任务标题.chars().take(80).collect();
    sqlx::query(
        r#"INSERT INTO "群_群组"
           ("群组编号", "群组名称", "群主编号", "关联任务编号")
           VALUES ($1, $2, $3, $4)"#,
    )
    .bind(群组编号)
    .bind(群组名称)
    .bind(发布者编号)
    .bind(任务编号)
    .execute(&mut **事务)
    .await?;
    sqlx::query(
        r#"INSERT INTO "群_成员"
           ("成员编号", "群组编号", "用户编号", "成员角色", "可见起始序号")
           VALUES ($1, $2, $3, '群主', 1)"#,
    )
    .bind(Uuid::new_v4())
    .bind(群组编号)
    .bind(发布者编号)
    .execute(&mut **事务)
    .await?;

    Ok(群组编号)
}

pub(super) async fn 加入委托群聊(
    事务: &mut Transaction<'_, Postgres>,
    任务编号: Uuid,
    接单者编号: Uuid,
) -> Result<(), 应用错误> {
    let 群组 = sqlx::query(
        r#"SELECT "群组编号", "最后消息序号"
           FROM "群_群组"
           WHERE "关联任务编号" = $1 AND "群组状态" = '正常'
           FOR UPDATE"#,
    )
    .bind(任务编号)
    .fetch_optional(&mut **事务)
    .await?
    .ok_or_else(|| 应用错误::冲突("委托关联群聊不存在或已归档".into()))?;
    let 群组编号: Uuid = 群组.get("群组编号");
    let 已是成员: bool = sqlx::query_scalar(
        r#"SELECT EXISTS (
               SELECT 1 FROM "群_成员"
               WHERE "群组编号" = $1 AND "用户编号" = $2 AND "成员状态" = '有效'
           )"#,
    )
    .bind(群组编号)
    .bind(接单者编号)
    .fetch_one(&mut **事务)
    .await?;
    if 已是成员 {
        return Ok(());
    }
    sqlx::query(
        r#"INSERT INTO "群_成员"
           ("成员编号", "群组编号", "用户编号", "成员角色", "可见起始序号")
           VALUES ($1, $2, $3, '成员', $4)"#,
    )
    .bind(Uuid::new_v4())
    .bind(群组编号)
    .bind(接单者编号)
    .bind(群组.get::<i64, _>("最后消息序号") + 1)
    .execute(&mut **事务)
    .await?;
    Ok(())
}

#[derive(Serialize)]
pub(super) struct 群组视图 {
    pub(super) 群组编号: Uuid,
    pub(super) 群组名称: String,
    pub(super) 群组公告: String,
    pub(super) 群主编号: Uuid,
    pub(super) 成员数: i64,
    pub(super) 最后消息序号: i64,
    pub(super) 最近消息: Option<String>,
    pub(super) 最近消息时间: Option<DateTime<Utc>>,
    pub(super) 已读至序号: i64,
}

#[derive(Serialize)]
pub(super) struct 群成员视图 {
    pub(super) 用户编号: Uuid,
    pub(super) 登录名: String,
    pub(super) 昵称: String,
    pub(super) 成员角色: String,
    pub(super) 加入时间: DateTime<Utc>,
}

#[derive(Serialize)]
pub(super) struct 群详情视图 {
    pub(super) 群组编号: Uuid,
    pub(super) 关联任务编号: Option<Uuid>,
    pub(super) 群组状态: String,
    pub(super) 群组名称: String,
    pub(super) 群组公告: String,
    pub(super) 群主编号: Uuid,
    pub(super) 最后消息序号: i64,
    pub(super) 成员列表: Vec<群成员视图>,
}

#[derive(Serialize)]
pub(super) struct 群邀请视图 {
    pub(super) 邀请编号: Uuid,
    pub(super) 群组编号: Uuid,
    pub(super) 群组名称: String,
    pub(super) 邀请人编号: Uuid,
    pub(super) 邀请人昵称: String,
    pub(super) 创建时间: DateTime<Utc>,
    pub(super) 过期时间: DateTime<Utc>,
}

#[derive(Serialize)]
pub(super) struct 群消息视图 {
    pub(super) 消息编号: Uuid,
    pub(super) 群组编号: Uuid,
    pub(super) 群内序号: i64,
    pub(super) 发送者编号: Uuid,
    pub(super) 发送者昵称: String,
    pub(super) 客户端消息编号: String,
    pub(super) 消息正文: String,
    pub(super) 创建时间: DateTime<Utc>,
}

#[derive(Deserialize)]
pub(super) struct 创建群组请求 {
    #[serde(alias = "name")]
    群组名称: String,
}

#[derive(Deserialize)]
pub(super) struct 邀请成员请求 {
    #[serde(alias = "user_id")]
    用户编号: Uuid,
}

#[derive(Deserialize)]
pub(super) struct 发送群消息请求 {
    #[serde(alias = "client_message_id")]
    客户端消息编号: String,
    #[serde(alias = "content")]
    消息正文: String,
}

#[derive(Deserialize)]
pub(super) struct 群消息查询条件 {
    before_sequence: Option<i64>,
    limit: Option<i64>,
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

pub(super) async fn 创建群组(
    State(状态): State<应用状态>,
    用户: Option<Extension<当前用户>>,
    Json(内容): Json<创建群组请求>,
) -> Result<Json<serde_json::Value>, 应用错误> {
    let 用户编号 = 用户
        .map(|Extension(用户)| 用户.编号)
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;
    let 群组名称 = 内容.群组名称.trim();
    if !(1..=80).contains(&群组名称.chars().count()) {
        return Err(应用错误::请求错误(
            "群组名称须为 1 至 80 个字符".into(),
        ));
    }

    let 群组编号 = Uuid::new_v4();
    let mut 事务 = 状态.数据库.begin().await?;
    sqlx::query(
        r#"INSERT INTO "群_群组" ("群组编号", "群组名称", "群主编号")
           VALUES ($1, $2, $3)"#,
    )
    .bind(群组编号)
    .bind(群组名称)
    .bind(用户编号)
    .execute(&mut *事务)
    .await?;
    sqlx::query(
        r#"INSERT INTO "群_成员"
           ("成员编号", "群组编号", "用户编号", "成员角色", "可见起始序号")
           VALUES ($1, $2, $3, '群主', 1)"#,
    )
    .bind(Uuid::new_v4())
    .bind(群组编号)
    .bind(用户编号)
    .execute(&mut *事务)
    .await?;
    事务.commit().await?;
    Ok(Json(serde_json::json!({"群组编号": 群组编号})))
}

pub(super) async fn 查询我的群组(
    State(状态): State<应用状态>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<群组视图>>, 应用错误> {
    let 用户编号 = 当前用户编号(&请求)?;
    let 记录集 = sqlx::query(
        r#"SELECT g."群组编号", g."群组名称", g."群组公告", g."群主编号",
                  g."最后消息序号", m."已读至序号",
                  (SELECT count(*) FROM "群_成员" active
                   WHERE active."群组编号" = g."群组编号" AND active."成员状态" = '有效')
                    AS "成员数",
                  latest."消息正文" AS "最近消息",
                  latest."创建时间" AS "最近消息时间"
           FROM "群_群组" g
           JOIN "群_成员" m ON m."群组编号" = g."群组编号"
                            AND m."用户编号" = $1 AND m."成员状态" = '有效'
           LEFT JOIN LATERAL (
               SELECT message."消息正文", message."创建时间", message."群内序号"
               FROM "群_消息" message
               WHERE message."群组编号" = g."群组编号"
                 AND message."群内序号" >= m."可见起始序号"
               ORDER BY message."群内序号" DESC
               LIMIT 1
           ) latest ON TRUE
           WHERE g."群组状态" = '正常'
             AND g."关联任务编号" IS NULL
           ORDER BY COALESCE(latest."群内序号", g."最后消息序号") DESC,
                    g."创建时间" DESC
           LIMIT 200"#,
    )
    .bind(用户编号)
    .fetch_all(&状态.数据库)
    .await?;
    Ok(Json(
        记录集
            .iter()
            .map(|记录| 群组视图 {
                群组编号: 记录.get("群组编号"),
                群组名称: 记录.get("群组名称"),
                群组公告: 记录.get("群组公告"),
                群主编号: 记录.get("群主编号"),
                成员数: 记录.get("成员数"),
                最后消息序号: 记录.get("最后消息序号"),
                最近消息: 记录.get("最近消息"),
                最近消息时间: 记录.get("最近消息时间"),
                已读至序号: 记录.get("已读至序号"),
            })
            .collect(),
    ))
}

pub(super) async fn 查询群邀请(
    State(状态): State<应用状态>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<群邀请视图>>, 应用错误> {
    let 用户编号 = 当前用户编号(&请求)?;
    let 记录集 = sqlx::query(
        r#"SELECT i."邀请编号", i."群组编号", g."群组名称",
                  i."邀请人编号", inviter."昵称" AS "邀请人昵称",
                  i."创建时间", i."过期时间"
           FROM "群_邀请" i
           JOIN "群_群组" g ON g."群组编号" = i."群组编号"
           JOIN "用户" inviter ON inviter."用户编号" = i."邀请人编号"
           WHERE i."受邀用户编号" = $1 AND i."邀请状态" = '待接受'
             AND i."过期时间" > now() AND g."群组状态" = '正常'
           ORDER BY i."创建时间" DESC
           LIMIT 100"#,
    )
    .bind(用户编号)
    .fetch_all(&状态.数据库)
    .await?;
    Ok(Json(
        记录集
            .iter()
            .map(|记录| 群邀请视图 {
                邀请编号: 记录.get("邀请编号"),
                群组编号: 记录.get("群组编号"),
                群组名称: 记录.get("群组名称"),
                邀请人编号: 记录.get("邀请人编号"),
                邀请人昵称: 记录.get("邀请人昵称"),
                创建时间: 记录.get("创建时间"),
                过期时间: 记录.get("过期时间"),
            })
            .collect(),
    ))
}

pub(super) async fn 查询群详情(
    State(状态): State<应用状态>,
    Path(群组编号): Path<Uuid>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<群详情视图>, 应用错误> {
    let 用户编号 = 当前用户编号(&请求)?;
    let 群组 = sqlx::query(
        r#"SELECT g."群组编号", g."关联任务编号", g."群组状态",
                  g."群组名称", g."群组公告", g."群主编号", g."最后消息序号"
           FROM "群_群组" g
           JOIN "群_成员" own ON own."群组编号" = g."群组编号"
                             AND own."用户编号" = $2 AND own."成员状态" = '有效'
           WHERE g."群组编号" = $1 AND g."群组状态" IN ('正常', '已归档')"#,
    )
    .bind(群组编号)
    .bind(用户编号)
    .fetch_optional(&状态.数据库)
    .await?
    .ok_or_else(|| 应用错误::禁止访问("群组不存在或你不是有效成员".into()))?;
    let 成员记录 = sqlx::query(
        r#"SELECT u."用户编号", u."登录名", u."昵称", m."成员角色", m."加入时间"
           FROM "群_成员" m
           JOIN "用户" u ON u."用户编号" = m."用户编号"
           WHERE m."群组编号" = $1 AND m."成员状态" = '有效'
           ORDER BY CASE m."成员角色" WHEN '群主' THEN 0 WHEN '管理员' THEN 1 ELSE 2 END,
                    m."加入时间", u."用户编号""#,
    )
    .bind(群组编号)
    .fetch_all(&状态.数据库)
    .await?;
    Ok(Json(群详情视图 {
        群组编号: 群组.get("群组编号"),
        关联任务编号: 群组.get("关联任务编号"),
        群组状态: 群组.get("群组状态"),
        群组名称: 群组.get("群组名称"),
        群组公告: 群组.get("群组公告"),
        群主编号: 群组.get("群主编号"),
        最后消息序号: 群组.get("最后消息序号"),
        成员列表: 成员记录
            .iter()
            .map(|记录| 群成员视图 {
                用户编号: 记录.get("用户编号"),
                登录名: 记录.get("登录名"),
                昵称: 记录.get("昵称"),
                成员角色: 记录.get("成员角色"),
                加入时间: 记录.get("加入时间"),
            })
            .collect(),
    }))
}

pub(super) async fn 查询委托群组(
    State(状态): State<应用状态>,
    Path(任务编号): Path<Uuid>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<serde_json::Value>, 应用错误> {
    let 用户编号 = 当前用户编号(&请求)?;
    let 群组编号: Uuid = sqlx::query_scalar(
        r#"SELECT g."群组编号"
           FROM "群_群组" g
           JOIN "群_成员" member ON member."群组编号" = g."群组编号"
                                AND member."用户编号" = $2
                                AND member."成员状态" = '有效'
           WHERE g."关联任务编号" = $1
             AND g."群组状态" IN ('正常', '已归档')"#,
    )
    .bind(任务编号)
    .bind(用户编号)
    .fetch_optional(&状态.数据库)
    .await?
    .ok_or_else(|| 应用错误::禁止访问("委托群聊不存在或你不是群成员".into()))?;
    Ok(Json(serde_json::json!({"群组编号": 群组编号})))
}

pub(super) async fn 邀请群成员(
    State(状态): State<应用状态>,
    Path(群组编号): Path<Uuid>,
    用户: Option<Extension<当前用户>>,
    Json(内容): Json<邀请成员请求>,
) -> Result<Json<serde_json::Value>, 应用错误> {
    let 邀请人编号 = 用户
        .map(|Extension(用户)| 用户.编号)
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;
    if 邀请人编号 == 内容.用户编号 {
        return Err(应用错误::请求错误("不能邀请自己加入群组".into()));
    }
    let mut 事务 = 状态.数据库.begin().await?;
    let 群组 = sqlx::query(
        r#"SELECT g."群组名称", g."最后消息序号", g."关联任务编号"
           FROM "群_群组" g
           JOIN "群_成员" m ON m."群组编号" = g."群组编号"
                            AND m."用户编号" = $2 AND m."成员状态" = '有效'
                            AND m."成员角色" IN ('群主', '管理员')
           WHERE g."群组编号" = $1 AND g."群组状态" = '正常'
           FOR UPDATE OF g"#,
    )
    .bind(群组编号)
    .bind(邀请人编号)
    .fetch_optional(&mut *事务)
    .await?
    .ok_or_else(|| 应用错误::禁止访问("只有群主或管理员可以邀请成员".into()))?;
    if 群组.get::<Option<Uuid>, _>("关联任务编号").is_some() {
        return Err(应用错误::禁止访问(
            "委托群成员由委托接单关系管理".into(),
        ));
    }
    let 目标用户登录名: String = sqlx::query_scalar(
        r#"SELECT "登录名" FROM "用户"
           WHERE "用户编号" = $1 AND "账号状态" = '正常'"#,
    )
    .bind(内容.用户编号)
    .fetch_optional(&mut *事务)
    .await?
    .ok_or_else(|| 应用错误::未找到("用户不存在或已停用".into()))?;
    if !是安全的_mqtt账号主题段(&目标用户登录名) {
        return Err(应用错误::请求错误(
            "目标用户登录名包含 MQTT 主题不支持的字符".into(),
        ));
    }
    let 已是好友 = sqlx::query(
        r#"SELECT 1 FROM "好友关系"
           WHERE "用户甲编号" = LEAST($1::uuid, $2::uuid)
             AND "用户乙编号" = GREATEST($1::uuid, $2::uuid)"#,
    )
    .bind(邀请人编号)
    .bind(内容.用户编号)
    .fetch_optional(&mut *事务)
    .await?;
    if 已是好友.is_none() {
        return Err(应用错误::禁止访问(
            "只能邀请自己的好友加入群组".into(),
        ));
    }
    let 成员情况 = sqlx::query(
        r#"SELECT count(*) FILTER (WHERE "成员状态" = '有效') AS "成员数",
                  bool_or("用户编号" = $2 AND "成员状态" = '有效') AS "已是成员"
           FROM "群_成员" WHERE "群组编号" = $1"#,
    )
    .bind(群组编号)
    .bind(内容.用户编号)
    .fetch_one(&mut *事务)
    .await?;
    if 成员情况.get::<bool, _>("已是成员") {
        return Err(应用错误::冲突("该用户已经是群成员".into()));
    }
    if 成员情况.get::<i64, _>("成员数") >= 群组人数上限 {
        return Err(应用错误::冲突("群组人数已达到上限".into()));
    }
    sqlx::query(
        r#"UPDATE "群_邀请"
           SET "邀请状态" = '已过期', "处理时间" = now()
           WHERE "群组编号" = $1 AND "受邀用户编号" = $2
             AND "邀请状态" = '待接受' AND "过期时间" <= now()"#,
    )
    .bind(群组编号)
    .bind(内容.用户编号)
    .execute(&mut *事务)
    .await?;
    let 待处理邀请 = sqlx::query(
        r#"SELECT "邀请编号" FROM "群_邀请"
           WHERE "群组编号" = $1 AND "受邀用户编号" = $2 AND "邀请状态" = '待接受'"#,
    )
    .bind(群组编号)
    .bind(内容.用户编号)
    .fetch_optional(&mut *事务)
    .await?;
    if 待处理邀请.is_some() {
        return Err(应用错误::冲突("该用户已有待处理的群邀请".into()));
    }

    let 邀请编号 = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO "群_邀请" ("邀请编号", "群组编号", "邀请人编号", "受邀用户编号")
           VALUES ($1, $2, $3, $4)"#,
    )
    .bind(邀请编号)
    .bind(群组编号)
    .bind(邀请人编号)
    .bind(内容.用户编号)
    .execute(&mut *事务)
    .await?;
    let 群组名称: String = 群组.get("群组名称");
    let 事件编号 = Uuid::new_v4();
    let 事件 = serde_json::json!({
        "事件编号": 事件编号,
        "事件类型": "group.invitation_created",
        "结构版本": 1,
        "发生时间": Utc::now(),
        "群组编号": 群组编号,
        "数据": {
            "邀请编号": 邀请编号,
            "群组编号": 群组编号,
            "群组名称": 群组名称,
            "邀请人编号": 邀请人编号
        }
    });
    sqlx::query(
        r#"INSERT INTO "事务发件箱" ("事件编号", "事件类型", "主题", "事件内容")
           VALUES ($1, 'group.invitation_created', $2, $3)"#,
    )
    .bind(事件编号)
    .bind(构造用户通知主题(内容.用户编号))
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
            tracing::error!(%事件编号, error = %错误, "immediate group invitation delivery failed");
        }
    });
    Ok(Json(serde_json::json!({"邀请编号": 邀请编号})))
}

pub(super) async fn 处理群邀请(
    State(状态): State<应用状态>,
    Path(邀请编号): Path<Uuid>,
    请求: axum::http::Request<axum::body::Body>,
    接受: bool,
) -> Result<Json<serde_json::Value>, 应用错误> {
    let 用户编号 = 当前用户编号(&请求)?;
    let mut 事务 = 状态.数据库.begin().await?;
    let 邀请 = sqlx::query(
        r#"SELECT "群组编号", "邀请状态", "过期时间"
           FROM "群_邀请"
           WHERE "邀请编号" = $1 AND "受邀用户编号" = $2
           FOR UPDATE"#,
    )
    .bind(邀请编号)
    .bind(用户编号)
    .fetch_optional(&mut *事务)
    .await?
    .ok_or_else(|| 应用错误::未找到("群邀请不存在".into()))?;
    if 邀请.get::<String, _>("邀请状态") != "待接受" {
        return Err(应用错误::冲突("该群邀请已处理".into()));
    }
    let 群组编号: Uuid = 邀请.get("群组编号");
    let 群组 = sqlx::query(
        r#"SELECT "最后消息序号" FROM "群_群组"
           WHERE "群组编号" = $1 AND "群组状态" = '正常'
           FOR UPDATE"#,
    )
    .bind(群组编号)
    .fetch_optional(&mut *事务)
    .await?
    .ok_or_else(|| 应用错误::未找到("群组不存在或已解散".into()))?;
    if 邀请.get::<DateTime<Utc>, _>("过期时间") <= Utc::now() {
        sqlx::query(
            r#"UPDATE "群_邀请" SET "邀请状态" = '已过期', "处理时间" = now()
               WHERE "邀请编号" = $1"#,
        )
        .bind(邀请编号)
        .execute(&mut *事务)
        .await?;
        事务.commit().await?;
        return Err(应用错误::冲突("群邀请已过期".into()));
    }

    if 接受 {
        let 成员数: i64 = sqlx::query_scalar(
            r#"SELECT count(*) FROM "群_成员"
               WHERE "群组编号" = $1 AND "成员状态" = '有效'"#,
        )
        .bind(群组编号)
        .fetch_one(&mut *事务)
        .await?;
        if 成员数 >= 群组人数上限 {
            return Err(应用错误::冲突("群组人数已达到上限".into()));
        }
        let 最新成员 = sqlx::query(
            r#"SELECT "成员编号" FROM "群_成员"
               WHERE "群组编号" = $1 AND "用户编号" = $2 AND "成员状态" = '有效'"#,
        )
        .bind(群组编号)
        .bind(用户编号)
        .fetch_optional(&mut *事务)
        .await?;
        if 最新成员.is_some() {
            return Err(应用错误::冲突("你已经是该群组成员".into()));
        }
        let 邀请人编号: Uuid = sqlx::query_scalar(
            r#"UPDATE "群_邀请"
               SET "邀请状态" = '已接受', "处理时间" = now()
               WHERE "邀请编号" = $1 RETURNING "邀请人编号""#,
        )
        .bind(邀请编号)
        .fetch_one(&mut *事务)
        .await?;
        sqlx::query(
            r#"INSERT INTO "群_成员"
               ("成员编号", "群组编号", "用户编号", "邀请人编号", "可见起始序号")
               VALUES ($1, $2, $3, $4, $5)"#,
        )
        .bind(Uuid::new_v4())
        .bind(群组编号)
        .bind(用户编号)
        .bind(邀请人编号)
        .bind(群组.get::<i64, _>("最后消息序号") + 1)
        .execute(&mut *事务)
        .await?;
    } else {
        sqlx::query(
            r#"UPDATE "群_邀请"
               SET "邀请状态" = '已拒绝', "处理时间" = now()
               WHERE "邀请编号" = $1"#,
        )
        .bind(邀请编号)
        .execute(&mut *事务)
        .await?;
    }
    事务.commit().await?;
    Ok(Json(
        serde_json::json!({"消息": if 接受 { "已加入群组" } else { "已拒绝群邀请" }}),
    ))
}

pub(super) async fn 接受群邀请(
    状态: State<应用状态>,
    邀请: Path<Uuid>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<serde_json::Value>, 应用错误> {
    处理群邀请(状态, 邀请, 请求, true).await
}

pub(super) async fn 拒绝群邀请(
    状态: State<应用状态>,
    邀请: Path<Uuid>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<serde_json::Value>, 应用错误> {
    处理群邀请(状态, 邀请, 请求, false).await
}

pub(super) async fn 退出群组(
    State(状态): State<应用状态>,
    Path(群组编号): Path<Uuid>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<serde_json::Value>, 应用错误> {
    let 用户编号 = 当前用户编号(&请求)?;
    let mut 事务 = 状态.数据库.begin().await?;
    let 群组 = sqlx::query(
        r#"SELECT "群主编号", "关联任务编号" FROM "群_群组"
           WHERE "群组编号" = $1 AND "群组状态" = '正常'
           FOR UPDATE"#,
    )
    .bind(群组编号)
    .fetch_optional(&mut *事务)
    .await?
    .ok_or_else(|| 应用错误::未找到("群组不存在或已解散".into()))?;
    let 群主编号: Uuid = 群组.get("群主编号");
    if 群组.get::<Option<Uuid>, _>("关联任务编号").is_some() {
        return Err(应用错误::禁止访问(
            "委托群成员由委托接单关系管理".into(),
        ));
    }
    if 群主编号 == 用户编号 {
        return Err(应用错误::禁止访问(
            "群主不能直接退出群组，需先转让群主身份".into(),
        ));
    }
    let 结果 = sqlx::query(
        r#"UPDATE "群_成员"
           SET "成员状态" = '主动退出', "离开时间" = now()
           WHERE "群组编号" = $1 AND "用户编号" = $2 AND "成员状态" = '有效'"#,
    )
    .bind(群组编号)
    .bind(用户编号)
    .execute(&mut *事务)
    .await?;
    if 结果.rows_affected() == 0 {
        return Err(应用错误::禁止访问("你不是该群组的有效成员".into()));
    }
    事务.commit().await?;
    Ok(Json(serde_json::json!({"消息": "已退出群组"})))
}

pub(super) async fn 移除群成员(
    State(状态): State<应用状态>,
    Path((群组编号, 目标用户编号)): Path<(Uuid, Uuid)>,
    用户: Option<Extension<当前用户>>,
) -> Result<Json<serde_json::Value>, 应用错误> {
    let 操作者编号 = 用户
        .map(|Extension(用户)| 用户.编号)
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;
    if 操作者编号 == 目标用户编号 {
        return Err(应用错误::请求错误("请使用退出群组接口".into()));
    }
    let mut 事务 = 状态.数据库.begin().await?;
    let 群组 = sqlx::query(
        r#"SELECT "关联任务编号" FROM "群_群组"
           WHERE "群组编号" = $1 AND "群组状态" = '正常'
           FOR UPDATE"#,
    )
    .bind(群组编号)
    .fetch_optional(&mut *事务)
    .await?
    .ok_or_else(|| 应用错误::未找到("群组不存在或已解散".into()))?;
    if 群组.get::<Option<Uuid>, _>("关联任务编号").is_some() {
        return Err(应用错误::禁止访问(
            "委托群成员由委托接单关系管理".into(),
        ));
    }
    let 权限 = sqlx::query(
        r#"SELECT own."成员角色" AS "操作者角色", target."成员角色" AS "目标角色"
           FROM "群_成员" own
           JOIN "群_成员" target ON target."群组编号" = own."群组编号"
           WHERE own."群组编号" = $1 AND own."用户编号" = $2
             AND own."成员状态" = '有效' AND target."用户编号" = $3
             AND target."成员状态" = '有效'
           FOR UPDATE OF own, target"#,
    )
    .bind(群组编号)
    .bind(操作者编号)
    .bind(目标用户编号)
    .fetch_optional(&mut *事务)
    .await?
    .ok_or_else(|| 应用错误::禁止访问("群成员不存在或无权操作".into()))?;
    let 操作者角色: String = 权限.get("操作者角色");
    let 目标角色: String = 权限.get("目标角色");
    if !matches!(操作者角色.as_str(), "群主" | "管理员")
        || 目标角色 == "群主"
        || (操作者角色 == "管理员" && 目标角色 == "管理员")
    {
        return Err(应用错误::禁止访问("无权移除该群成员".into()));
    }
    sqlx::query(
        r#"UPDATE "群_成员" SET "成员状态" = '被移除', "离开时间" = now()
           WHERE "群组编号" = $1 AND "用户编号" = $2 AND "成员状态" = '有效'"#,
    )
    .bind(群组编号)
    .bind(目标用户编号)
    .execute(&mut *事务)
    .await?;
    事务.commit().await?;
    Ok(Json(serde_json::json!({"消息": "已移除群成员"})))
}

pub(super) async fn 查询群消息(
    State(状态): State<应用状态>,
    Path(群组编号): Path<Uuid>,
    Query(条件): Query<群消息查询条件>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<群消息视图>>, 应用错误> {
    let 用户编号 = 当前用户编号(&请求)?;
    let before = 条件.before_sequence.unwrap_or(i64::MAX);
    if before < 1 {
        return Err(应用错误::请求错误("消息序号游标必须大于零".into()));
    }
    let limit = 条件.limit.unwrap_or(50).clamp(1, 100);
    let 记录集 = sqlx::query(
        r#"SELECT m."消息编号", m."群组编号", m."群内序号", m."发送者编号",
                  u."昵称" AS "发送者昵称", m."客户端消息编号",
                  m."消息正文", m."创建时间"
           FROM (
               SELECT message.*
               FROM "群_消息" message
               JOIN "群_成员" member ON member."群组编号" = message."群组编号"
                                    AND member."用户编号" = $2
                                    AND member."成员状态" = '有效'
               WHERE message."群组编号" = $1
                 AND message."群内序号" >= member."可见起始序号"
                 AND message."群内序号" < $3
               ORDER BY message."群内序号" DESC
               LIMIT $4
           ) m
           JOIN "用户" u ON u."用户编号" = m."发送者编号"
           ORDER BY m."群内序号""#,
    )
    .bind(群组编号)
    .bind(用户编号)
    .bind(before)
    .bind(limit)
    .fetch_all(&状态.数据库)
    .await?;
    Ok(Json(
        记录集
            .iter()
            .map(|记录| 群消息视图 {
                消息编号: 记录.get("消息编号"),
                群组编号: 记录.get("群组编号"),
                群内序号: 记录.get("群内序号"),
                发送者编号: 记录.get("发送者编号"),
                发送者昵称: 记录.get("发送者昵称"),
                客户端消息编号: 记录.get("客户端消息编号"),
                消息正文: 记录.get("消息正文"),
                创建时间: 记录.get("创建时间"),
            })
            .collect(),
    ))
}

pub(super) async fn 发送群消息(
    State(状态): State<应用状态>,
    Path(群组编号): Path<Uuid>,
    用户: Option<Extension<当前用户>>,
    Json(内容): Json<发送群消息请求>,
) -> Result<Json<群消息视图>, 应用错误> {
    let 用户编号 = 用户
        .map(|Extension(用户)| 用户.编号)
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;
    let 正文 = 内容.消息正文.trim();
    if !(1..=4000).contains(&正文.chars().count())
        || 内容.客户端消息编号.trim().is_empty()
        || 内容.客户端消息编号.len() > 100
    {
        return Err(应用错误::请求错误(
            "消息不能为空且最多 4000 个字符，客户端消息编号最多 100 字节".into(),
        ));
    }

    let mut 事务 = 状态.数据库.begin().await?;
    let 群组 = sqlx::query(
        r#"SELECT g."最后消息序号", u."昵称" AS "发送者昵称"
           FROM "群_群组" g
           JOIN "群_成员" m ON m."群组编号" = g."群组编号"
                            AND m."用户编号" = $2 AND m."成员状态" = '有效'
           JOIN "用户" u ON u."用户编号" = $2 AND u."账号状态" = '正常'
           WHERE g."群组编号" = $1 AND g."群组状态" = '正常'
           FOR UPDATE OF g"#,
    )
    .bind(群组编号)
    .bind(用户编号)
    .fetch_optional(&mut *事务)
    .await?
    .ok_or_else(|| 应用错误::禁止访问("群组不存在或你不是有效成员".into()))?;
    let 既有消息 = sqlx::query(
        r#"SELECT message."消息编号", message."群内序号", message."发送者编号",
                  sender."昵称" AS "发送者昵称", message."客户端消息编号",
                  message."消息正文", message."创建时间"
           FROM "群_消息" message
           JOIN "用户" sender ON sender."用户编号" = message."发送者编号"
           WHERE message."群组编号" = $1 AND message."发送者编号" = $2
             AND message."客户端消息编号" = $3"#,
    )
    .bind(群组编号)
    .bind(用户编号)
    .bind(内容.客户端消息编号.trim())
    .fetch_optional(&mut *事务)
    .await?;
    if let Some(记录) = 既有消息 {
        if 记录.get::<String, _>("消息正文") != 正文 {
            return Err(应用错误::冲突(
                "该客户端消息编号已用于其他消息内容".into(),
            ));
        }
        事务.commit().await?;
        return Ok(Json(群消息视图 {
            消息编号: 记录.get("消息编号"),
            群组编号,
            群内序号: 记录.get("群内序号"),
            发送者编号: 记录.get("发送者编号"),
            发送者昵称: 记录.get("发送者昵称"),
            客户端消息编号: 记录.get("客户端消息编号"),
            消息正文: 记录.get("消息正文"),
            创建时间: 记录.get("创建时间"),
        }));
    }

    let 群内序号: i64 = sqlx::query_scalar(
        r#"UPDATE "群_群组" SET "最后消息序号" = "最后消息序号" + 1,
                  "更新时间" = now()
           WHERE "群组编号" = $1 RETURNING "最后消息序号""#,
    )
    .bind(群组编号)
    .fetch_one(&mut *事务)
    .await?;
    let 消息编号 = Uuid::new_v4();
    let 消息记录 = sqlx::query(
        r#"INSERT INTO "群_消息"
           ("消息编号", "群组编号", "群内序号", "发送者编号",
            "客户端消息编号", "消息正文")
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING "创建时间""#,
    )
    .bind(消息编号)
    .bind(群组编号)
    .bind(群内序号)
    .bind(用户编号)
    .bind(内容.客户端消息编号.trim())
    .bind(正文)
    .fetch_one(&mut *事务)
    .await?;
    let 发送者昵称: String = 群组.get("发送者昵称");
    let 创建时间: DateTime<Utc> = 消息记录.get("创建时间");
    let 事件编号 = Uuid::new_v4();
    let 事件 = serde_json::json!({
        "事件编号": 事件编号,
        "事件类型": "group.message_created",
        "结构版本": 1,
        "发生时间": 创建时间,
        "群组编号": 群组编号,
        "数据": {
            "消息编号": 消息编号,
            "群组编号": 群组编号,
            "群内序号": 群内序号,
            "发送者编号": 用户编号,
            "发送者昵称": 发送者昵称,
            "客户端消息编号": 内容.客户端消息编号.trim(),
            "消息正文": 正文,
            "创建时间": 创建时间
        }
    });
    sqlx::query(
        r#"INSERT INTO "事务发件箱" ("事件编号", "事件类型", "主题", "事件内容")
           VALUES ($1, 'group.message_created', $2, $3)"#,
    )
    .bind(事件编号)
    .bind(format!("q/{群组编号}"))
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
            tracing::error!(%事件编号, error = %错误, "immediate group MQTT delivery failed");
        }
    });
    Ok(Json(群消息视图 {
        消息编号,
        群组编号,
        群内序号,
        发送者编号: 用户编号,
        发送者昵称,
        客户端消息编号: 内容.客户端消息编号.trim().to_owned(),
        消息正文: 正文.to_owned(),
        创建时间,
    }))
}

#[cfg(test)]
mod 数据库集成测试 {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    #[tokio::test]
    #[ignore = "需要 rust_houduan/.env 中配置 PostgreSQL"]
    async fn 委托群发布时创建群接单加入且释放后隔离历史消息() {
        dotenvy::dotenv().expect("读取 rust_houduan/.env 失败");
        let 数据库地址 = std::env::var("DATABASE_URL").expect("DATABASE_URL 未配置");
        let 数据库 = PgPoolOptions::new()
            .max_connections(2)
            .connect(&数据库地址)
            .await
            .expect("连接 PostgreSQL 失败");
        sqlx::migrate!("./migrations")
            .run(&数据库)
            .await
            .expect("运行数据库迁移失败");
        let 旧聊天表仍存在: bool =
            sqlx::query_scalar(r#"SELECT to_regclass('"聊天消息"') IS NOT NULL"#)
                .fetch_one(&数据库)
                .await
                .expect("检查旧聊天表失败");
        assert!(!旧聊天表仍存在, "迁移后不应保留旧任务聊天表");
        let 旧待投递事件仍存在: bool = sqlx::query_scalar(
            r#"SELECT EXISTS (
                   SELECT 1 FROM "事务发件箱"
                   WHERE "事件类型" = 'chat.message_created'
                     AND "主题" LIKE 'renwu/%/messages'
               )"#,
        )
        .fetch_one(&数据库)
        .await
        .expect("检查旧任务聊天 outbox 事件失败");
        assert!(!旧待投递事件仍存在, "迁移后不应保留旧任务聊天 outbox 事件");

        let 发布者编号 = Uuid::new_v4();
        let 首位接单者编号 = Uuid::new_v4();
        let 后续接单者编号 = Uuid::new_v4();
        let 任务编号 = Uuid::new_v4();
        let mut 事务 = 数据库.begin().await.expect("开始测试事务失败");

        for (用户编号, 后缀) in [
            (发布者编号, "owner"),
            (首位接单者编号, "worker1"),
            (后续接单者编号, "worker2"),
        ] {
            sqlx::query(
                r#"INSERT INTO "用户" ("用户编号", "登录名", "密码摘要", "昵称")
                   VALUES ($1, $2, 'test-only', $3)"#,
            )
            .bind(用户编号)
            .bind(format!("group-test-{}-{后缀}", Uuid::new_v4()))
            .bind(后缀)
            .execute(&mut *事务)
            .await
            .expect("创建测试用户失败");
        }
        sqlx::query(
            r#"INSERT INTO "任务"
               ("任务编号", "发布者编号", "标题", "描述", "任务状态")
               VALUES ($1, $2, '事务测试委托', '', '待接单')"#,
        )
        .bind(任务编号)
        .bind(发布者编号)
        .execute(&mut *事务)
        .await
        .expect("创建测试委托失败");

        let 委托群组编号 = 创建委托群聊(&mut 事务, 任务编号, 发布者编号, "事务测试委托")
            .await
            .expect("创建委托群失败");
        let 有效成员数: i64 = sqlx::query_scalar(
            r#"SELECT count(*) FROM "群_成员"
               WHERE "群组编号" = $1 AND "成员状态" = '有效'"#,
        )
        .bind(委托群组编号)
        .fetch_one(&mut *事务)
        .await
        .expect("查询委托群成员失败");
        assert_eq!(有效成员数, 1);

        sqlx::query(
            r#"INSERT INTO "群_消息"
               ("消息编号", "群组编号", "群内序号", "发送者编号",
                "客户端消息编号", "消息正文")
               VALUES ($1, $2, 1, $3, 'history-1', '上一任接单者的历史消息')"#,
        )
        .bind(Uuid::new_v4())
        .bind(委托群组编号)
        .bind(发布者编号)
        .execute(&mut *事务)
        .await
        .expect("创建历史测试消息失败");
        sqlx::query(r#"UPDATE "群_群组" SET "最后消息序号" = 1 WHERE "群组编号" = $1"#)
            .bind(委托群组编号)
            .execute(&mut *事务)
            .await
            .expect("更新测试群序号失败");
        sqlx::query(
            r#"UPDATE "任务" SET "任务状态" = '已接单', "接单者编号" = $2 WHERE "任务编号" = $1"#,
        )
        .bind(任务编号)
        .bind(首位接单者编号)
        .execute(&mut *事务)
        .await
        .expect("更新委托接单者失败");
        加入委托群聊(&mut 事务, 任务编号, 首位接单者编号)
            .await
            .expect("接单者加入委托群失败");
        let 首位成员可见起始序号: i64 = sqlx::query_scalar(
            r#"SELECT "可见起始序号" FROM "群_成员"
               WHERE "群组编号" = $1 AND "用户编号" = $2 AND "成员状态" = '有效'"#,
        )
        .bind(委托群组编号)
        .bind(首位接单者编号)
        .fetch_one(&mut *事务)
        .await
        .expect("查询首位接单者可见范围失败");
        assert_eq!(首位成员可见起始序号, 2);
        sqlx::query(
            r#"UPDATE "群_成员"
               SET "成员状态" = '主动退出', "离开时间" = now()
               WHERE "群组编号" = $1 AND "用户编号" = $2 AND "成员状态" = '有效'"#,
        )
        .bind(委托群组编号)
        .bind(首位接单者编号)
        .execute(&mut *事务)
        .await
        .expect("释放原接单者群成员失败");
        sqlx::query(r#"UPDATE "任务" SET "接单者编号" = $2 WHERE "任务编号" = $1"#)
            .bind(任务编号)
            .bind(后续接单者编号)
            .execute(&mut *事务)
            .await
            .expect("更新测试委托接单者失败");

        加入委托群聊(&mut 事务, 任务编号, 后续接单者编号)
            .await
            .expect("新接单者加入委托群失败");
        let 新成员可见起始序号: i64 = sqlx::query_scalar(
            r#"SELECT "可见起始序号" FROM "群_成员"
               WHERE "群组编号" = $1 AND "用户编号" = $2 AND "成员状态" = '有效'"#,
        )
        .bind(委托群组编号)
        .bind(后续接单者编号)
        .fetch_one(&mut *事务)
        .await
        .expect("查询新接单者可见范围失败");
        assert_eq!(新成员可见起始序号, 2);
        let 原接单者仍有效: bool = sqlx::query_scalar(
            r#"SELECT EXISTS (
                   SELECT 1 FROM "群_成员"
                   WHERE "群组编号" = $1 AND "用户编号" = $2 AND "成员状态" = '有效'
               )"#,
        )
        .bind(委托群组编号)
        .bind(首位接单者编号)
        .fetch_one(&mut *事务)
        .await
        .expect("查询原接单者群成员状态失败");
        assert!(!原接单者仍有效);

        sqlx::query(
            r#"UPDATE "群_群组"
               SET "群组状态" = '已归档', "归档时间" = now()
               WHERE "群组编号" = $1"#,
        )
        .bind(委托群组编号)
        .execute(&mut *事务)
        .await
        .expect("归档测试委托群失败");
        assert!(
            加入委托群聊(&mut 事务, 任务编号, 首位接单者编号)
                .await
                .is_err(),
            "归档群不应接受新成员"
        );

        事务.rollback().await.expect("回滚测试数据失败");
    }
}
