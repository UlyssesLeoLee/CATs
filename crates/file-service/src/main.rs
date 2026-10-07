//! `file-service` 入口
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: ULYS-152 切片 B-3

use actix_web::{web, App, HttpServer};
use cats_rbac::RbacChecker;
use std::env;
use std::sync::Arc;
use tracing::info;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    cats_common::init_tracing();

    // DATABASE_URL 必须设置, 否则 fail-fast (per 安全约束: 不打印值)
    if env::var("DATABASE_URL").is_err() {
        eprintln!("ERROR: DATABASE_URL env var not set");
        std::process::exit(1);
    }

    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8085".to_string());

    // 构造连接池 (lazy, 不实际连 DB)
    let pool = match file_service::db::build_pool().await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ERROR: db pool build failed: {e}");
            std::process::exit(1);
        }
    };

    // 共享 RbacChecker (per ULYS-152 切片 B-3 §"RBAC 中间件挂在每个 endpoint")
    let rbac_checker = Arc::new(RbacChecker::new());

    info!(bind_addr = %bind_addr, "starting file-service");

    let pool_data = web::Data::new(pool);
    let rbac_data = web::Data::new(rbac_checker);
    HttpServer::new(move || {
        // 路由表只有这一份 —— main.rs 与集成测试共用
        // `handlers::configure_routes`（per BACKEND_STATUS §4.1t）。
        // 先 clone 再 move：这个闭包是 `Fn`。
        let pool = pool_data.clone();
        let rbac = rbac_data.clone();
        App::new().configure(move |c| {
            file_service::handlers::configure_routes(c, pool.clone(), rbac.clone())
        })
    })
    .bind(&bind_addr)?
    .run()
    .await
}
