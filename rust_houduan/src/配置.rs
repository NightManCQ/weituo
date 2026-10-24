use std::{env, net::SocketAddr};

use anyhow::{Context, Result};

#[derive(Clone)]
pub(crate) struct 运行配置 {
    pub(crate) 数据库地址: String,
    pub(crate) 令牌密钥: String,
    pub(crate) 接口监听地址: SocketAddr,
    pub(crate) 跨域来源: String,
    pub(crate) 运行模式: String,
    pub(crate) 日志过滤器: String,
    pub(crate) 高德安全密钥: Option<String>,
}

impl 运行配置 {
    pub(crate) fn 读取() -> Result<Self> {
        dotenvy::dotenv().ok();

        let 令牌密钥 = env::var("JWT_SECRET").context("必须配置 JWT_SECRET")?;
        anyhow::ensure!(令牌密钥.len() >= 32, "JWT_SECRET 至少需要 32 字节");

        let 接口监听地址 = env::var("API_BIND")
            .unwrap_or_else(|_| "127.0.0.1:8080".to_owned())
            .parse()
            .context("API_BIND 格式无效")?;

        let 运行模式 = env::args().nth(1).unwrap_or_else(|| "all".to_owned());
        anyhow::ensure!(
            matches!(运行模式.as_str(), "all" | "api" | "worker"),
            "运行模式无效: {运行模式}；用法: renwu-houduan [all|api|worker]"
        );

        Ok(Self {
            数据库地址: env::var("DATABASE_URL").context("必须配置 DATABASE_URL")?,
            令牌密钥,
            接口监听地址,
            跨域来源: env::var("CORS_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:5173".to_owned()),
            运行模式,
            日志过滤器: env::var("RUST_LOG")
                .unwrap_or_else(|_| "renwu_houduan=info,tower_http=info".to_owned()),
            高德安全密钥: env::var("AMAP_SECURITY_CODE")
                .ok()
                .filter(|值| !值.trim().is_empty()),
        })
    }
}
