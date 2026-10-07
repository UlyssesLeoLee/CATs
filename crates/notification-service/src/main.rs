//! `notification-service` 入口
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: ULYS-152 切片 B-4

use actix_web::{web, App, HttpServer};
use cats_rbac::RbacChecker;
use notification_service::EventBus;
use std::env;
use std::sync::Arc;
use tracing::info;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    cats_common::init_tracing();

    if env::var("DATABASE_URL").is_err() {
        eprintln!("ERROR: DATABASE_URL env var not set");
        std::process::exit(1);
    }

    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8086".to_string());

    let pool = match notification_service::db::build_pool().await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ERROR: db pool build failed: {e}");
            std::process::exit(1);
        }
    };

    info!(bind_addr = %bind_addr, "starting notification-service");

    let pool_data = web::Data::new(pool);
    let bus_data = web::Data::new(EventBus::new());
    // 共享 RbacChecker (per ULYS-152 切片 B-4 §"RBAC 中间件挂在每个 endpoint")
    let rbac_data = web::Data::new(Arc::new(RbacChecker::new()));
    HttpServer::new(move || {
        // 路由表只有这一份 —— main.rs 与集成测试共用
        // `handlers::configure_routes`（per BACKEND_STATUS §4.1t）。
        // 先 clone 再 move：这个闭包是 `Fn`。
        let pool = pool_data.clone();
        let bus = bus_data.clone();
        let rbac = rbac_data.clone();
        App::new().configure(move |c| {
            notification_service::handlers::configure_routes(
                c,
                pool.clone(),
                bus.clone(),
                rbac.clone(),
            )
        })
    })
    .bind(&bind_addr)?
    .run()
    .await
}
