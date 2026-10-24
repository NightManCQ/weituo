use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use axum::{
    Json,
    extract::State,
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::{Duration as ChronoDuration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use sqlx::Row;
use tracing::error;
use uuid::Uuid;

use crate::共享::*;

pub(super) async fn 验证可选身份(
    State(状态): State<应用状态>,
    mut 请求: axum::http::Request<axum::body::Body>,
    后续处理: Next,
) -> Response {
    if let Some(值) = 请求.headers().get(header::AUTHORIZATION) {
        let 认证内容 = 值.to_str().unwrap_or_default();
        if let Some(令牌) = 认证内容.strip_prefix("Bearer ") {
            match decode::<令牌声明>(
                令牌,
                &DecodingKey::from_secret(状态.令牌密钥.as_bytes()),
                &Validation::default(),
            ) {
                Ok(令牌数据) => {
                    match sqlx::query(
                        r#"SELECT u."用户编号",
                                  COALESCE(array_agg(r."角色名称") FILTER (
                                      WHERE r."角色编号" IS NOT NULL
                                  ), ARRAY[]::text[]) AS "角色名称"
                           FROM "用户" u
                           LEFT JOIN "用户角色" ur ON ur."用户编号" = u."用户编号"
                           LEFT JOIN "角色" r ON r."角色编号" = ur."角色编号"
                           WHERE u."用户编号" = $1 AND u."账号状态" = '正常'
                           GROUP BY u."用户编号""#,
                    )
                    .bind(令牌数据.claims.主体)
                    .fetch_optional(&状态.数据库)
                    .await
                    {
                        Ok(Some(记录)) => {
                            请求.extensions_mut().insert(当前用户 {
                                编号: 记录.get("用户编号"),
                                角色列表: 记录.get("角色名称"),
                            });
                        }
                        Ok(None) => {
                            return (
                                StatusCode::UNAUTHORIZED,
                                Json(接口错误 {
                                    消息: "账号不存在或已停用".into(),
                                }),
                            )
                                .into_response();
                        }
                        Err(err) => {
                            error!(error = %err, "authentication lookup failed");
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(接口错误 {
                                    消息: "认证服务暂不可用".into(),
                                }),
                            )
                                .into_response();
                        }
                    }
                }
                Err(_) => {
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(接口错误 {
                            消息: "访问令牌无效或已过期".into(),
                        }),
                    )
                        .into_response();
                }
            }
        }
    }
    后续处理.run(请求).await
}

fn 签发访问令牌(用户编号: Uuid, 密钥: &str) -> Result<String, 应用错误> {
    let 令牌内容 = 令牌声明 {
        主体: 用户编号,
        过期时间: (Utc::now() + ChronoDuration::hours(2)).timestamp() as usize,
    };
    encode(
        &Header::default(),
        &令牌内容,
        &EncodingKey::from_secret(密钥.as_bytes()),
    )
    .map_err(|e| 应用错误::内部错误(e.into()))
}

pub(super) async fn 注册用户(
    State(状态): State<应用状态>,
    Json(事件内容): Json<注册请求>,
) -> Result<(StatusCode, Json<认证响应>), 应用错误> {
    let 登录用户 = 事件内容.登录名.trim();
    let 昵称 = 事件内容.昵称.trim();
    if !(3..=64).contains(&登录用户.chars().count())
        || 事件内容.密码.len() < 10
        || 事件内容.密码.len() > 128
        || !(1..=40).contains(&昵称.chars().count())
    {
        return Err(应用错误::请求错误(
            "登录名长度需为 3-64，密码需为 10-128 字节，昵称需为 1-40 个字符".into(),
        ));
    }
    let 盐值 = SaltString::generate(&mut OsRng);
    let 密码摘要 = Argon2::default()
        .hash_password(事件内容.密码.as_bytes(), &盐值)
        .map_err(|e| 应用错误::内部错误(anyhow::anyhow!(e.to_string())))?
        .to_string();
    let 编号 = Uuid::new_v4();
    let mut 事务 = 状态.数据库.begin().await?;
    let 记录 = sqlx::query(
        r#"INSERT INTO "用户" ("用户编号", "登录名", "密码摘要", "昵称")
           VALUES ($1, $2, $3, $4)
           RETURNING "用户编号", "登录名", "昵称""#,
    )
    .bind(编号)
    .bind(登录用户)
    .bind(密码摘要)
    .bind(昵称)
    .fetch_one(&mut *事务)
    .await
    .map_err(|err| {
        if err
            .as_database_error()
            .is_some_and(|数据库| 数据库.code().as_deref() == Some("23505"))
        {
            应用错误::冲突("登录名已被使用".into())
        } else {
            应用错误::内部错误(err.into())
        }
    })?;
    sqlx::query(
        r#"INSERT INTO "用户角色" ("用户编号", "角色编号")
           SELECT $1, "角色编号" FROM "角色" WHERE "角色名称" = '普通用户'"#,
    )
    .bind(编号)
    .execute(&mut *事务)
    .await?;
    crate::接口::积分::确保用户积分账户(&mut 事务, 编号).await?;
    事务.commit().await?;
    let 用户 = 用户视图 {
        用户编号: 记录.get("用户编号"),
        登录名: 记录.get("登录名"),
        昵称: 记录.get("昵称"),
    };
    let 令牌 = 签发访问令牌(用户.用户编号, &状态.令牌密钥)?;
    Ok((
        StatusCode::CREATED,
        Json(认证响应 {
            访问令牌: 令牌,
            令牌类型: "Bearer",
            有效秒数: 7200,
            用户,
        }),
    ))
}

pub(super) async fn 登录用户(
    State(状态): State<应用状态>,
    Json(事件内容): Json<登录请求>,
) -> Result<Json<认证响应>, 应用错误> {
    let 记录 = sqlx::query(
        r#"SELECT "用户编号", "登录名", "密码摘要", "昵称"
           FROM "用户" WHERE "登录名" = $1 AND "账号状态" = '正常'"#,
    )
    .bind(事件内容.登录名.trim())
    .fetch_optional(&状态.数据库)
    .await?
    .ok_or_else(|| 应用错误::未授权("登录名或密码错误".into()))?;
    let 摘要: String = 记录.get("密码摘要");
    let 已解析摘要 = PasswordHash::new(&摘要)
        .map_err(|e| 应用错误::内部错误(anyhow::anyhow!(e.to_string())))?;
    if Argon2::default()
        .verify_password(事件内容.密码.as_bytes(), &已解析摘要)
        .is_err()
    {
        return Err(应用错误::未授权("登录名或密码错误".into()));
    }
    let 用户 = 用户视图 {
        用户编号: 记录.get("用户编号"),
        登录名: 记录.get("登录名"),
        昵称: 记录.get("昵称"),
    };
    let 令牌 = 签发访问令牌(用户.用户编号, &状态.令牌密钥)?;
    Ok(Json(认证响应 {
        访问令牌: 令牌,
        令牌类型: "Bearer",
        有效秒数: 7200,
        用户,
    }))
}
