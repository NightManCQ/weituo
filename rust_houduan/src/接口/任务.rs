use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::Utc;
use serde_json::json;
use sqlx::Row;
use sqlx::postgres::types::PgPoint;
use uuid::Uuid;

use super::提取幂等键;
use crate::共享::*;
use crate::领域::{任务状态错误, 计算任务状态};

const 任务视图字段: &str = r#""任务编号", "发布者编号", "接单者编号", "标题", "描述",
    "任务状态", "截止时间", "创建时间", "版本号", "经纬", "国", "省", "市", "县", "镇", "村", "地址""#;

// Only fixed SQL templates and the constant column list are interpolated here.
fn 任务记录查询(模板: &str) -> sqlx::AssertSqlSafe<String> {
    sqlx::AssertSqlSafe(模板.replace("{任务视图字段}", 任务视图字段))
}

pub(super) async fn 发布任务(
    State(状态): State<应用状态>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<(StatusCode, Json<任务视图>), 应用错误> {
    let 用户 = 请求
        .extensions()
        .get::<当前用户>()
        .cloned()
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;
    let 幂等键 = 提取幂等键(请求.headers())?;
    let 事件内容: 任务内容请求 = serde_json::from_slice(
        &axum::body::to_bytes(请求.into_body(), 1_000_000)
            .await
            .map_err(|e| 应用错误::请求错误(e.to_string()))?,
    )
    .map_err(|e| 应用错误::请求错误(e.to_string()))?;
    为用户发布任务(&状态, 用户, 事件内容, 幂等键).await
}

async fn 为用户发布任务(
    状态: &应用状态,
    用户: 当前用户,
    事件内容: 任务内容请求,
    幂等键: String,
) -> Result<(StatusCode, Json<任务视图>), 应用错误> {
    let 标题 = 事件内容.标题.trim();
    let 描述 = 事件内容.描述.trim();
    校验任务内容(标题, 描述, 事件内容.截止时间)?;
    let 位置 = 校验任务位置(事件内容.位置)?;
    let 编号 = Uuid::new_v4();
    let mut 事务 = 状态.数据库.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1)::bigint)")
        .bind(format!("{}:{}", 用户.编号, 幂等键))
        .execute(&mut *事务)
        .await?;
    if let Some(已有记录) = sqlx::query(
        r#"SELECT "任务编号", "事件类型" FROM "任务事件"
           WHERE "操作者编号" = $1 AND "请求幂等键" = $2"#,
    )
    .bind(用户.编号)
    .bind(&幂等键)
    .fetch_optional(&mut *事务)
    .await?
    {
        if 已有记录.get::<String, _>("事件类型") != "task.created" {
            return Err(应用错误::冲突("此幂等键已用于其他任务操作".into()));
        }
        let 已有编号: Uuid = 已有记录.get("任务编号");
        let 记录 = sqlx::query(任务记录查询(
            r#"SELECT {任务视图字段} FROM "任务" WHERE "任务编号" = $1"#,
        ))
        .bind(已有编号)
        .fetch_one(&mut *事务)
        .await?;
        事务.commit().await?;
        return Ok((StatusCode::OK, Json(映射任务记录(&记录))));
    }
    let 经纬 = 位置.as_ref().map(|位置| PgPoint {
        x: 位置.经度,
        y: 位置.纬度,
    });
    let 记录 = sqlx::query(任务记录查询(
        r#"INSERT INTO "任务"
           ("任务编号", "发布者编号", "标题", "描述", "任务状态", "截止时间",
            "经纬", "国", "省", "市", "县", "镇", "村", "地址")
           VALUES ($1, $2, $3, $4, '待接单', $5, $6, $7, $8, $9, $10, $11, $12, $13)
           RETURNING {任务视图字段}"#,
    ))
    .bind(编号)
    .bind(用户.编号)
    .bind(标题)
    .bind(描述)
    .bind(事件内容.截止时间)
    .bind(经纬)
    .bind(位置.as_ref().and_then(|值| 值.国.clone()))
    .bind(位置.as_ref().and_then(|值| 值.省.clone()))
    .bind(位置.as_ref().and_then(|值| 值.市.clone()))
    .bind(位置.as_ref().and_then(|值| 值.县.clone()))
    .bind(位置.as_ref().and_then(|值| 值.镇.clone()))
    .bind(位置.as_ref().and_then(|值| 值.村.clone()))
    .bind(位置.as_ref().and_then(|值| 值.地址.clone()))
    .fetch_one(&mut *事务)
    .await?;
    let 任务 = 映射任务记录(&记录);
    let 事件编号 = Uuid::new_v4();
    let 事件 = json!({
        "事件编号": 事件编号,
        "事件类型": "task.created",
        "结构版本": 2,
        "发生时间": Utc::now(),
        "任务编号": 编号,
        "数据": {"任务状态": "待接单"}
    });
    sqlx::query(
        r#"INSERT INTO "事务发件箱" ("事件编号", "事件类型", "主题", "事件内容")
                   VALUES ($1, 'task.created', $2, $3)"#,
    )
    .bind(事件编号)
    .bind(format!("renwu/{编号}/events"))
    .bind(事件)
    .execute(&mut *事务)
    .await?;
    sqlx::query(
        r#"INSERT INTO "任务事件"
                   ("事件编号", "任务编号", "操作者编号", "事件类型", "新状态", "请求幂等键")
                   VALUES ($1, $2, $3, 'task.created', '待接单', $4)"#,
    )
    .bind(事件编号)
    .bind(编号)
    .bind(用户.编号)
    .bind(幂等键)
    .execute(&mut *事务)
    .await?;
    事务.commit().await?;
    Ok((StatusCode::CREATED, Json(任务)))
}

fn 校验任务内容(
    标题: &str,
    描述: &str,
    截止时间: Option<chrono::DateTime<Utc>>,
) -> Result<(), 应用错误> {
    if 标题.is_empty() || 标题.chars().count() > 120 || 描述.chars().count() > 10_000 {
        return Err(应用错误::请求错误(
            "标题为 1-120 个字符，描述最多 10000 个字符".into(),
        ));
    }
    if 截止时间.is_some_and(|时间| 时间 <= Utc::now()) {
        return Err(应用错误::请求错误(
            "截止时间必须晚于当前时间".into(),
        ));
    }
    Ok(())
}

fn 校验任务位置(
    mut 位置: Option<任务位置>
) -> Result<Option<任务位置>, 应用错误> {
    if let Some(位置) = &mut 位置 {
        if !位置.经度.is_finite()
            || !位置.纬度.is_finite()
            || !(-180.0..=180.0).contains(&位置.经度)
            || !(-90.0..=90.0).contains(&位置.纬度)
        {
            return Err(应用错误::请求错误("任务经纬度超出有效范围".into()));
        }
        清理位置文本(&mut 位置.国, 100, "国家")?;
        清理位置文本(&mut 位置.省, 100, "省")?;
        清理位置文本(&mut 位置.市, 100, "市")?;
        清理位置文本(&mut 位置.县, 100, "县")?;
        清理位置文本(&mut 位置.镇, 100, "镇")?;
        清理位置文本(&mut 位置.村, 100, "村")?;
        清理位置文本(&mut 位置.地址, 1000, "地址")?;
    }
    Ok(位置)
}

fn 清理位置文本(
    字段: &mut Option<String>,
    最大长度: usize,
    字段名称: &str,
) -> Result<(), 应用错误> {
    if let Some(值) = 字段 {
        let 清理后 = 值.trim();
        if 清理后.chars().count() > 最大长度 {
            return Err(应用错误::请求错误(format!(
                "{字段名称}最多 {最大长度} 个字符"
            )));
        }
        *字段 = (!清理后.is_empty()).then(|| 清理后.to_owned());
    }
    Ok(())
}

pub(super) async fn 查询任务列表(
    State(状态): State<应用状态>,
    Query(查询条件): Query<任务列表查询>,
) -> Result<Json<Vec<任务视图>>, 应用错误> {
    if 查询条件.状态.as_ref().is_some_and(|状态值| {
        !["待接单", "已接单", "进行中", "已完成", "已取消", "已过期"].contains(&状态值.as_str())
    }) {
        return Err(应用错误::请求错误("任务状态无效".into()));
    }
    let 数量上限 = 查询条件.数量上限.unwrap_or(50).clamp(1, 100);
    let 记录集 = sqlx::query(任务记录查询(
        r#"SELECT {任务视图字段}
           FROM "任务" WHERE ($1::text IS NULL OR "任务状态" = $1)
           ORDER BY "创建时间" DESC, "任务编号" DESC LIMIT $2"#,
    ))
    .bind(查询条件.状态)
    .bind(数量上限)
    .fetch_all(&状态.数据库)
    .await?;
    Ok(Json(记录集.iter().map(映射任务记录).collect()))
}

pub(super) async fn 查询我的已接任务(
    State(状态): State<应用状态>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<任务视图>>, 应用错误> {
    let 用户 = 请求
        .extensions()
        .get::<当前用户>()
        .cloned()
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;
    let 记录集 = sqlx::query(任务记录查询(
        r#"SELECT {任务视图字段}
           FROM "任务"
           WHERE "接单者编号" = $1
           ORDER BY "更新时间" DESC, "任务编号" DESC"#,
    ))
    .bind(用户.编号)
    .fetch_all(&状态.数据库)
    .await?;
    Ok(Json(记录集.iter().map(映射任务记录).collect()))
}

pub(super) async fn 查询委托(
    State(状态): State<应用状态>,
    Path(编号): Path<Uuid>,
) -> Result<Json<任务视图>, 应用错误> {
    let 记录 = sqlx::query(任务记录查询(
        r#"SELECT {任务视图字段} FROM "任务" WHERE "任务编号" = $1"#,
    ))
    .bind(编号)
    .fetch_optional(&状态.数据库)
    .await?
    .ok_or_else(|| 应用错误::未找到("委托不存在".into()))?;
    Ok(Json(映射任务记录(&记录)))
}

fn 映射任务记录(记录: &sqlx::postgres::PgRow) -> 任务视图 {
    let 经纬: Option<PgPoint> = 记录.get("经纬");
    任务视图 {
        任务编号: 记录.get("任务编号"),
        发布者编号: 记录.get("发布者编号"),
        接单者编号: 记录.get("接单者编号"),
        标题: 记录.get("标题"),
        描述: 记录.get("描述"),
        任务状态: 记录.get("任务状态"),
        截止时间: 记录.get("截止时间"),
        创建时间: 记录.get("创建时间"),
        版本号: 记录.get("版本号"),
        位置: 经纬.map(|经纬| 任务位置 {
            经度: 经纬.x,
            纬度: 经纬.y,
            国: 记录.get("国"),
            省: 记录.get("省"),
            市: 记录.get("市"),
            县: 记录.get("县"),
            镇: 记录.get("镇"),
            村: 记录.get("村"),
            地址: 记录.get("地址"),
        }),
    }
}

pub(super) async fn 修改任务(
    State(状态): State<应用状态>,
    Path(编号): Path<Uuid>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<任务视图>, 应用错误> {
    let 用户 = 请求
        .extensions()
        .get::<当前用户>()
        .cloned()
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;
    let 幂等键 = 提取幂等键(请求.headers())?;
    let 内容: 任务内容请求 = serde_json::from_slice(
        &axum::body::to_bytes(请求.into_body(), 1_000_000)
            .await
            .map_err(|错误| 应用错误::请求错误(错误.to_string()))?,
    )
    .map_err(|错误| 应用错误::请求错误(错误.to_string()))?;
    修改用户任务(&状态, 用户, 编号, 内容, 幂等键).await
}

async fn 修改用户任务(
    状态: &应用状态,
    用户: 当前用户,
    任务编号: Uuid,
    内容: 任务内容请求,
    幂等键: String,
) -> Result<Json<任务视图>, 应用错误> {
    let 标题 = 内容.标题.trim();
    let 描述 = 内容.描述.trim();
    校验任务内容(标题, 描述, 内容.截止时间)?;
    let 位置 = 校验任务位置(内容.位置)?;
    let 经纬 = 位置.as_ref().map(|位置| PgPoint {
        x: 位置.经度,
        y: 位置.纬度,
    });

    let mut 事务 = 状态.数据库.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1)::bigint)")
        .bind(format!("{}:{}", 用户.编号, 幂等键))
        .execute(&mut *事务)
        .await?;
    let 当前记录 = sqlx::query(
        r#"SELECT "发布者编号", "任务状态"
           FROM "任务" WHERE "任务编号" = $1 FOR UPDATE"#,
    )
    .bind(任务编号)
    .fetch_optional(&mut *事务)
    .await?
    .ok_or_else(|| 应用错误::未找到("任务不存在".into()))?;
    if let Some(已有记录) = sqlx::query(
        r#"SELECT "任务编号", "事件类型" FROM "任务事件"
           WHERE "操作者编号" = $1 AND "请求幂等键" = $2"#,
    )
    .bind(用户.编号)
    .bind(&幂等键)
    .fetch_optional(&mut *事务)
    .await?
    {
        if 已有记录.get::<Uuid, _>("任务编号") != 任务编号
            || 已有记录.get::<String, _>("事件类型") != "task.updated"
        {
            return Err(应用错误::冲突("此幂等键已用于其他任务操作".into()));
        }
        let 记录 = sqlx::query(任务记录查询(
            r#"SELECT {任务视图字段} FROM "任务" WHERE "任务编号" = $1"#,
        ))
        .bind(任务编号)
        .fetch_one(&mut *事务)
        .await?;
        事务.commit().await?;
        return Ok(Json(映射任务记录(&记录)));
    }

    if 当前记录.get::<Uuid, _>("发布者编号") != 用户.编号 {
        return Err(应用错误::禁止访问(
            "只有任务发布者可以修改任务".into(),
        ));
    }
    let 当前状态: String = 当前记录.get("任务状态");
    if 当前状态 != "待接单" {
        return Err(应用错误::冲突("只有待接单任务可以修改".into()));
    }

    let 记录 = sqlx::query(任务记录查询(
        r#"UPDATE "任务" SET
                  "标题" = $2, "描述" = $3, "截止时间" = $4,
                  "经纬" = $5, "国" = $6, "省" = $7, "市" = $8,
                  "县" = $9, "镇" = $10, "村" = $11, "地址" = $12,
                  "版本号" = "版本号" + 1, "更新时间" = now()
           WHERE "任务编号" = $1
           RETURNING {任务视图字段}"#,
    ))
    .bind(任务编号)
    .bind(标题)
    .bind(描述)
    .bind(内容.截止时间)
    .bind(经纬)
    .bind(位置.as_ref().and_then(|值| 值.国.clone()))
    .bind(位置.as_ref().and_then(|值| 值.省.clone()))
    .bind(位置.as_ref().and_then(|值| 值.市.clone()))
    .bind(位置.as_ref().and_then(|值| 值.县.clone()))
    .bind(位置.as_ref().and_then(|值| 值.镇.clone()))
    .bind(位置.as_ref().and_then(|值| 值.村.clone()))
    .bind(位置.as_ref().and_then(|值| 值.地址.clone()))
    .fetch_one(&mut *事务)
    .await?;
    let 任务 = 映射任务记录(&记录);
    let 事件编号 = Uuid::new_v4();
    let 事件 = json!({
        "事件编号": 事件编号,
        "事件类型": "task.updated",
        "结构版本": 2,
        "发生时间": Utc::now(),
        "任务编号": 任务编号,
        "数据": {"任务状态": 当前状态, "版本号": 任务.版本号}
    });
    sqlx::query(
        r#"INSERT INTO "任务事件"
                   ("事件编号", "任务编号", "操作者编号", "事件类型", "原状态", "新状态", "请求幂等键")
                   VALUES ($1, $2, $3, 'task.updated', $4, $4, $5)"#,
    )
    .bind(事件编号)
    .bind(任务编号)
    .bind(用户.编号)
    .bind(当前状态)
    .bind(幂等键)
    .execute(&mut *事务)
    .await?;
    sqlx::query(
        r#"INSERT INTO "事务发件箱" ("事件编号", "事件类型", "主题", "事件内容")
                   VALUES ($1, 'task.updated', $2, $3)"#,
    )
    .bind(事件编号)
    .bind(format!("renwu/{任务编号}/events"))
    .bind(事件)
    .execute(&mut *事务)
    .await?;
    事务.commit().await?;
    Ok(Json(任务))
}

async fn 流转任务状态(
    状态: &应用状态,
    用户: 当前用户,
    任务编号: Uuid,
    操作: &str,
    幂等键: String,
) -> Result<Json<任务视图>, 应用错误> {
    let mut 事务 = 状态.数据库.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1)::bigint)")
        .bind(format!("{}:{}", 用户.编号, 幂等键))
        .execute(&mut *事务)
        .await?;
    let 当前记录 = sqlx::query(
        r#"SELECT "发布者编号", "接单者编号", "任务状态"
           FROM "任务" WHERE "任务编号" = $1 FOR UPDATE"#,
    )
    .bind(任务编号)
    .fetch_optional(&mut *事务)
    .await?
    .ok_or_else(|| 应用错误::未找到("任务不存在".into()))?;
    let 预期事件类型 = match 操作 {
        "claim" => "task.claimed",
        "start" => "task.started",
        "complete" => "task.completed",
        "cancel" => "task.cancelled",
        "release" => "task.released",
        _ => return Err(应用错误::请求错误("未知任务操作".into())),
    };
    if let Some(已有记录) = sqlx::query(
        r#"SELECT "任务编号", "事件类型" FROM "任务事件"
           WHERE "操作者编号" = $1 AND "请求幂等键" = $2"#,
    )
    .bind(用户.编号)
    .bind(&幂等键)
    .fetch_optional(&mut *事务)
    .await?
    {
        if 已有记录.get::<Uuid, _>("任务编号") != 任务编号
            || 已有记录.get::<String, _>("事件类型") != 预期事件类型
        {
            return Err(应用错误::冲突("此幂等键已用于其他任务操作".into()));
        }
        let 记录 = sqlx::query(任务记录查询(
            r#"SELECT {任务视图字段} FROM "任务" WHERE "任务编号" = $1"#,
        ))
        .bind(任务编号)
        .fetch_one(&mut *事务)
        .await?;
        事务.commit().await?;
        return Ok(Json(映射任务记录(&记录)));
    }
    let 发布者: Uuid = 当前记录.get("发布者编号");
    let 接单者: Option<Uuid> = 当前记录.get("接单者编号");
    let 原任务状态: String = 当前记录.get("任务状态");
    let (new_status, new_claimant, 事件类型) =
        计算任务状态(操作, &原任务状态, 发布者, 接单者, 用户.编号).map_err(|err| match err {
            任务状态错误::禁止操作(消息) => 应用错误::禁止访问(消息.to_owned()),
            任务状态错误::状态不允许(状态) => {
                应用错误::冲突(format!("当前状态“{状态}”不允许此操作"))
            }
        })?;
    let 记录 = sqlx::query(任务记录查询(
        r#"UPDATE "任务" SET "任务状态" = $2, "接单者编号" = $3,
                  "接单时间" = CASE WHEN $2 = '已接单' THEN now()
                                    WHEN $2 = '待接单' THEN NULL ELSE "接单时间" END,
                  "版本号" = "版本号" + 1, "更新时间" = now()
           WHERE "任务编号" = $1
           RETURNING {任务视图字段}"#,
    ))
    .bind(任务编号)
    .bind(new_status)
    .bind(new_claimant)
    .fetch_one(&mut *事务)
    .await?;
    let 事件编号 = Uuid::new_v4();
    let 主题 = format!("renwu/{任务编号}/events");
    let 事件 = json!({
        "事件编号": 事件编号,
        "事件类型": 事件类型,
        "结构版本": 2,
        "发生时间": Utc::now(),
        "任务编号": 任务编号,
        "数据": {"任务状态": new_status}
    });
    sqlx::query(
        r#"INSERT INTO "任务事件"
                   ("事件编号", "任务编号", "操作者编号", "事件类型", "原状态", "新状态", "请求幂等键")
                   VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
    )
    .bind(事件编号)
    .bind(任务编号)
    .bind(用户.编号)
    .bind(事件类型)
    .bind(原任务状态)
    .bind(new_status)
    .bind(幂等键)
    .execute(&mut *事务)
    .await?;
    sqlx::query(
        r#"INSERT INTO "事务发件箱" ("事件编号", "事件类型", "主题", "事件内容")
                   VALUES ($1, $2, $3, $4)"#,
    )
    .bind(事件编号)
    .bind(事件类型)
    .bind(主题)
    .bind(事件)
    .execute(&mut *事务)
    .await?;
    事务.commit().await?;
    Ok(Json(映射任务记录(&记录)))
}

pub(super) async fn 接取任务(
    State(s): State<应用状态>,
    Path(编号): Path<Uuid>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<任务视图>, 应用错误> {
    let 用户 = 请求
        .extensions()
        .get::<当前用户>()
        .cloned()
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;
    let 键 = 提取幂等键(请求.headers())?;
    流转任务状态(&s, 用户, 编号, "claim", 键).await
}

pub(super) async fn 开始任务(
    State(s): State<应用状态>,
    Path(编号): Path<Uuid>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<任务视图>, 应用错误> {
    let 用户 = 请求
        .extensions()
        .get::<当前用户>()
        .cloned()
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;
    let 键 = 提取幂等键(请求.headers())?;
    流转任务状态(&s, 用户, 编号, "start", 键).await
}

pub(super) async fn 完成任务(
    State(s): State<应用状态>,
    Path(编号): Path<Uuid>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<任务视图>, 应用错误> {
    let 用户 = 请求
        .extensions()
        .get::<当前用户>()
        .cloned()
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;
    let 键 = 提取幂等键(请求.headers())?;
    流转任务状态(&s, 用户, 编号, "complete", 键).await
}

pub(super) async fn 取消任务(
    State(s): State<应用状态>,
    Path(编号): Path<Uuid>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<任务视图>, 应用错误> {
    let 用户 = 请求
        .extensions()
        .get::<当前用户>()
        .cloned()
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;
    let 键 = 提取幂等键(请求.headers())?;
    流转任务状态(&s, 用户, 编号, "cancel", 键).await
}

pub(super) async fn 释放任务(
    State(s): State<应用状态>,
    Path(编号): Path<Uuid>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<任务视图>, 应用错误> {
    let 用户 = 请求
        .extensions()
        .get::<当前用户>()
        .cloned()
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;
    let 键 = 提取幂等键(请求.headers())?;
    流转任务状态(&s, 用户, 编号, "release", 键).await
}

#[cfg(test)]
mod 位置校验测试 {
    use super::*;

    #[test]
    fn 任务创建请求可接收地图位置并序列化为中文字段() {
        let 请求: 任务内容请求 = serde_json::from_str(
            r#"{
                "title":"测试任务",
                "description":"",
                "location":{
                    "longitude":116.397428,
                    "latitude":39.90923,
                    "country":"中国",
                    "province":"北京市",
                    "city":"北京市",
                    "county":"东城区",
                    "town":"东华门街道",
                    "village":"示例社区",
                    "address":"示例门牌"
                }
            }"#,
        )
        .expect("应解析英文 API 位置字段");
        let 位置 = 请求.位置.expect("应包含位置");
        let 数据 = serde_json::to_value(位置).expect("位置应可序列化");
        assert_eq!(数据["经度"], 116.397428);
        assert_eq!(数据["纬度"], 39.90923);
        assert_eq!(数据["县"], "东城区");
        assert_eq!(数据["地址"], "示例门牌");
    }

    #[test]
    fn 任务位置拒绝超范围坐标并清理地址字段() {
        let 错误 = 校验任务位置(Some(任务位置 {
            经度: 181.0,
            纬度: 39.9,
            国: None,
            省: None,
            市: None,
            县: None,
            镇: None,
            村: None,
            地址: None,
        }))
        .expect_err("无效经度必须被拒绝");
        assert!(matches!(错误, 应用错误::请求错误(_)));

        let 位置 = 校验任务位置(Some(任务位置 {
            经度: 116.4,
            纬度: 39.9,
            国: Some(" 中国 ".into()),
            省: None,
            市: None,
            县: None,
            镇: None,
            村: None,
            地址: Some("   ".into()),
        }))
        .expect("有效位置应通过校验")
        .expect("位置应保留");
        assert_eq!(位置.国.as_deref(), Some("中国"));
        assert_eq!(位置.地址, None);
    }
}
