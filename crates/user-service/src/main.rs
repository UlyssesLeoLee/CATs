//! `user-service` 入口
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0+1.md §2 T-02

use actix_web::{web, App, HttpServer};
use std::env;
use tracing::info;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    cats_common::init_tracing();

    // DATABASE_URL 必须设置, 否则 fail-fast (per 安全约束: 不打印值)
    if env::var("DATABASE_URL").is_err() {
        eprintln!("ERROR: DATABASE_URL env var not set");
        std::process::exit(1);
    }

    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8082".to_string());

    // 构造连接池 (lazy, 不实际连 DB)
    let pool = match user_service::db::build_pool().await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ERROR: db pool build failed: {e}");
            std::process::exit(1);
        }
    };

    info!(bind_addr = %bind_addr, "starting user-service");

    let pool_data = web::Data::new(pool);
    HttpServer::new(move || {
        // 路由表只有这一份 —— main.rs 与集成测试共用
        // `handlers::configure_routes`（per BACKEND_STATUS §4.1t）。
        // 先 clone 再 move：这个闭包是 `Fn`，不能把 `pool_data` 直接 move 进去。
        let pool = pool_data.clone();
        App::new().configure(move |c| user_service::handlers::configure_routes(c, pool.clone()))
    })
    .bind(&bind_addr)?
    .run()
    .await
}
