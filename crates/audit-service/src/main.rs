//! `audit-service` 入口
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1
//! 引用: ULYS-153 切片 C-2 — 真实 Kafka consumer (REST proxy 拉取)
//!
//! 2026-10-05 接线。此前本文件只注册 `/healthz` 一个路由，而 `db.rs` /
//! `handlers.rs` / `models.rs` / `state.rs` 四个文件因为 `lib.rs` 没有
//! `mod` 声明而从未被编译 —— 也就是说这个服务**从来没有过任何业务端点**。
//!
//! 路由：
//! - `GET  /healthz`             — 存活探针（无 auth）
//! - `GET  /readyz`              — 就绪探针，含 DB 探活
//! - `GET  /v1/audit-logs`       — 按 org 分页列出（RBAC: Audit Read）
//! - `POST /v1/audit-logs/test`  — 手动 ingest 一条（开发/测试用）
//! - 另 spawn `run_consumer_loop` 订阅 `cats.audit.v1`（per ULYS-153 C-2）

use actix_web::{web, App, HttpServer};
use std::env;
use tracing::info;

use audit_service::handlers;
use audit_service::state::AppState;

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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    cats_common::init_tracing();
    let cfg = Config::from_env();
    info!(bind_addr = %cfg.bind_addr, "starting audit-service");

    // 2026-10-05 行为变更：原来 DATABASE_URL 缺失时**照常起 HTTP**，
    // 只有 consumer 被禁用。那在没有 DB 的情况下所有业务端点都必然失败，
    // 容器却显示 Up —— 又一次"看起来正常"。
    //
    // 既然现在有了真端点，缺库就该启动即失败（与 file-service / worker-service
    // 一致），并且**不打印 URL 的值**（仓库安全约束）。
    //
    // 用 `db::build_pool`（connect 而非 connect_lazy）：compose 已保证
    // db-init 先跑完，库连不上就是真故障，应该当场暴露而不是拖到第一个请求。
    let pool = match audit_service::db::build_pool().await {
        Ok(p) => p,
        Err(e) => {
            // 打印错误码而不打印连接串
            eprintln!("ERROR: audit_db unavailable: {}", e);
            return Err(std::io::Error::other("audit_db unavailable"));
        }
    };
    info!("audit_db pool ready");

    // 启动 Kafka consumer (per ULYS-153 切片 C-2)
    {
        let pool = pool.clone();
        let topic = cfg.kafka_topic.clone();
        tokio::spawn(async move {
            audit_service::run_consumer_loop(pool, topic).await;
        });
        info!(topic = %cfg.kafka_topic, "audit consumer task spawned");
    }

    let state = web::Data::new(AppState::new(pool.clone()));
    let pool_data = web::Data::new(pool);
    let bind_addr = cfg.bind_addr.clone();

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .app_data(pool_data.clone())
            // 路由表在 handlers::configure —— 与集成测试共用同一份，
            // 避免"测试验证的路由表"和"服务实际监听的路由表"两份。
            .configure(handlers::configure)
    })
    .bind(&bind_addr)?
    .run()
    .await
}
