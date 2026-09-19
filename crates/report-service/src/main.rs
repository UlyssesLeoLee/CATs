//! `report-service` 入口

use actix_web::{web, App, HttpServer};
use cats_common::init_tracing;
use report_service::{db, handlers, AppState};
use std::env;
use tracing::info;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_tracing();

    if env::var("DATABASE_URL").is_err() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "DATABASE_URL not set"));
    }

    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8087".to_string());

    let report = match db::build_pool().await {
        Ok(p) => p,
        Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
    };
    if let Err(e) = db::run_migrations(&report).await {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
    }
    let task_ro = match db::build_task_ro_pool().await {
        Ok(p) => p,
        Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
    };
    if task_ro.is_none() {
        tracing::warn!("TASK_DATABASE_URL not set; /v1/reports/* will fail");
    }

    let state = AppState::new(db::Pools { report, task_ro });
    let state_data = web::Data::new(state);

    info!(bind_addr = %bind_addr, "starting report-service");

    HttpServer::new(move || {
        App::new()
            .app_data(state_data.clone())
            .route("/healthz", web::get().to(handlers::healthz))
            .route("/readyz", web::get().to(handlers::readyz))
            .route(
                "/v1/reports/usage",
                web::get().to(handlers::get_usage_report),
            )
            .route(
                "/v1/reports/quality",
                web::get().to(handlers::get_quality_report),
            )
    })
    .bind(&bind_addr)?
    .run()
    .await
}