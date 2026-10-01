//! `audit-service` 入口
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1
//! 引用: ULYS-153 切片 C-2 — 真实 Kafka consumer (REST proxy 拉取)
//!
//! M0 阶段：actix-web 4 健康检查占位，端口与绑定地址走环境变量。
//! M1 阶段 (per ULYS-153 切片 C-2): spawn `run_consumer_loop` 订阅 `cats.audit.v1`.

use actix_web::{web, App, HttpResponse, HttpServer};
use cats_common::AppMeta;
use serde::Serialize;
use sqlx::postgres::PgPoolOptions;
use std::env;
use tracing::{error, info};

/// 业务配置 (从 env 读取)
#[derive(Debug, Clone)]
struct Config {
    bind_addr: String,
    kafka_topic: String,
}

impl Config {
    fn from_env() -> Self {
        Self {
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8088".to_string()),
            kafka_topic: env::var("KAFKA_AUDIT_TOPIC")
                .unwrap_or_else(|_| "cats.audit.v1".to_string()),
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
    info!(bind_addr = %cfg.bind_addr, "starting audit-service");

    // 构造 audit_db 连接池 (lazy connect, 不阻塞启动)
    // 若 DATABASE_URL 未设, 仍启动 HTTP 但 consumer 不能落档
    let pool = match env::var("DATABASE_URL").ok() {
        Some(url) => match PgPoolOptions::new()
            .max_connections(10)
            .connect_lazy(&url)
        {
            Ok(p) => Some(p),
            Err(e) => {
                error!(error = %e, "audit_db pool build failed; consumer disabled, HTTP only");
                None
            }
        },
        None => {
            error!("DATABASE_URL not set; consumer disabled, HTTP only");
            None
        }
    };

    // 启动 Kafka consumer (per ULYS-153 切片 C-2)
    if let Some(pool) = pool.clone() {
        let topic = cfg.kafka_topic.clone();
        tokio::spawn(async move {
            audit_service::run_consumer_loop(pool, topic).await;
        });
        info!(topic = %cfg.kafka_topic, "audit consumer task spawned");
    }

    let bind_addr = cfg.bind_addr.clone();
    HttpServer::new(move || App::new().route("/healthz", web::get().to(healthz)))
        .bind(&bind_addr)?
        .run()
        .await
}
