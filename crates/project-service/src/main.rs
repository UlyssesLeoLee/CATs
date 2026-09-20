//! `project-service` 入口
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1 (project-service)
//! 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.2
//! 引用: ULYS-150 切片 B-1 §"5 endpoint"

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

    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8083".to_string());

    // 构造连接池 (lazy, 不实际连 DB)
    let pool = match project_service::db::build_pool().await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ERROR: db pool build failed: {e}");
            std::process::exit(1);
        }
    };

    info!(bind_addr = %bind_addr, "starting project-service");

    let pool_data = web::Data::new(pool);
    HttpServer::new(move || {
        App::new()
            .app_data(pool_data.clone())
            .route(
                "/healthz",
                web::get().to(project_service::handlers::healthz),
            )
            // POST   /v1/projects                  — 创建
            .route(
                "/v1/projects",
                web::post().to(project_service::handlers::create_project),
            )
            // GET    /v1/projects                  — 列表 (workspace_id 分页过滤)
            .route(
                "/v1/projects",
                web::get().to(project_service::handlers::list_projects),
            )
            // GET    /v1/projects/{id}             — 详情
            .route(
                "/v1/projects/{id}",
                web::get().to(project_service::handlers::get_project),
            )
            // PATCH  /v1/projects/{id}             — 部分更新
            .route(
                "/v1/projects/{id}",
                web::patch().to(project_service::handlers::patch_project),
            )
            // DELETE /v1/projects/{id}             — 软删除
            .route(
                "/v1/projects/{id}",
                web::delete().to(project_service::handlers::delete_project),
            )
    })
    .bind(&bind_addr)?
    .run()
    .await
}