use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use rumqttc::AsyncClient;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};
use thiserror::Error;
use tokio::sync::oneshot;
use tracing::error;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct 应用状态 {
    pub(crate) 数据库: PgPool,
    pub(crate) 令牌密钥: String,
    pub(crate) 消息代理: 消息发布器,
    pub(crate) 高德客户端: reqwest::Client,
    pub(crate) 高德安全密钥: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct 接口错误 {
    pub(crate) 消息: String,
}

#[derive(Debug, Error)]
pub(crate) enum 应用错误 {
    #[error("{0}")]
    请求错误(String),
    #[error("{0}")]
    未授权(String),
    #[error("{0}")]
    禁止访问(String),
    #[error("{0}")]
    未找到(String),
    #[error("{0}")]
    冲突(String),
    #[error("{0}")]
    服务不可用(String),
    #[error("服务内部错误")]
    内部错误(#[from] anyhow::Error),
    #[error("数据库操作失败")]
    数据库错误(#[from] sqlx::Error),
}

impl IntoResponse for 应用错误 {
    fn into_response(self) -> Response {
        let (状态, 消息) = match self {
            Self::请求错误(消息) => (StatusCode::BAD_REQUEST, 消息),
            Self::未授权(消息) => (StatusCode::UNAUTHORIZED, 消息),
            Self::禁止访问(消息) => (StatusCode::FORBIDDEN, 消息),
            Self::未找到(消息) => (StatusCode::NOT_FOUND, 消息),
            Self::冲突(消息) => (StatusCode::CONFLICT, 消息),
            Self::服务不可用(消息) => (StatusCode::SERVICE_UNAVAILABLE, 消息),
            Self::内部错误(错误) => {
                error!(错误 = %错误, "request failed");
                (StatusCode::INTERNAL_SERVER_ERROR, "服务内部错误".to_owned())
            }
            Self::数据库错误(错误) => {
                error!(错误 = %错误, "database operation failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "数据库暂不可用".to_owned(),
                )
            }
        };
        (状态, Json(接口错误 { 消息 })).into_response()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct 令牌声明 {
    #[serde(rename = "sub")]
    pub(crate) 主体: Uuid,
    #[serde(rename = "exp")]
    pub(crate) 过期时间: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct 当前用户 {
    pub(crate) 编号: Uuid,
    pub(crate) 角色列表: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct 认证响应 {
    pub(crate) 访问令牌: String,
    pub(crate) 令牌类型: &'static str,
    pub(crate) 有效秒数: u64,
    pub(crate) 用户: 用户视图,
}

#[derive(Serialize)]
pub(crate) struct 用户视图 {
    pub(crate) 用户编号: Uuid,
    pub(crate) 登录名: String,
    pub(crate) 昵称: String,
}

#[derive(Deserialize)]
pub(crate) struct 注册请求 {
    #[serde(rename = "login_name")]
    pub(crate) 登录名: String,
    #[serde(rename = "password")]
    pub(crate) 密码: String,
    #[serde(rename = "nickname")]
    pub(crate) 昵称: String,
}

#[derive(Deserialize)]
pub(crate) struct 登录请求 {
    #[serde(rename = "login_name")]
    pub(crate) 登录名: String,
    #[serde(rename = "password")]
    pub(crate) 密码: String,
}

#[derive(Deserialize)]
pub(crate) struct 任务内容请求 {
    #[serde(rename = "title")]
    pub(crate) 标题: String,
    #[serde(rename = "description")]
    pub(crate) 描述: String,
    #[serde(rename = "deadline")]
    pub(crate) 截止时间: Option<DateTime<Utc>>,
    #[serde(rename = "location")]
    pub(crate) 位置: Option<任务位置>,
    #[serde(rename = "reward_points", default)]
    pub(crate) 报酬积分: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct 任务位置 {
    #[serde(rename = "经度", alias = "longitude", alias = "lng")]
    pub(crate) 经度: f64,
    #[serde(rename = "纬度", alias = "latitude", alias = "lat")]
    pub(crate) 纬度: f64,
    #[serde(rename = "国", alias = "country", default)]
    pub(crate) 国: Option<String>,
    #[serde(rename = "省", alias = "province", default)]
    pub(crate) 省: Option<String>,
    #[serde(rename = "市", alias = "city", default)]
    pub(crate) 市: Option<String>,
    #[serde(rename = "县", alias = "county", alias = "district", default)]
    pub(crate) 县: Option<String>,
    #[serde(rename = "镇", alias = "town", alias = "township", default)]
    pub(crate) 镇: Option<String>,
    #[serde(rename = "村", alias = "village", default)]
    pub(crate) 村: Option<String>,
    #[serde(rename = "地址", alias = "address", default)]
    pub(crate) 地址: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct 任务视图 {
    pub(crate) 任务编号: Uuid,
    pub(crate) 发布者编号: Uuid,
    pub(crate) 接单者编号: Option<Uuid>,
    pub(crate) 标题: String,
    pub(crate) 描述: String,
    pub(crate) 任务状态: String,
    pub(crate) 截止时间: Option<DateTime<Utc>>,
    pub(crate) 创建时间: DateTime<Utc>,
    pub(crate) 版本号: i32,
    pub(crate) 位置: Option<任务位置>,
    pub(crate) 报酬积分: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct 任务列表查询 {
    #[serde(rename = "status")]
    pub(crate) 状态: Option<String>,
    #[serde(rename = "limit")]
    pub(crate) 数量上限: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct 实时连接信息 {
    pub(crate) 消息地址: String,
    pub(crate) 消息令牌: String,
    pub(crate) 主题列表: Vec<String>,
    pub(crate) 有效秒数: u64,
    pub(crate) 说明: &'static str,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct 消息令牌声明 {
    #[serde(rename = "exp")]
    pub(crate) 过期时间: usize,
    #[serde(rename = "username")]
    pub(crate) 用户名: String,
    #[serde(rename = "acl")]
    pub(crate) 访问控制: 消息访问控制,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct 消息访问控制 {
    #[serde(rename = "pub")]
    pub(crate) 发布权限: Vec<String>,
    #[serde(rename = "sub")]
    pub(crate) 订阅权限: Vec<String>,
}

pub(crate) fn 是安全的_mqtt账号主题段(账号: &str) -> bool {
    !账号.is_empty()
        && !账号
            .chars()
            .any(|字符| matches!(字符, '/' | '+' | '#' | '\0'))
}

pub(crate) fn 构造任务事件主题(
    关联群组编号: Option<Uuid>,
    发布者登录名: &str,
) -> anyhow::Result<String> {
    if let Some(群组编号) = 关联群组编号 {
        return Ok(format!("q/{群组编号}"));
    }
    anyhow::ensure!(
        是安全的_mqtt账号主题段(发布者登录名),
        "发布者登录名不能安全地用作 MQTT 主题段"
    );
    Ok(format!("u/{发布者登录名}"))
}

#[cfg(test)]
mod mqtt_账号主题测试 {
    use super::{是安全的_mqtt账号主题段, 构造任务事件主题};
    use uuid::Uuid;

    #[test]
    fn 账号可作为单个通知主题段() {
        assert!(是安全的_mqtt账号主题段("changqing"));
        assert!(是安全的_mqtt账号主题段("用户.账号_01"));
        assert!(!是安全的_mqtt账号主题段(""));
        assert!(!是安全的_mqtt账号主题段("user/account"));
        assert!(!是安全的_mqtt账号主题段("user+test"));
        assert!(!是安全的_mqtt账号主题段("user#"));
        assert!(!是安全的_mqtt账号主题段("user\0test"));
    }

    #[test]
    fn 委托任务事件优先使用关联群主题() {
        let 群组编号 = Uuid::new_v4();
        assert_eq!(
            构造任务事件主题(Some(群组编号), "changqing").unwrap(),
            format!("q/{群组编号}")
        );
        assert_eq!(构造任务事件主题(None, "changqing").unwrap(), "u/changqing");
        assert!(构造任务事件主题(None, "unsafe/account").is_err());
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct 发件箱任务 {
    #[serde(rename = "event_id")]
    pub(crate) 事件编号: Uuid,
}

#[derive(Clone)]
pub(crate) struct 工作者状态 {
    pub(crate) 数据库: PgPool,
    pub(crate) 消息代理: 消息发布器,
}

pub(crate) type 待确认发布 = (Uuid, oneshot::Sender<()>);
pub(crate) type 待发布队列 = Arc<Mutex<VecDeque<待确认发布>>>;
pub(crate) type 发布确认表 = Arc<Mutex<HashMap<u16, 待确认发布>>>;

#[derive(Clone)]
pub(crate) struct 消息发布器 {
    pub(crate) 客户端: AsyncClient,
    pub(crate) 待发送队列: 待发布队列,
    pub(crate) 发布锁: Arc<tokio::sync::Mutex<()>>,
}

#[cfg(test)]
mod 响应字段测试 {
    use super::*;
    use serde_json::Value;

    fn 校验字段(数据: &Value, 预期字段: &[&str]) {
        let 对象 = 数据.as_object().expect("序列化结果应为对象");
        let mut 实际字段: Vec<_> = 对象.keys().map(String::as_str).collect();
        let mut 预期字段 = 预期字段.to_vec();
        实际字段.sort_unstable();
        预期字段.sort_unstable();
        assert_eq!(实际字段, 预期字段);
    }

    #[test]
    fn 所有接口响应均使用中文业务字段名() {
        let 时间 = DateTime::<Utc>::from_timestamp(0, 0).expect("有效时间戳");
        let 用户 = 用户视图 {
            用户编号: Uuid::nil(),
            登录名: "测试用户".into(),
            昵称: "测试昵称".into(),
        };
        let 认证数据 = serde_json::to_value(认证响应 {
            访问令牌: "token".into(),
            令牌类型: "Bearer",
            有效秒数: 7200,
            用户,
        })
        .expect("认证响应应可序列化");
        校验字段(&认证数据, &["访问令牌", "令牌类型", "有效秒数", "用户"]);
        校验字段(&认证数据["用户"], &["用户编号", "登录名", "昵称"]);

        let 任务数据 = serde_json::to_value(任务视图 {
            任务编号: Uuid::nil(),
            发布者编号: Uuid::nil(),
            接单者编号: None,
            标题: "标题".into(),
            描述: "描述".into(),
            任务状态: "待接单".into(),
            截止时间: None,
            创建时间: 时间,
            版本号: 1,
            位置: None,
            报酬积分: Some(100),
        })
        .expect("任务响应应可序列化");
        校验字段(
            &任务数据,
            &[
                "任务编号",
                "发布者编号",
                "接单者编号",
                "标题",
                "描述",
                "任务状态",
                "截止时间",
                "创建时间",
                "版本号",
                "位置",
                "报酬积分",
            ],
        );

        let 实时数据 = serde_json::to_value(实时连接信息 {
            消息地址: "wss://example.invalid/mqtt".into(),
            消息令牌: "token".into(),
            主题列表: Vec::new(),
            有效秒数: 60,
            说明: "说明",
        })
        .expect("实时凭据响应应可序列化");
        校验字段(
            &实时数据,
            &["消息地址", "消息令牌", "主题列表", "有效秒数", "说明"],
        );

        let 错误数据 = serde_json::to_value(接口错误 {
            消息: "错误".into(),
        })
        .expect("错误响应应可序列化");
        校验字段(&错误数据, &["消息"]);
    }
}
