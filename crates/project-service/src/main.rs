//! `project-service` 入口
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md §6.3

use actix_web::{web, App, HttpServer};
use cats_common::init_tracing;
use project_service::{db, handlers, AppState};
use std::env;
use tracing::info;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_tracing();

    if env::var("DATABASE_URL").is_err() {
        eprintln!("ERROR: DATABASE_URL env var not set");
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "DATABASE_URL not set"));
    }

    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8083".to_string());

    let pool = match db::build_pool().await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ERROR: db pool build failed: {e}");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
        }
    };
    if let Err(e) = db::run_migrations(&pool).await {
        eprintln!("ERROR: migration failed: {e}");
        return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
    }

    let state = AppState::new(pool);
    let state_data = web::Data::new(state);

    info!(bind_addr = %bind_addr, "starting project-service");

    HttpServer::new(move || {
        App::new()
            .app_data(state_data.clone())
            .route("/healthz", web::get().to(handlers::healthz))
            .route("/readyz", web::get().to(handlers::readyz))
            .route("/v1/projects", web::post().to(handlers::create_project))
            .route("/v1/projects", web::get().to(handlers::list_projects))
            .route("/v1/projects/{id}", web::get().to(handlers::get_project))
            .route("/v1/projects/{id}", web::put().to(handlers::update_project))
            .route(
                "/v1/projects/{id}/members",
                web::post().to(handlers::add_member),
            )
            .route(
                "/v1/projects/{id}/members",
                web::get().to(handlers::list_members),
            )
    })
    .bind(&bind_addr)?
    .run()
    .await
}