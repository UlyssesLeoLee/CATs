//! `report-service` 入口 — per ULYS-153 切片 C-1
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1
//!
//! M0 阶段：actix-web 4 健康检查占位。
//! M1 阶段 (per ULYS-153 切片 C-1): 3 报表 endpoint + report_db 跨表聚合

use actix_web::{web, App, HttpServer};
use std::env;
use tracing::{error, info};

/// 业务配置 (从 env 读取)
#[derive(Debug, Clone)]
struct Config {
    bind_addr: String,
}

impl Config {
    fn from_env() -> Self {
        Self {
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8087".to_string()),
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    cats_common::init_tracing();
    let cfg = Config::from_env();
    info!(bind_addr = %cfg.bind_addr, "starting report-service");

    // 构造 report_db 连接池 (lazy connect)
    // 若 DATABASE_URL 未设, 仍启动 HTTP 但 endpoint 会返回 500
    let pool = match env::var("DATABASE_URL").ok() {
        Some(url) => match sqlx::postgres::PgPoolOptions::new()
            .max_connections(10)
            .connect_lazy(&url)
        {
            Ok(p) => p,
            Err(e) => {
                error!(error = %e, "report_db pool build failed; endpoints will return 500");
                // 仍用 lazy connect 给一个空 pool, 保证启动不挂
                sqlx::postgres::PgPoolOptions::new()
                    .max_connections(1)
                    .connect_lazy("postgres://invalid/invalid")
                    .expect("fallback lazy pool build")
            }
        },
        None => {
            error!("DATABASE_URL not set; endpoints will return 500");
            sqlx::postgres::PgPoolOptions::new()
                .max_connections(1)
                .connect_lazy("postgres://invalid/invalid")
                .expect("fallback lazy pool build")
        }
    };

    let pool_data = web::Data::new(pool);
    let bind_addr = cfg.bind_addr.clone();
    HttpServer::new(move || {
        App::new()
            .app_data(pool_data.clone())
            .route("/healthz", web::get().to(report_service::handlers::healthz))
            .route(
                "/v1/reports/usage",
                web::get().to(report_service::handlers::usage_report),
            )
            .route(
                "/v1/reports/translation-volume",
                web::get().to(report_service::handlers::translation_volume),
            )
            .route(
                "/v1/reports/audit-summary",
                web::get().to(report_service::handlers::audit_summary),
            )
    })
    .bind(&bind_addr)?
    .run()
    .await
}
