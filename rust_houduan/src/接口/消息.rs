use axum::{Json, body::Body, http::Request};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

use crate::共享::*;

#[derive(Serialize)]
pub(super) struct 消息会话视图 {
    pub(super) 会话类型: String,
    pub(super) 会话编号: Uuid,
    pub(super) 关联编号: Uuid,
    pub(super) 会话名称: String,
    pub(super) 联系人昵称: Option<String>,
    pub(super) 会话说明: String,
    pub(super) 最近消息: Option<String>,
    pub(super) 最近消息时间: DateTime<Utc>,
    pub(super) 会话状态: Option<String>,
    pub(super) 成员数: Option<i64>,
    pub(super) 邀请人编号: Option<Uuid>,
    pub(super) 过期时间: Option<DateTime<Utc>>,
    pub(super) MQTT主题: String,
    pub(super) 申请人登录名: Option<String>,
    pub(super) 申请附言: Option<String>,
}

pub(super) async fn 查询我的消息会话(
    状态: axum::extract::State<应用状态>,
    请求: Request<Body>,
) -> Result<Json<Vec<消息会话视图>>, 应用错误> {
    let 用户编号 = 请求
        .extensions()
        .get::<当前用户>()
        .map(|用户| 用户.编号)
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;

    let 记录集 = sqlx::query(
        r#"(
               SELECT '委托'::text AS "会话类型",
                      g."群组编号" AS "会话编号",
                      COALESCE(t."接单者编号", t."发布者编号") AS "关联编号",
                      t."标题" AS "会话名称",
                      联系人."昵称" AS "联系人昵称",
                      CASE WHEN t."接单者编号" IS NULL
                           THEN '等待接单'::text ELSE '委托沟通'::text
                      END AS "会话说明",
                      COALESCE(
                          latest."消息正文",
                          CASE WHEN t."接单者编号" IS NULL
                               THEN '委托已发布，等待接单'::text
                               ELSE '委托沟通已开始'::text
                          END
                      ) AS "最近消息",
                      COALESCE(latest."创建时间", g."创建时间")
                          AS "最近消息时间",
                      t."任务状态" AS "会话状态",
                      (SELECT count(*) FROM "群_成员" active
                       WHERE active."群组编号" = g."群组编号"
                         AND active."成员状态" = '有效') AS "成员数",
                      NULL::uuid AS "邀请人编号",
                      NULL::timestamptz AS "过期时间",
                      NULL::text AS "申请人登录名",
                      NULL::text AS "申请附言"
               FROM "任务" t
               JOIN "群_群组" g ON g."关联任务编号" = t."任务编号"
                                  AND g."群组状态" = '正常'
               JOIN "群_成员" own ON own."群组编号" = g."群组编号"
                                 AND own."用户编号" = $1
                                 AND own."成员状态" = '有效'
               LEFT JOIN "用户" 联系人 ON 联系人."用户编号" = CASE
                   WHEN t."发布者编号" = $1 THEN t."接单者编号"
                   ELSE t."发布者编号"
               END
               LEFT JOIN LATERAL (
                   SELECT message."消息正文", message."创建时间"
                   FROM "群_消息" message
                   WHERE message."群组编号" = g."群组编号"
                     AND message."群内序号" >= own."可见起始序号"
                   ORDER BY message."群内序号" DESC
                   LIMIT 1
               ) latest ON TRUE
               WHERE t."发布者编号" = $1 OR t."接单者编号" = $1
               ORDER BY COALESCE(latest."创建时间", g."创建时间") DESC,
                        t."任务编号" DESC
               LIMIT 200
           )
           UNION ALL
           (
               SELECT '群邀请'::text AS "会话类型",
                      i."邀请编号" AS "会话编号",
                      i."群组编号" AS "关联编号",
                      g."群组名称" AS "会话名称",
                      inviter."昵称" AS "联系人昵称",
                      '群聊邀请'::text AS "会话说明",
                      ('邀请你加入群聊：' || g."群组名称") AS "最近消息",
                      i."创建时间" AS "最近消息时间",
                      '待接受'::text AS "会话状态",
                      NULL::bigint AS "成员数",
                      i."邀请人编号" AS "邀请人编号",
                      i."过期时间" AS "过期时间",
                      NULL::text AS "申请人登录名",
                      NULL::text AS "申请附言"
               FROM "群_邀请" i
               JOIN "群_群组" g ON g."群组编号" = i."群组编号"
               JOIN "用户" inviter ON inviter."用户编号" = i."邀请人编号"
               WHERE i."受邀用户编号" = $1
                 AND i."邀请状态" = '待接受'
                 AND i."过期时间" > now()
                 AND g."群组状态" = '正常'
               ORDER BY i."创建时间" DESC
               LIMIT 100
           )
           UNION ALL
           (
               SELECT '好友验证'::text AS "会话类型",
                      a."申请编号" AS "会话编号",
                      a."申请人编号" AS "关联编号",
                      applicant."昵称" AS "会话名称",
                      applicant."昵称" AS "联系人昵称",
                      '好友验证'::text AS "会话说明",
                      a."申请附言" AS "最近消息",
                      a."创建时间" AS "最近消息时间",
                      '待接受'::text AS "会话状态",
                      NULL::bigint AS "成员数",
                      NULL::uuid AS "邀请人编号",
                      NULL::timestamptz AS "过期时间",
                      applicant."登录名" AS "申请人登录名",
                      a."申请附言" AS "申请附言"
               FROM "好友申请" a
               JOIN "用户" applicant ON applicant."用户编号" = a."申请人编号"
               WHERE a."目标用户编号" = $1
                 AND a."申请状态" = '待处理'
               ORDER BY a."创建时间" DESC
               LIMIT 100
           )
           UNION ALL
           (
               SELECT '好友'::text AS "会话类型",
                      s."会话编号",
                      contact."用户编号" AS "关联编号",
                      contact."昵称" AS "会话名称",
                      contact."昵称" AS "联系人昵称",
                      '好友私聊'::text AS "会话说明",
                      latest."消息正文" AS "最近消息",
                      COALESCE(latest."创建时间", s."创建时间") AS "最近消息时间",
                      NULL::text AS "会话状态",
                      NULL::bigint AS "成员数",
                      NULL::uuid AS "邀请人编号",
                      NULL::timestamptz AS "过期时间",
                      NULL::text AS "申请人登录名",
                      NULL::text AS "申请附言"
               FROM "私聊会话" s
               JOIN "好友关系" f
                 ON f."用户甲编号" = s."用户甲编号"
                AND f."用户乙编号" = s."用户乙编号"
               JOIN "用户" contact ON contact."用户编号" = CASE
                   WHEN s."用户甲编号" = $1 THEN s."用户乙编号"
                   ELSE s."用户甲编号"
               END
               LEFT JOIN LATERAL (
                   SELECT message."消息正文", message."创建时间"
                   FROM "私聊消息" message
                   WHERE message."会话编号" = s."会话编号"
                   ORDER BY message."创建时间" DESC, message."消息编号" DESC
                   LIMIT 1
               ) latest ON TRUE
               WHERE s."用户甲编号" = $1 OR s."用户乙编号" = $1
               ORDER BY COALESCE(latest."创建时间", s."创建时间") DESC,
                        s."会话编号" DESC
               LIMIT 200
           )
           UNION ALL
           (
               SELECT '群聊'::text AS "会话类型",
                      g."群组编号" AS "会话编号",
                      g."群组编号" AS "关联编号",
                      g."群组名称" AS "会话名称",
                      NULL::text AS "联系人昵称",
                      '群聊'::text AS "会话说明",
                      latest."消息正文" AS "最近消息",
                      COALESCE(latest."创建时间", g."创建时间") AS "最近消息时间",
                      NULL::text AS "会话状态",
                      (SELECT count(*) FROM "群_成员" active
                       WHERE active."群组编号" = g."群组编号"
                         AND active."成员状态" = '有效') AS "成员数",
                      NULL::uuid AS "邀请人编号",
                      NULL::timestamptz AS "过期时间",
                      NULL::text AS "申请人登录名",
                      NULL::text AS "申请附言"
               FROM "群_群组" g
               JOIN "群_成员" member
                 ON member."群组编号" = g."群组编号"
                AND member."用户编号" = $1
                AND member."成员状态" = '有效'
               LEFT JOIN LATERAL (
                   SELECT message."消息正文", message."创建时间"
                   FROM "群_消息" message
                   WHERE message."群组编号" = g."群组编号"
                     AND message."群内序号" >= member."可见起始序号"
                   ORDER BY message."群内序号" DESC
                   LIMIT 1
               ) latest ON TRUE
               WHERE g."群组状态" = '正常'
                 AND g."关联任务编号" IS NULL
               ORDER BY COALESCE(latest."创建时间", g."创建时间") DESC,
                        g."群组编号" DESC
               LIMIT 200
           )
           ORDER BY "最近消息时间" DESC, "会话编号" DESC
           LIMIT 500"#,
    )
    .bind(用户编号)
    .fetch_all(&状态.数据库)
    .await?;

    Ok(Json(
        记录集
            .iter()
            .map(|记录| {
                let 会话类型: String = 记录.get("会话类型");
                let 关联编号: Uuid = 记录.get("关联编号");
                let MQTT主题 = match 会话类型.as_str() {
                    "委托" | "群聊" => format!("q/{关联编号}"),
                    "好友" | "群邀请" => 构造用户通知主题(用户编号),
                    _ => 构造用户通知主题(用户编号),
                };
                消息会话视图 {
                    会话类型,
                    会话编号: 记录.get("会话编号"),
                    关联编号,
                    会话名称: 记录.get("会话名称"),
                    联系人昵称: 记录.get("联系人昵称"),
                    会话说明: 记录.get("会话说明"),
                    最近消息: 记录.get("最近消息"),
                    最近消息时间: 记录.get("最近消息时间"),
                    会话状态: 记录.get("会话状态"),
                    成员数: 记录.get("成员数"),
                    邀请人编号: 记录.get("邀请人编号"),
                    过期时间: 记录.get("过期时间"),
                    MQTT主题,
                    申请人登录名: 记录.get("申请人登录名"),
                    申请附言: 记录.get("申请附言"),
                }
            })
            .collect(),
    ))
}
