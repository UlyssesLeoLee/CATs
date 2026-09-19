//! `worker-service` 入口

use actix_web::{web, App, HttpServer};
use cats_common::init_tracing;
use std::env;
use tracing::info;
use worker_service::{handlers, scheduler, AppState};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_tracing();

    if env::var("DATABASE_URL").is_err() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "DATABASE_URL not set",
        ));
    }

    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8089".to_string());

    let pool = match sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&env::var("DATABASE_URL").unwrap())
        .await
    {
        Ok(p) => p,
        Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
    };

    // 启动 scheduler loop (后台)
    let sched_pool = pool.clone();
    tokio::spawn(async move {
        scheduler::run_scheduler_loop(sched_pool).await;
    });

    let state = AppState::new(pool);
    let state_data = web::Data::new(state);

    info!(bind_addr = %bind_addr, "starting worker-service");

    HttpServer::new(move || {
        App::new()
            .app_data(state_data.clone())
            .route("/healthz", web::get().to(handlers::healthz))
            .route("/readyz", web::get().to(handlers::readyz))
            .route("/v1/worker/tick", web::post().to(handlers::manual_tick))
    })
    .bind(&bind_addr)?
    .run()
    .await
}