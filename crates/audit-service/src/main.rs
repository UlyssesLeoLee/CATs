//! `audit-service` 入口

use actix_web::{web, App, HttpServer};
use audit_service::{consumer, db, handlers, AppState};
use cats_common::init_tracing;
use std::env;
use tracing::info;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_tracing();

    if env::var("DATABASE_URL").is_err() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "DATABASE_URL not set",
        ));
    }

    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8088".to_string());

    let pool = match db::build_pool().await {
        Ok(p) => p,
        Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
    };
    if let Err(e) = db::run_migrations(&pool).await {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
    }

    // 启动 Kafka consumer stub (per §4.2)
    let consumer_pool = pool.clone();
    let topic = env::var("KAFKA_AUDIT_TOPIC").unwrap_or_else(|_| "cats.audit.v1".to_string());
    tokio::spawn(async move {
        consumer::run_consumer_loop(consumer_pool, topic).await;
    });

    let state = AppState::new(pool);
    let state_data = web::Data::new(state);

    info!(bind_addr = %bind_addr, "starting audit-service");

    HttpServer::new(move || {
        App::new()
            .app_data(state_data.clone())
            .route("/healthz", web::get().to(handlers::healthz))
            .route("/readyz", web::get().to(handlers::readyz))
            .route(
                "/v1/audit-logs",
                web::get().to(handlers::list_audit_logs),
            )
            .route(
                "/v1/audit-logs/test",
                web::post().to(handlers::test_ingest),
            )
    })
    .bind(&bind_addr)?
    .run()
    .await
}