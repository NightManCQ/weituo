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
    let 登录名: String = sqlx::query_scalar(
        r#"SELECT "登录名" FROM "用户"
           WHERE "用户编号" = $1 AND "账号状态" = '正常'"#,
    )
    .bind(用户.编号)
    .fetch_optional(&状态.数据库)
    .await?
    .ok_or_else(|| 应用错误::未授权("账号不存在或已停用".into()))?;
    if !是安全的_mqtt账号主题段(&登录名) {
        return Err(应用错误::请求错误(
            "登录名包含 MQTT 主题不支持的字符，无法建立实时连接".into(),
        ));
    }
    let 群组记录集 = sqlx::query(
        r#"SELECT g."群组编号"
           FROM "群_成员" m
           JOIN "群_群组" g ON g."群组编号" = m."群组编号"
           WHERE m."用户编号" = $1
             AND m."成员状态" = '有效'
             AND g."群组状态" = '正常'
           ORDER BY g."群组编号""#,
    )
    .bind(用户.编号)
    .fetch_all(&状态.数据库)
    .await?;
    let mut 主题列表: Vec<String> = 群组记录集
        .iter()
        .map(|记录| {
            let 群组编号: Uuid = 记录.get("群组编号");
            format!("q/{群组编号}")
        })
        .collect();
    主题列表.push(format!("u/{登录名}"));
    let 有效秒数: u64 = 60;
    let 过期时间 = (Utc::now() + ChronoDuration::seconds(60)).timestamp() as usize;
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
        有效秒数,
        说明: "EMQX 必须配置 MQTT_JWT_SECRET 对应的 JWT 认证器并启用 ACL claim；仅订阅当前用户有效群组及本人通知主题",
    }))
}
