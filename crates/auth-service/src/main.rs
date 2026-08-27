//! `auth-service` 入口
//!
//! M0 阶段：actix-web 4 健康检查占位，端口与绑定地址走环境变量。
//! 业务 endpoint 在 M1 阶段按微服务架构书 §4.1 + OpenAPI v1 落地。

use actix_web::{web, App, HttpResponse, HttpServer};
use cats_common::AppMeta;
use serde::Serialize;
use std::env;
use tracing::info;

/// 业务配置（M0 占位：从 env 读取；M1 替换为结构化 config）
#[derive(Debug, Clone)]
struct Config {
    bind_addr: String,
}

impl Config {
    fn from_env() -> Self {
        Self {
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8081".to_string()),
        }
    }
}

/// 健康检查响应
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    app: AppMeta,
}

/// `GET /healthz` — 存活探针 + 启动探针复用
async fn healthz() -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok",
        app: AppMeta::current(),
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    cats_common::init_tracing();
    let cfg = Config::from_env();
    info!(bind_addr = %cfg.bind_addr, "starting auth-service");

    HttpServer::new(|| App::new().route("/healthz", web::get().to(healthz)))
        .bind(&cfg.bind_addr)?
        .run()
        .await
}
