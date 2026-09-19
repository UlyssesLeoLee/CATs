//! `file-service` 入口

use actix_web::{web, App, HttpServer};
use cats_common::init_tracing;
use file_service::{db, handlers, AppState};
use std::env;
use tracing::info;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_tracing();

    if env::var("DATABASE_URL").is_err() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "DATABASE_URL not set"));
    }

    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8085".to_string());

    let pool = match db::build_pool().await {
        Ok(p) => p,
        Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
    };
    if let Err(e) = db::run_migrations(&pool).await {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
    }
    if let Err(e) = file_service::storage::ensure_base_dir().await {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
    }

    let state = AppState::new(pool);
    let state_data = web::Data::new(state);

    info!(bind_addr = %bind_addr, "starting file-service");

    HttpServer::new(move || {
        App::new()
            .app_data(state_data.clone())
            .route("/healthz", web::get().to(handlers::healthz))
            .route("/readyz", web::get().to(handlers::readyz))
            .route("/v1/files", web::post().to(handlers::upload_file))
            .route("/v1/files", web::get().to(handlers::list_files))
            .route(
                "/v1/files/{id}/content",
                web::get().to(handlers::get_file_content),
            )
    })
    .bind(&bind_addr)?
    .run()
    .await
}