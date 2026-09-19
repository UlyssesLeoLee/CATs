//! `notification-service` 入口

use actix_web::{web, App, HttpServer};
use cats_common::init_tracing;
use notification_service::{consumer, db, handlers, AppState};
use std::env;
use tracing::info;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_tracing();

    if env::var("DATABASE_URL").is_err() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "DATABASE_URL not set"));
    }

    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8086".to_string());

    let pool = match db::build_pool().await {
        Ok(p) => p,
        Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
    };
    if let Err(e) = db::run_migrations(&pool).await {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
    }

    // 启动 Kafka consumer stub (per §4.2)
    let consumer_pool = pool.clone();
    let topic = env::var("KAFKA_NOTIFICATIONS_TOPIC")
        .unwrap_or_else(|_| "cats.notifications.v1".to_string());
    tokio::spawn(async move {
        consumer::run_consumer_loop(consumer_pool, topic).await;
    });

    let state = AppState::new(pool);
    let state_data = web::Data::new(state);

    info!(bind_addr = %bind_addr, "starting notification-service");

    HttpServer::new(move || {
        App::new()
            .app_data(state_data.clone())
            .route("/healthz", web::get().to(handlers::healthz))
            .route("/readyz", web::get().to(handlers::readyz))
            .route(
                "/v1/notifications",
                web::get().to(handlers::list_my_notifications),
            )
            .route(
                "/v1/notifications/test",
                web::post().to(handlers::test_publish),
            )
    })
    .bind(&bind_addr)?
    .run()
    .await
}