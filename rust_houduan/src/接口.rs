#[path = "接口/任务.rs"]
mod 任务;
#[path = "接口/实时.rs"]
mod 实时;
#[path = "接口/积分.rs"]
pub(crate) mod 积分;
#[path = "接口/管理.rs"]
mod 管理;
#[path = "接口/聊天.rs"]
pub(crate) mod 聊天;
#[path = "接口/认证.rs"]
mod 认证;

use anyhow::{Context, Result};
use axum::{
    Json, Router,
    body::Body,
    extract::{OriginalUri, Path, State},
    http::{HeaderMap, header},
    middleware,
    response::Response,
    routing::{get, post},
};
use serde::Serialize;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

use crate::{共享::*, 配置::运行配置};

pub(crate) async fn 启动(状态: 应用状态, 配置: 运行配置) -> Result<()> {
    let 跨域来源: axum::http::HeaderValue =
        配置.跨域来源.parse().context("CORS_ORIGIN 格式无效")?;
    let 应用 = Router::new()
        .route("/health", get(健康检查))
        .route("/_AMapService/{*path}", get(代理高德地图服务))
        .route("/api/v1/auth/register", post(认证::注册用户))
        .route("/api/v1/auth/login", post(认证::登录用户))
        .route(
            "/api/v1/tasks",
            get(任务::查询任务列表).post(任务::发布任务),
        )
        .route("/api/v1/my/tasks", get(任务::查询我的已接任务))
        .route("/api/v1/my/conversations", get(聊天::查询我的聊天会话))
        .route(
            "/api/v1/tasks/{id}",
            get(任务::查询委托).put(任务::修改任务),
        )
        .route("/api/v1/weituo/{id}", get(任务::查询委托))
        .route("/api/v1/tasks/{id}/claim", post(任务::接取任务))
        .route("/api/v1/tasks/{id}/start", post(任务::开始任务))
        .route("/api/v1/tasks/{id}/complete", post(任务::完成任务))
        .route(
            "/api/v1/tasks/{id}/accept-completion",
            post(任务::验收完成任务),
        )
        .route("/api/v1/tasks/{id}/cancel", post(任务::取消任务))
        .route("/api/v1/tasks/{id}/release", post(任务::释放任务))
        .route("/api/v1/my/points", get(积分::查询我的积分账户))
        .route("/api/v1/my/points/ledger", get(积分::查询我的积分流水))
        .route(
            "/api/v1/admin/users/{id}/points",
            post(积分::管理员发放积分),
        )
        .route("/api/v1/tasks/{id}/messages", get(聊天::查询聊天消息))
        .route("/api/v1/realtime/credentials", post(实时::获取实时凭据))
        .route("/api/v1/admin/users/{id}/disable", post(管理::禁用用户))
        .route("/api/v1/admin/users/{id}/enable", post(管理::启用用户))
        .layer(middleware::from_fn_with_state(
            状态.clone(),
            认证::验证可选身份,
        ))
        .layer(
            CorsLayer::new()
                .allow_origin(跨域来源)
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::PUT,
                    axum::http::Method::OPTIONS,
                ])
                .allow_headers([
                    header::AUTHORIZATION,
                    header::CONTENT_TYPE,
                    header::HeaderName::from_static("idempotency-key"),
                ]),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(状态);

    let 监听地址 = 配置.接口监听地址;
    let 监听器 = tokio::net::TcpListener::bind(监听地址)
        .await
        .with_context(|| {
            format!("监听地址 {监听地址} 失败；若端口已占用，请停止旧 API 进程或修改 API_BIND")
        })?;
    info!(%监听地址, "API 已启动");
    axum::serve(监听器, 应用)
        .await
        .context("API 服务异常退出")?;
    Ok(())
}

#[derive(Serialize)]
struct 健康响应 {
    状态: &'static str,
}

async fn 健康检查(
    State(状态): State<应用状态>
) -> Result<Json<健康响应>, 应用错误> {
    sqlx::query("SELECT 1").execute(&状态.数据库).await?;
    Ok(Json(健康响应 { 状态: "ok" }))
}

pub(super) fn 提取幂等键(请求头: &HeaderMap) -> Result<String, 应用错误> {
    let 值 = 请求头
        .get("idempotency-key")
        .ok_or_else(|| 应用错误::请求错误("必须提供 Idempotency-Key 请求头".into()))?
        .to_str()
        .map_err(|_| 应用错误::请求错误("Idempotency-Key 格式无效".into()))?
        .trim();
    if 值.is_empty() || 值.len() > 100 {
        return Err(应用错误::请求错误(
            "Idempotency-Key 长度需为 1-100 字节".into(),
        ));
    }
    Ok(值.to_owned())
}

async fn 代理高德地图服务(
    State(状态): State<应用状态>,
    Path(路径): Path<String>,
    请求头: HeaderMap,
    OriginalUri(请求地址): OriginalUri,
) -> Result<Response, 应用错误> {
    if !有效的高德服务路径(&路径) {
        return Err(应用错误::请求错误("高德地图服务路径无效".into()));
    }
    let 安全密钥 = 状态
        .高德安全密钥
        .as_deref()
        .ok_or_else(|| 应用错误::服务不可用("地图服务尚未配置".into()))?;
    let 主机 = if 路径 == "v4/map/styles" || 路径.starts_with("v4/map/styles/") {
        "webapi.amap.com"
    } else {
        "restapi.amap.com"
    };
    let mut 上游地址 = reqwest::Url::parse(&format!("https://{主机}/{路径}"))
        .map_err(|_| 应用错误::内部错误(anyhow::anyhow!("高德地图代理地址无效")))?;
    if let Some(查询) = 请求地址.query() {
        上游地址.set_query(Some(查询));
    }
    let 查询参数: Vec<_> = 上游地址
        .query_pairs()
        .filter(|(名称, _)| 名称 != "jscode")
        .map(|(名称, 值)| (名称.into_owned(), 值.into_owned()))
        .collect();
    上游地址.set_query(None);
    {
        let mut 参数 = 上游地址.query_pairs_mut();
        for (名称, 值) in 查询参数 {
            参数.append_pair(&名称, &值);
        }
        参数.append_pair("jscode", 安全密钥);
    }

    let mut 上游请求 = 状态.高德客户端.get(上游地址);
    if let Some(来源页) = 请求头.get(header::REFERER) {
        上游请求 = 上游请求.header(header::REFERER, 来源页);
    }
    let 上游响应 = 上游请求
        .send()
        .await
        .map_err(|_| 应用错误::内部错误(anyhow::anyhow!("高德地图上游请求失败")))?;
    let 状态码 = axum::http::StatusCode::from_u16(上游响应.status().as_u16())
        .map_err(|_| 应用错误::内部错误(anyhow::anyhow!("高德地图响应状态无效")))?;
    let 内容类型 = 上游响应.headers().get(header::CONTENT_TYPE).cloned();
    let 内容 = 上游响应
        .bytes()
        .await
        .map_err(|_| 应用错误::内部错误(anyhow::anyhow!("读取高德地图响应失败")))?;
    let mut 响应 = Response::builder().status(状态码);
    if let Some(内容类型) = 内容类型 {
        响应 = 响应.header(header::CONTENT_TYPE, 内容类型);
    }
    响应
        .body(Body::from(内容))
        .map_err(|_| 应用错误::内部错误(anyhow::anyhow!("构造高德地图响应失败")))
}

fn 有效的高德服务路径(路径: &str) -> bool {
    let 格式有效 = !路径.is_empty()
        && 路径
            .bytes()
            .all(|字节| 字节.is_ascii_alphanumeric() || matches!(字节, b'/' | b'.' | b'_' | b'-'))
        && 路径.split('/').all(|片段| 片段 != "." && 片段 != "..");
    格式有效
        && (路径 == "v3/geocode/geo"
            || 路径 == "v3/geocode/regeo"
            || 路径 == "v4/map/styles"
            || 路径.starts_with("v4/map/styles/"))
}

#[cfg(test)]
mod 高德代理路径测试 {
    use super::有效的高德服务路径;

    #[test]
    fn 代理只接受固定主机可处理的安全路径片段() {
        assert!(有效的高德服务路径("v3/geocode/regeo"));
        assert!(有效的高德服务路径("v3/geocode/geo"));
        assert!(有效的高德服务路径("v4/map/styles/abc"));
        assert!(!有效的高德服务路径("../internal"));
        assert!(!有效的高德服务路径("v3/%2e%2e/internal"));
        assert!(!有效的高德服务路径("v3/anything"));
        assert!(!有效的高德服务路径("https://example.com"));
    }
}
