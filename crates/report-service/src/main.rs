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
    // 共享 RbacChecker (per ULYS-153 切片 C-1 §"RBAC 集成")
    let rbac_data = web::Data::new(std::sync::Arc::new(cats_rbac::RbacChecker::new()));
    let bind_addr = cfg.bind_addr.clone();
    HttpServer::new(move || {
        // 路由表 + app_data 都在 handlers::configure_routes 里，与集成测试共用一份。
        // `HttpServer::new` 的闭包是 `Fn`，每个 worker 线程都会再调一次，
        // 所以先把两个 `web::Data` clone 出来再 move 进 configure 的闭包里。
        let pool_data = pool_data.clone();
        let rbac_data = rbac_data.clone();
        App::new().configure(move |c| {
            report_service::handlers::configure_routes(c, pool_data.clone(), rbac_data.clone())
        })
    })
    .bind(&bind_addr)?
    .run()
    .await
}
