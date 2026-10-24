use axum::{
    Json,
    extract::{Path, State},
};
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use crate::共享::*;

async fn 设置用户状态(
    应用状态: 应用状态,
    请求: axum::http::Request<axum::body::Body>,
    目标用户编号: Uuid,
    账号状态: &'static str,
) -> Result<Json<Value>, 应用错误> {
    let 操作者 = 请求
        .extensions()
        .get::<当前用户>()
        .cloned()
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;
    if !操作者
        .角色列表
        .iter()
        .any(|role| role == "管理员" || role == "运营员")
    {
        return Err(应用错误::禁止访问("需要运营员或管理员权限".into()));
    }
    if 操作者.编号 == 目标用户编号 {
        return Err(应用错误::冲突("不能修改自己的账号状态".into()));
    }
    let mut 事务 = 应用状态.数据库.begin().await?;
    let 记录 = sqlx::query(
        r#"UPDATE "用户" SET "账号状态" = $2
           WHERE "用户编号" = $1
           RETURNING "用户编号", "账号状态""#,
    )
    .bind(目标用户编号)
    .bind(账号状态)
    .fetch_optional(&mut *事务)
    .await?
    .ok_or_else(|| 应用错误::未找到("用户不存在".into()))?;
    sqlx::query(
        r#"INSERT INTO "操作审计"
           ("审计编号", "操作者编号", "目标用户编号", "操作类型", "操作内容")
           VALUES ($1, $2, $3, $4, $5)"#,
    )
    .bind(Uuid::new_v4())
    .bind(操作者.编号)
    .bind(目标用户编号)
    .bind(if 账号状态 == "禁用" {
        "user.disabled"
    } else {
        "user.enabled"
    })
    .bind(json!({"account_status": 账号状态}))
    .execute(&mut *事务)
    .await?;
    事务.commit().await?;
    Ok(Json(json!({
        "用户编号": 记录.get::<Uuid, _>("用户编号"),
        "账号状态": 记录.get::<String, _>("账号状态")
    })))
}

pub(super) async fn 禁用用户(
    State(状态): State<应用状态>,
    Path(目标用户编号): Path<Uuid>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<Value>, 应用错误> {
    设置用户状态(状态, 请求, 目标用户编号, "禁用").await
}

pub(super) async fn 启用用户(
    State(状态): State<应用状态>,
    Path(目标用户编号): Path<Uuid>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<Value>, 应用错误> {
    设置用户状态(状态, 请求, 目标用户编号, "正常").await
}
