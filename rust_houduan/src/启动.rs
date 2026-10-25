#[path = "共享.rs"]
mod 共享;
#[path = "工作者.rs"]
mod 工作者;
#[path = "接口.rs"]
mod 接口;
#[path = "配置.rs"]
mod 配置;
#[path = "领域.rs"]
mod 领域;

use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let 配置 = 配置::运行配置::读取()?;
    tracing_subscriber::fmt()
        .with_env_filter(配置.日志过滤器.clone())
        .init();
    let 数据库 = PgPoolOptions::new()
        .max_connections(10)
        .connect(&配置.数据库地址)
        .await
        .context("连接 PostgreSQL 失败")?;
    sqlx::migrate!("./migrations")
        .run(&数据库)
        .await
        .context("运行数据库迁移失败")?;
    let 高德客户端 = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("初始化高德地图代理客户端失败")?;
    let 消息代理 = 工作者::连接消息代理().await?;
    match 配置.运行模式.as_str() {
        "worker" => 工作者::启动(数据库, 消息代理).await,
        "api" => {
            接口::启动(
                共享::应用状态 {
                    数据库,
                    令牌密钥: 配置.令牌密钥.clone(),
                    消息代理,
                    高德客户端,
                    高德安全密钥: 配置.高德安全密钥.clone(),
                },
                配置,
            )
            .await
        }
        "all" => {
            let 接口状态 = 共享::应用状态 {
                数据库: 数据库.clone(),
                令牌密钥: 配置.令牌密钥.clone(),
                消息代理: 消息代理.clone(),
                高德客户端,
                高德安全密钥: 配置.高德安全密钥.clone(),
            };
            let 接口服务 = 接口::启动(接口状态, 配置.clone());
            let 后台服务 = 工作者::启动(数据库, 消息代理);
            tokio::try_join!(接口服务, 后台服务)?;
            Ok(())
        }
        模式 => anyhow::bail!("运行模式无效: {模式}；用法: renwu-houduan [all|api|worker]"),
    }
}
