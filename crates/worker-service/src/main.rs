//! `worker-service` 入口
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//!
//! M1 阶段：真实启动 — 构造 PgPool + 后台调度循环 + 3 个 endpoint。
//! 业务配置:
//! - `BIND_ADDR` — 默认 `0.0.0.0:8089`
//! - `DATABASE_URL` — task_db 连接 URL, **必填** (per §5.2 注入规范; 缺则 fail-fast)

use actix_web::{web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::time::Duration;
use tracing::info;
use worker_service::{handlers, scheduler, state::AppState};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    cats_common::init_tracing();

    // DATABASE_URL 必须设置 (per 安全约束: 不打印值)
    if env::var("DATABASE_URL").is_err() {
        eprintln!("ERROR: DATABASE_URL env var not set");
        std::process::exit(1);
    }

    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8089".to_string());
    info!(bind_addr = %bind_addr, "starting worker-service");

    // 构造连接池 (lazy, 不实际连 DB — 与 file-service / task-service 同模式)
    let url = match env::var("DATABASE_URL") {
        Ok(u) => u,
        Err(_) => {
            eprintln!("ERROR: DATABASE_URL env var not set");
            std::process::exit(1);
        }
    };
    let pool = match PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(3))
        .connect_lazy(&url)
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ERROR: db pool build failed: {e}");
            std::process::exit(1);
        }
    };

    // 后台调度循环: 每 30s 抢一批 pending → in_progress → completed/qa_blocked
    let scheduler_pool = pool.clone();
    tokio::spawn(async move {
        scheduler::run_scheduler_loop(scheduler_pool).await;
    });

    let app_state = web::Data::new(AppState::new(pool.clone()));
    let pool_data = web::Data::new(pool);

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .app_data(pool_data.clone())
            .route("/healthz", web::get().to(handlers::healthz))
            .route("/readyz", web::get().to(handlers::readyz))
            .route(
                "/v1/worker/tick",
                web::post().to(handlers::manual_tick),
            )
    })
    .bind(&bind_addr)?
    .run()
    .await
}
