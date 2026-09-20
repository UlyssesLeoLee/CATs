//! `notification-service` 入口
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: ULYS-152 切片 B-4

use actix_web::{web, App, HttpServer};
use notification_service::EventBus;
use std::env;
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
    HttpServer::new(move || {
        App::new()
            .app_data(pool_data.clone())
            .app_data(bus_data.clone())
            .route(
                "/healthz",
                web::get().to(notification_service::handlers::healthz),
            )
            .route(
                "/v1/notifications",
                web::get().to(notification_service::handlers::list_notifications),
            )
            .route(
                "/v1/notifications",
                web::post().to(notification_service::handlers::create_notification),
            )
            .route(
                "/v1/notifications/{id}/read",
                web::patch().to(notification_service::handlers::mark_notification_read),
            )
            .route(
                "/v1/notifications/ws",
                web::get().to(notification_service::handlers::notification_stream),
            )
    })
    .bind(&bind_addr)?
    .run()
    .await
}