//! `file-service` 入口
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: ULYS-152 切片 B-3

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

    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8085".to_string());

    // 构造连接池 (lazy, 不实际连 DB)
    let pool = match file_service::db::build_pool().await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ERROR: db pool build failed: {e}");
            std::process::exit(1);
        }
    };

    info!(bind_addr = %bind_addr, "starting file-service");

    let pool_data = web::Data::new(pool);
    HttpServer::new(move || {
        App::new()
            .app_data(pool_data.clone())
            .route("/healthz", web::get().to(file_service::handlers::healthz))
            .route(
                "/v1/files",
                web::post().to(file_service::handlers::upload_file),
            )
            .route(
                "/v1/files",
                web::get().to(file_service::handlers::list_files),
            )
            .route(
                "/v1/files/{id}",
                web::get().to(file_service::handlers::download_file),
            )
            .route(
                "/v1/files/{id}",
                web::delete().to(file_service::handlers::delete_file),
            )
            .route(
                "/v1/files/{id}/metadata",
                web::get().to(file_service::handlers::get_file_metadata),
            )
    })
    .bind(&bind_addr)?
    .run()
    .await
}