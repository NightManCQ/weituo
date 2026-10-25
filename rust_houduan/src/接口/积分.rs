use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use serde_json::json;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use super::提取幂等键;
use crate::共享::*;

const 系统发行账户编号: Uuid = Uuid::from_u128(0x20000000000040008000000000000001);
pub(crate) const 单笔积分上限: i64 = 1_000_000_000;

#[derive(serde::Serialize)]
pub(crate) struct 积分账户视图 {
    pub(crate) 可用积分: i64,
    pub(crate) 冻结积分: i64,
    pub(crate) 总积分: i64,
}

#[derive(serde::Serialize)]
pub(crate) struct 积分流水视图 {
    pub(crate) 流水编号: Uuid,
    pub(crate) 交易编号: Uuid,
    pub(crate) 交易类型: String,
    pub(crate) 委托编号: Option<Uuid>,
    pub(crate) 说明: String,
    pub(crate) 可用变动: i64,
    pub(crate) 冻结变动: i64,
    pub(crate) 可用余额: i64,
    pub(crate) 冻结余额: i64,
    pub(crate) 创建时间: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub(crate) struct 积分发放请求 {
    #[serde(rename = "amount")]
    积分数量: i64,
    #[serde(rename = "description", default)]
    说明: String,
}

#[derive(Deserialize)]
pub(crate) struct 积分流水查询 {
    #[serde(rename = "limit")]
    数量上限: Option<i64>,
}

#[derive(Clone, Copy)]
enum 余额类型 {
    可用,
    冻结,
}

impl 余额类型 {
    fn 字段(self) -> &'static str {
        match self {
            Self::可用 => "可用",
            Self::冻结 => "冻结",
        }
    }

    fn 流水可用变动(self, 数量: i64, 正向: bool) -> i64 {
        if matches!(self, Self::可用) {
            if 正向 { 数量 } else { -数量 }
        } else {
            0
        }
    }

    fn 流水冻结变动(self, 数量: i64, 正向: bool) -> i64 {
        if matches!(self, Self::冻结) {
            if 正向 { 数量 } else { -数量 }
        } else {
            0
        }
    }
}

pub(crate) async fn 确保用户积分账户(
    事务: &mut Transaction<'_, Postgres>,
    用户编号: Uuid,
) -> Result<Uuid, 应用错误> {
    sqlx::query(
        r#"INSERT INTO "积分账户" ("账户编号", "用户编号", "账户类型")
           VALUES ($1, $2, '用户')
           ON CONFLICT ("用户编号") DO NOTHING"#,
    )
    .bind(Uuid::new_v4())
    .bind(用户编号)
    .execute(&mut **事务)
    .await?;
    let 记录 = sqlx::query(r#"SELECT "账户编号" FROM "积分账户" WHERE "用户编号" = $1"#)
        .bind(用户编号)
        .fetch_optional(&mut **事务)
        .await?
        .ok_or_else(|| 应用错误::内部错误(anyhow::anyhow!("用户积分账户创建后仍不存在")))?;
    Ok(记录.get("账户编号"))
}

async fn 查询用户积分账户(
    数据库: &sqlx::PgPool,
    用户编号: Uuid,
) -> Result<积分账户视图, 应用错误> {
    let mut 事务 = 数据库.begin().await?;
    let 账户编号 = 确保用户积分账户(&mut 事务, 用户编号).await?;
    let 记录 = sqlx::query(
        r#"SELECT "可用积分", "冻结积分"
           FROM "积分账户" WHERE "账户编号" = $1"#,
    )
    .bind(账户编号)
    .fetch_one(&mut *事务)
    .await?;
    let 可用积分: i64 = 记录.get("可用积分");
    let 冻结积分: i64 = 记录.get("冻结积分");
    事务.commit().await?;
    构建积分账户视图(可用积分, 冻结积分)
}

pub(super) async fn 查询我的积分账户(
    State(状态): State<应用状态>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<积分账户视图>, 应用错误> {
    let 用户 = 请求
        .extensions()
        .get::<当前用户>()
        .cloned()
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;
    Ok(Json(查询用户积分账户(&状态.数据库, 用户.编号).await?))
}

pub(super) async fn 查询我的积分流水(
    State(状态): State<应用状态>,
    Query(查询条件): Query<积分流水查询>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<积分流水视图>>, 应用错误> {
    let 用户 = 请求
        .extensions()
        .get::<当前用户>()
        .cloned()
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;
    let 上限 = 查询条件.数量上限.unwrap_or(50).clamp(1, 100);
    let mut 事务 = 状态.数据库.begin().await?;
    let 账户编号 = 确保用户积分账户(&mut 事务, 用户.编号).await?;
    let 记录集 = sqlx::query(
        r#"SELECT l."流水编号", l."交易编号", t."交易类型", t."委托编号",
                  t."说明", l."可用变动", l."冻结变动", l."可用余额",
                  l."冻结余额", l."创建时间"
           FROM "积分流水" l
           JOIN "积分交易" t ON t."交易编号" = l."交易编号"
           WHERE l."账户编号" = $1
           ORDER BY l."创建时间" DESC, l."流水编号" DESC
           LIMIT $2"#,
    )
    .bind(账户编号)
    .bind(上限)
    .fetch_all(&mut *事务)
    .await?;
    let 流水 = 记录集
        .iter()
        .map(|记录| 积分流水视图 {
            流水编号: 记录.get("流水编号"),
            交易编号: 记录.get("交易编号"),
            交易类型: 记录.get("交易类型"),
            委托编号: 记录.get("委托编号"),
            说明: 记录.get("说明"),
            可用变动: 记录.get("可用变动"),
            冻结变动: 记录.get("冻结变动"),
            可用余额: 记录.get("可用余额"),
            冻结余额: 记录.get("冻结余额"),
            创建时间: 记录.get("创建时间"),
        })
        .collect();
    事务.commit().await?;
    Ok(Json(流水))
}

pub(super) async fn 管理员发放积分(
    State(状态): State<应用状态>,
    Path(目标用户编号): Path<Uuid>,
    请求: axum::http::Request<axum::body::Body>,
) -> Result<Json<积分账户视图>, 应用错误> {
    let 操作者 = 请求
        .extensions()
        .get::<当前用户>()
        .cloned()
        .ok_or_else(|| 应用错误::未授权("请先登录".into()))?;
    if !操作者
        .角色列表
        .iter()
        .any(|角色| 角色 == "管理员" || 角色 == "运营员")
    {
        return Err(应用错误::禁止访问("需要运营员或管理员权限".into()));
    }
    let 幂等键 = 提取幂等键(请求.headers())?;
    let 内容: 积分发放请求 = serde_json::from_slice(
        &axum::body::to_bytes(请求.into_body(), 64_000)
            .await
            .map_err(|错误| 应用错误::请求错误(错误.to_string()))?,
    )
    .map_err(|错误| 应用错误::请求错误(错误.to_string()))?;
    let 说明 = 内容.说明.trim();
    if !(1..=单笔积分上限).contains(&内容.积分数量) || 说明.chars().count() > 500 {
        return Err(应用错误::请求错误(format!(
            "发放积分需为 1 至 {单笔积分上限} 的整数，说明最多 500 个字符"
        )));
    }

    let mut 事务 = 状态.数据库.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1)::bigint)")
        .bind(format!("积分发放:{}:{幂等键}", 操作者.编号))
        .execute(&mut *事务)
        .await?;
    if let Some(已有记录) = sqlx::query(
        r#"SELECT t."交易类型", t."积分数量", a."用户编号" AS "目标用户编号"
           FROM "积分交易" t
           JOIN "积分账户" a ON a."账户编号" = t."目标账户编号"
           WHERE t."操作者编号" = $1 AND t."请求幂等键" = $2"#,
    )
    .bind(操作者.编号)
    .bind(format!("issue:{幂等键}"))
    .fetch_optional(&mut *事务)
    .await?
    {
        if 已有记录.get::<String, _>("交易类型") != "系统发放"
            || 已有记录.get::<Uuid, _>("目标用户编号") != 目标用户编号
            || 已有记录.get::<i64, _>("积分数量") != 内容.积分数量
        {
            return Err(应用错误::冲突("此幂等键已用于不同的积分发放".into()));
        }
        let 账户编号 = 确保用户积分账户(&mut 事务, 目标用户编号).await?;
        let 账户 = 查询账户余额(&mut 事务, 账户编号).await?;
        事务.commit().await?;
        return Ok(Json(账户));
    }

    let 用户存在 = sqlx::query(r#"SELECT 1 FROM "用户" WHERE "用户编号" = $1"#)
        .bind(目标用户编号)
        .fetch_optional(&mut *事务)
        .await?
        .is_some();
    if !用户存在 {
        return Err(应用错误::未找到("目标用户不存在".into()));
    }
    let 目标账户编号 = 确保用户积分账户(&mut 事务, 目标用户编号).await?;
    转移积分(
        &mut 事务,
        转账参数 {
            交易类型: "系统发放",
            来源账户编号: 系统发行账户编号,
            来源余额类型: 余额类型::可用,
            目标账户编号,
            目标余额类型: 余额类型::可用,
            积分数量: 内容.积分数量,
            委托编号: None,
            操作者编号: Some(操作者.编号),
            幂等键: Some(format!("issue:{幂等键}")),
            说明: 说明.to_owned(),
        },
    )
    .await?;
    sqlx::query(
        r#"INSERT INTO "操作审计"
           ("审计编号", "操作者编号", "目标用户编号", "操作类型", "操作内容")
           VALUES ($1, $2, $3, 'points.issued', $4)"#,
    )
    .bind(Uuid::new_v4())
    .bind(操作者.编号)
    .bind(目标用户编号)
    .bind(json!({"积分数量": 内容.积分数量, "说明": 说明}))
    .execute(&mut *事务)
    .await?;
    let 账户 = 查询账户余额(&mut 事务, 目标账户编号).await?;
    事务.commit().await?;
    Ok(Json(账户))
}

struct 转账参数<'a> {
    交易类型: &'a str,
    来源账户编号: Uuid,
    来源余额类型: 余额类型,
    目标账户编号: Uuid,
    目标余额类型: 余额类型,
    积分数量: i64,
    委托编号: Option<Uuid>,
    操作者编号: Option<Uuid>,
    幂等键: Option<String>,
    说明: String,
}

async fn 转移积分(
    事务: &mut Transaction<'_, Postgres>,
    参数: 转账参数<'_>,
) -> Result<(), 应用错误> {
    let 交易编号 = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO "积分交易"
           ("交易编号", "交易类型", "来源账户编号", "目标账户编号",
            "来源余额类型", "目标余额类型", "积分数量", "委托编号",
            "操作者编号", "请求幂等键", "说明")
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"#,
    )
    .bind(交易编号)
    .bind(参数.交易类型)
    .bind(参数.来源账户编号)
    .bind(参数.目标账户编号)
    .bind(参数.来源余额类型.字段())
    .bind(参数.目标余额类型.字段())
    .bind(参数.积分数量)
    .bind(参数.委托编号)
    .bind(参数.操作者编号)
    .bind(参数.幂等键)
    .bind(&参数.说明)
    .execute(&mut **事务)
    .await?;

    let mut 账户编号列表 = vec![参数.来源账户编号, 参数.目标账户编号];
    账户编号列表.sort_unstable();
    账户编号列表.dedup();
    for 账户编号 in &账户编号列表 {
        sqlx::query(r#"SELECT "账户编号" FROM "积分账户" WHERE "账户编号" = $1 FOR UPDATE"#)
            .bind(账户编号)
            .fetch_optional(&mut **事务)
            .await?
            .ok_or_else(|| {
                应用错误::内部错误(anyhow::anyhow!("积分交易引用了不存在的账户"))
            })?;
    }

    let 来源变更 = match 参数.来源余额类型 {
        余额类型::可用 => {
            sqlx::query(
                r#"UPDATE "积分账户" SET "可用积分" = "可用积分" - $2, "更新时间" = now()
                   WHERE "账户编号" = $1 AND ("账户类型" = '发行' OR "可用积分" >= $2)"#,
            )
            .bind(参数.来源账户编号)
            .bind(参数.积分数量)
            .execute(&mut **事务)
            .await?
        }
        余额类型::冻结 => {
            sqlx::query(
                r#"UPDATE "积分账户" SET "冻结积分" = "冻结积分" - $2, "更新时间" = now()
                   WHERE "账户编号" = $1 AND "冻结积分" >= $2"#,
            )
            .bind(参数.来源账户编号)
            .bind(参数.积分数量)
            .execute(&mut **事务)
            .await?
        }
    };
    if 来源变更.rows_affected() != 1 {
        let 余额类型 = match 参数.来源余额类型 {
            余额类型::可用 => "可用",
            余额类型::冻结 => "冻结",
        };
        return Err(应用错误::冲突(format!(
            "{余额类型}积分不足，无法冻结或转出"
        )));
    }

    match 参数.目标余额类型 {
        余额类型::可用 => {
            sqlx::query(
                r#"UPDATE "积分账户" SET "可用积分" = "可用积分" + $2, "更新时间" = now()
                   WHERE "账户编号" = $1"#,
            )
            .bind(参数.目标账户编号)
            .bind(参数.积分数量)
            .execute(&mut **事务)
            .await?;
        }
        余额类型::冻结 => {
            sqlx::query(
                r#"UPDATE "积分账户" SET "冻结积分" = "冻结积分" + $2, "更新时间" = now()
                   WHERE "账户编号" = $1"#,
            )
            .bind(参数.目标账户编号)
            .bind(参数.积分数量)
            .execute(&mut **事务)
            .await?;
        }
    }

    if 参数.来源账户编号 == 参数.目标账户编号 {
        let 可用变动 = 参数.来源余额类型.流水可用变动(参数.积分数量, false)
            + 参数.目标余额类型.流水可用变动(参数.积分数量, true);
        let 冻结变动 = 参数.来源余额类型.流水冻结变动(参数.积分数量, false)
            + 参数.目标余额类型.流水冻结变动(参数.积分数量, true);
        插入积分流水(事务, 交易编号, 参数.来源账户编号, 可用变动, 冻结变动).await?;
    } else {
        插入积分流水(
            事务,
            交易编号,
            参数.来源账户编号,
            参数.来源余额类型.流水可用变动(参数.积分数量, false),
            参数.来源余额类型.流水冻结变动(参数.积分数量, false),
        )
        .await?;
        插入积分流水(
            事务,
            交易编号,
            参数.目标账户编号,
            参数.目标余额类型.流水可用变动(参数.积分数量, true),
            参数.目标余额类型.流水冻结变动(参数.积分数量, true),
        )
        .await?;
    }
    Ok(())
}

async fn 插入积分流水(
    事务: &mut Transaction<'_, Postgres>,
    交易编号: Uuid,
    账户编号: Uuid,
    可用变动: i64,
    冻结变动: i64,
) -> Result<(), 应用错误> {
    let 账户 = 查询账户余额(事务, 账户编号).await?;
    sqlx::query(
        r#"INSERT INTO "积分流水"
           ("流水编号", "交易编号", "账户编号", "可用变动", "冻结变动",
            "可用余额", "冻结余额")
           VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
    )
    .bind(Uuid::new_v4())
    .bind(交易编号)
    .bind(账户编号)
    .bind(可用变动)
    .bind(冻结变动)
    .bind(账户.可用积分)
    .bind(账户.冻结积分)
    .execute(&mut **事务)
    .await?;
    Ok(())
}

async fn 查询账户余额(
    事务: &mut Transaction<'_, Postgres>,
    账户编号: Uuid,
) -> Result<积分账户视图, 应用错误> {
    let 记录 = sqlx::query(
        r#"SELECT "可用积分", "冻结积分"
           FROM "积分账户" WHERE "账户编号" = $1"#,
    )
    .bind(账户编号)
    .fetch_one(&mut **事务)
    .await?;
    let 可用积分: i64 = 记录.get("可用积分");
    let 冻结积分: i64 = 记录.get("冻结积分");
    构建积分账户视图(可用积分, 冻结积分)
}

fn 构建积分账户视图(
    可用积分: i64,
    冻结积分: i64,
) -> Result<积分账户视图, 应用错误> {
    Ok(积分账户视图 {
        可用积分,
        冻结积分,
        总积分: 可用积分.checked_add(冻结积分).ok_or_else(|| {
            应用错误::内部错误(anyhow::anyhow!("积分账户总余额超过整数范围"))
        })?,
    })
}

pub(crate) async fn 调整委托报酬(
    事务: &mut Transaction<'_, Postgres>,
    委托编号: Uuid,
    发布者编号: Uuid,
    旧报酬: Option<i64>,
    新报酬: Option<i64>,
    操作者编号: Uuid,
    幂等键: &str,
) -> Result<(), 应用错误> {
    let (Some(新报酬), 旧报酬) = (新报酬, 旧报酬) else {
        return Ok(());
    };
    if !(0..=单笔积分上限).contains(&新报酬) {
        return Err(应用错误::请求错误(format!(
            "委托报酬需为 0 至 {单笔积分上限} 的整数积分"
        )));
    }
    if 旧报酬 == Some(新报酬) {
        return Ok(());
    }
    let 目标账户编号 = 确保用户积分账户(事务, 发布者编号).await?;
    let 调整数 = match 旧报酬 {
        Some(旧值) => 新报酬 - 旧值,
        None => 新报酬,
    };
    if 调整数 != 0 {
        let (来源余额类型, 目标余额类型, 数量, 交易类型) = if 调整数 > 0 {
            (余额类型::可用, 余额类型::冻结, 调整数, "委托托管")
        } else {
            (余额类型::冻结, 余额类型::可用, -调整数, "委托退款")
        };
        转移积分(
            事务,
            转账参数 {
                交易类型,
                来源账户编号: 目标账户编号,
                来源余额类型,
                目标账户编号,
                目标余额类型,
                积分数量: 数量,
                委托编号: Some(委托编号),
                操作者编号: Some(操作者编号),
                幂等键: Some(format!("commission.edit:{委托编号}:{幂等键}")),
                说明: "修改委托报酬".into(),
            },
        )
        .await?;
    }
    let 更新结果 = sqlx::query(
        r#"INSERT INTO "委托报酬"
           ("委托编号", "发布者编号", "积分数量", "结算状态")
           VALUES ($1, $2, $3, '托管中')
           ON CONFLICT ("委托编号") DO UPDATE SET
               "积分数量" = EXCLUDED."积分数量", "更新时间" = now()
           WHERE "委托报酬"."结算状态" = '托管中'"#,
    )
    .bind(委托编号)
    .bind(发布者编号)
    .bind(新报酬)
    .execute(&mut **事务)
    .await?;
    if 更新结果.rows_affected() != 1 {
        return Err(应用错误::冲突(
            "委托报酬不处于可调整的托管状态".into(),
        ));
    }
    Ok(())
}

pub(crate) async fn 创建委托报酬托管(
    事务: &mut Transaction<'_, Postgres>,
    委托编号: Uuid,
    发布者编号: Uuid,
    报酬积分: i64,
    操作者编号: Uuid,
    幂等键: &str,
) -> Result<(), 应用错误> {
    if !(0..=单笔积分上限).contains(&报酬积分) {
        return Err(应用错误::请求错误(format!(
            "委托报酬需为 0 至 {单笔积分上限} 的整数积分"
        )));
    }
    if 报酬积分 > 0 {
        let 账户编号 = 确保用户积分账户(事务, 发布者编号).await?;
        转移积分(
            事务,
            转账参数 {
                交易类型: "委托托管",
                来源账户编号: 账户编号,
                来源余额类型: 余额类型::可用,
                目标账户编号: 账户编号,
                目标余额类型: 余额类型::冻结,
                积分数量: 报酬积分,
                委托编号: Some(委托编号),
                操作者编号: Some(操作者编号),
                幂等键: Some(format!("commission.create:{委托编号}:{幂等键}")),
                说明: "发布委托并托管报酬".into(),
            },
        )
        .await?;
    }
    sqlx::query(
        r#"INSERT INTO "委托报酬"
           ("委托编号", "发布者编号", "积分数量", "结算状态")
           VALUES ($1, $2, $3, '托管中')"#,
    )
    .bind(委托编号)
    .bind(发布者编号)
    .bind(报酬积分)
    .execute(&mut **事务)
    .await?;
    Ok(())
}

pub(crate) async fn 提交委托报酬验收(
    事务: &mut Transaction<'_, Postgres>,
    委托编号: Uuid,
) -> Result<(), 应用错误> {
    let 更新结果 = sqlx::query(
        r#"UPDATE "委托报酬" SET "结算状态" = '待验收', "更新时间" = now()
           WHERE "委托编号" = $1 AND "结算状态" = '托管中'"#,
    )
    .bind(委托编号)
    .execute(&mut **事务)
    .await?;
    if 更新结果.rows_affected() == 0 {
        let 有报酬: bool = sqlx::query_scalar(
            r#"SELECT EXISTS(SELECT 1 FROM "任务" WHERE "任务编号" = $1 AND "报酬积分" IS NOT NULL)"#,
        )
        .bind(委托编号)
        .fetch_one(&mut **事务)
        .await?;
        if 有报酬 {
            return Err(应用错误::冲突(
                "委托报酬托管状态异常，无法提交验收".into(),
            ));
        }
    }
    Ok(())
}

pub(crate) async fn 发放委托报酬(
    事务: &mut Transaction<'_, Postgres>,
    委托编号: Uuid,
    接单者编号: Uuid,
    操作者编号: Uuid,
    幂等键: &str,
) -> Result<(), 应用错误> {
    let 报酬 = sqlx::query(
        r#"SELECT "发布者编号", "积分数量", "结算状态"
           FROM "委托报酬" WHERE "委托编号" = $1 FOR UPDATE"#,
    )
    .bind(委托编号)
    .fetch_optional(&mut **事务)
    .await?;
    let Some(报酬) = 报酬 else {
        let 有报酬: bool = sqlx::query_scalar(
            r#"SELECT EXISTS(SELECT 1 FROM "任务" WHERE "任务编号" = $1 AND "报酬积分" IS NOT NULL)"#,
        )
        .bind(委托编号)
        .fetch_one(&mut **事务)
        .await?;
        if 有报酬 {
            return Err(应用错误::冲突(
                "委托缺少积分托管记录，无法验收发放".into(),
            ));
        }
        return Ok(());
    };
    if 报酬.get::<String, _>("结算状态") != "待验收" {
        return Err(应用错误::冲突("委托报酬当前不处于待验收状态".into()));
    }
    let 发布者编号: Uuid = 报酬.get("发布者编号");
    let 数量: i64 = 报酬.get("积分数量");
    if 数量 > 0 {
        let 发布者账户编号 = 确保用户积分账户(事务, 发布者编号).await?;
        let 接单者账户编号 = 确保用户积分账户(事务, 接单者编号).await?;
        转移积分(
            事务,
            转账参数 {
                交易类型: "报酬发放",
                来源账户编号: 发布者账户编号,
                来源余额类型: 余额类型::冻结,
                目标账户编号: 接单者账户编号,
                目标余额类型: 余额类型::可用,
                积分数量: 数量,
                委托编号: Some(委托编号),
                操作者编号: Some(操作者编号),
                幂等键: Some(format!("commission.payout:{委托编号}:{幂等键}")),
                说明: "发布者验收，报酬已发放".into(),
            },
        )
        .await?;
    }
    sqlx::query(
        r#"UPDATE "委托报酬" SET "接单者编号" = $2, "结算状态" = '已发放',
                  "更新时间" = now()
           WHERE "委托编号" = $1 AND "结算状态" = '待验收'"#,
    )
    .bind(委托编号)
    .bind(接单者编号)
    .execute(&mut **事务)
    .await?;
    Ok(())
}

pub(crate) async fn 退款委托报酬(
    事务: &mut Transaction<'_, Postgres>,
    委托编号: Uuid,
    操作者编号: Option<Uuid>,
    幂等键: Option<&str>,
) -> Result<(), 应用错误> {
    let 报酬 = sqlx::query(
        r#"SELECT "发布者编号", "积分数量", "结算状态"
           FROM "委托报酬" WHERE "委托编号" = $1 FOR UPDATE"#,
    )
    .bind(委托编号)
    .fetch_optional(&mut **事务)
    .await?;
    let Some(报酬) = 报酬 else {
        return Ok(());
    };
    if 报酬.get::<String, _>("结算状态") != "托管中" {
        return Err(应用错误::冲突("仅托管中的委托报酬可以退款".into()));
    }
    let 发布者编号: Uuid = 报酬.get("发布者编号");
    let 数量: i64 = 报酬.get("积分数量");
    if 数量 > 0 {
        let 账户编号 = 确保用户积分账户(事务, 发布者编号).await?;
        转移积分(
            事务,
            转账参数 {
                交易类型: "委托退款",
                来源账户编号: 账户编号,
                来源余额类型: 余额类型::冻结,
                目标账户编号: 账户编号,
                目标余额类型: 余额类型::可用,
                积分数量: 数量,
                委托编号: Some(委托编号),
                操作者编号,
                幂等键: 幂等键.map(|键| format!("commission.refund:{委托编号}:{键}")),
                说明: "委托取消或过期，报酬已退回".into(),
            },
        )
        .await?;
    }
    sqlx::query(
        r#"UPDATE "委托报酬" SET "结算状态" = '已退款', "更新时间" = now()
           WHERE "委托编号" = $1 AND "结算状态" = '托管中'"#,
    )
    .bind(委托编号)
    .execute(&mut **事务)
    .await?;
    Ok(())
}

#[cfg(test)]
mod 数据库集成测试 {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    #[tokio::test]
    #[ignore = "需要 rust_houduan/.env 中配置并已迁移的测试 PostgreSQL"]
    async fn 托管验收发放与退款保持账户及流水一致() {
        let _ = dotenvy::dotenv();
        let 数据库地址 = std::env::var("DATABASE_URL").expect("需设置 DATABASE_URL");
        let 数据库 = PgPoolOptions::new()
            .max_connections(1)
            .connect(&数据库地址)
            .await
            .expect("连接测试 PostgreSQL 失败");
        let mut 事务 = 数据库.begin().await.expect("开始测试事务失败");
        let 发布者编号 = Uuid::new_v4();
        let 接单者编号 = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO "用户" ("用户编号", "登录名", "密码摘要", "昵称")
               VALUES ($1, $2, 'integration-test', '积分测试发布者'),
                      ($3, $4, 'integration-test', '积分测试接单者')"#,
        )
        .bind(发布者编号)
        .bind(format!("points_test_{发布者编号}"))
        .bind(接单者编号)
        .bind(format!("points_test_{接单者编号}"))
        .execute(&mut *事务)
        .await
        .expect("插入测试用户失败");
        let 发布者账户编号 = 确保用户积分账户(&mut 事务, 发布者编号)
            .await
            .expect("创建发布者积分账户失败");
        let 接单者账户编号 = 确保用户积分账户(&mut 事务, 接单者编号)
            .await
            .expect("创建接单者积分账户失败");
        转移积分(
            &mut 事务,
            转账参数 {
                交易类型: "系统发放",
                来源账户编号: 系统发行账户编号,
                来源余额类型: 余额类型::可用,
                目标账户编号: 发布者账户编号,
                目标余额类型: 余额类型::可用,
                积分数量: 100,
                委托编号: None,
                操作者编号: None,
                幂等键: None,
                说明: "积分集成测试初始发放".into(),
            },
        )
        .await
        .expect("发放测试积分失败");

        let 验收委托编号 = 创建测试委托(&mut 事务, 发布者编号, 50).await;
        创建委托报酬托管(
            &mut 事务,
            验收委托编号,
            发布者编号,
            50,
            发布者编号,
            "integration-escrow",
        )
        .await
        .expect("冻结测试报酬失败");
        let 托管余额 = 查询账户余额(&mut 事务, 发布者账户编号)
            .await
            .expect("读取托管余额失败");
        assert_eq!((托管余额.可用积分, 托管余额.冻结积分), (50, 50));

        提交委托报酬验收(&mut 事务, 验收委托编号)
            .await
            .expect("提交验收状态失败");
        发放委托报酬(
            &mut 事务,
            验收委托编号,
            接单者编号,
            发布者编号,
            "integration-accept",
        )
        .await
        .expect("发放测试报酬失败");
        let 接单者余额 = 查询账户余额(&mut 事务, 接单者账户编号)
            .await
            .expect("读取接单者余额失败");
        assert_eq!(接单者余额.可用积分, 50);
        let 发布者余额 = 查询账户余额(&mut 事务, 发布者账户编号)
            .await
            .expect("读取发布者结算余额失败");
        assert_eq!((发布者余额.可用积分, 发布者余额.冻结积分), (50, 0));

        let 退款委托编号 = 创建测试委托(&mut 事务, 发布者编号, 30).await;
        创建委托报酬托管(
            &mut 事务,
            退款委托编号,
            发布者编号,
            30,
            发布者编号,
            "integration-refund",
        )
        .await
        .expect("冻结待退款报酬失败");
        退款委托报酬(
            &mut 事务,
            退款委托编号,
            Some(发布者编号),
            Some("integration-cancel"),
        )
        .await
        .expect("退款测试报酬失败");
        let 最终余额 = 查询账户余额(&mut 事务, 发布者账户编号)
            .await
            .expect("读取退款后余额失败");
        assert_eq!((最终余额.可用积分, 最终余额.冻结积分), (50, 0));

        let 零报酬委托编号 = 创建测试委托(&mut 事务, 发布者编号, 0).await;
        创建委托报酬托管(
            &mut 事务,
            零报酬委托编号,
            发布者编号,
            0,
            发布者编号,
            "integration-zero-reward",
        )
        .await
        .expect("创建零报酬委托失败");
        提交委托报酬验收(&mut 事务, 零报酬委托编号)
            .await
            .expect("提交零报酬委托验收状态失败");
        发放委托报酬(
            &mut 事务,
            零报酬委托编号,
            接单者编号,
            发布者编号,
            "integration-zero-accept",
        )
        .await
        .expect("完成零报酬委托验收失败");
        let 零报酬转账数: i64 =
            sqlx::query_scalar(r#"SELECT count(*) FROM "积分交易" WHERE "委托编号" = $1"#)
                .bind(零报酬委托编号)
                .fetch_one(&mut *事务)
                .await
                .expect("核对零报酬交易数失败");
        assert_eq!(零报酬转账数, 0);

        let 最终零报酬余额 = 查询账户余额(&mut 事务, 发布者账户编号)
            .await
            .expect("读取零报酬验收后余额失败");
        assert_eq!((最终零报酬余额.可用积分, 最终零报酬余额.冻结积分), (50, 0));

        let 未平衡流水数: i64 = sqlx::query_scalar(
            r#"SELECT count(*) FROM (
                   SELECT t."交易编号"
                   FROM "积分交易" t
                   JOIN "积分流水" l ON l."交易编号" = t."交易编号"
                   WHERE t."委托编号" IN ($1, $2)
                   GROUP BY t."交易编号"
                   HAVING sum(l."可用变动" + l."冻结变动") <> 0
               ) 不平衡交易"#,
        )
        .bind(验收委托编号)
        .bind(退款委托编号)
        .fetch_one(&mut *事务)
        .await
        .expect("核对积分流水平衡失败");
        assert_eq!(未平衡流水数, 0);

        事务.rollback().await.expect("回滚积分集成测试数据失败");
        数据库.close().await;
    }

    async fn 创建测试委托(
        事务: &mut Transaction<'_, Postgres>,
        发布者编号: Uuid,
        报酬积分: i64,
    ) -> Uuid {
        let 委托编号 = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO "任务"
               ("任务编号", "发布者编号", "标题", "描述", "任务状态", "报酬积分")
               VALUES ($1, $2, '积分集成测试委托', '', '待接单', $3)"#,
        )
        .bind(委托编号)
        .bind(发布者编号)
        .bind(报酬积分)
        .execute(&mut **事务)
        .await
        .expect("插入积分集成测试委托失败");
        委托编号
    }
}
