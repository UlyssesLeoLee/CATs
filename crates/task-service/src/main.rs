//! `task-service` 入口

use actix_web::{web, App, HttpServer};
use cats_common::init_tracing;
use std::env;
use task_service::{db, handlers, AppState};
use tracing::info;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_tracing();

    if env::var("DATABASE_URL").is_err() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "DATABASE_URL not set"));
    }

    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8084".to_string());

    let pool = match db::build_pool().await {
        Ok(p) => p,
        Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
    };
    if let Err(e) = db::run_migrations(&pool).await {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
    }

    let state = AppState::new(pool);
    let state_data = web::Data::new(state);

    info!(bind_addr = %bind_addr, "starting task-service");

    HttpServer::new(move || {
        App::new()
            .app_data(state_data.clone())
            .route("/healthz", web::get().to(handlers::healthz))
            .route("/readyz", web::get().to(handlers::readyz))
            .route("/v1/tasks", web::post().to(handlers::create_task))
            .route("/v1/tasks", web::get().to(handlers::list_tasks))
            .route("/v1/tasks/{id}", web::get().to(handlers::get_task))
            .route(
                "/v1/tasks/{id}/status",
                web::patch().to(handlers::update_task_status),
            )
    })
    .bind(&bind_addr)?
    .run()
    .await
}