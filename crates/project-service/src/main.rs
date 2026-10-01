//! `project-service` 入口
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1 (project-service)
//! 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.2
//! 引用: ULYS-150 切片 B-1 §"5 endpoint"
//! 引用: ULYS-150 复审 §"RBAC 中间件挂在每个 endpoint" (per handler inline 调用 rbac::enforce)

use actix_web::{web, App, HttpServer};
use cats_rbac::RbacChecker;
use std::env;
use std::sync::Arc;
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

    // 共享 RbacChecker (per 切片 B-1 §RBAC 中间件挂在每个 endpoint)
    let rbac_checker = Arc::new(RbacChecker::new());

    info!(bind_addr = %bind_addr, "starting project-service");

    let pool_data = web::Data::new(pool);
    let rbac_data = web::Data::new(rbac_checker);
    HttpServer::new(move || {
        App::new()
            .app_data(pool_data.clone())
            .app_data(rbac_data.clone())
            .route(
                "/healthz",
                web::get().to(project_service::handlers::healthz),
            )
            // POST   /v1/projects                  — 创建 (RBAC: Project Create)
            .route(
                "/v1/projects",
                web::post().to(project_service::handlers::create_project),
            )
            // GET    /v1/projects                  — 列表 (RBAC: Project Read)
            .route(
                "/v1/projects",
                web::get().to(project_service::handlers::list_projects),
            )
            // GET    /v1/projects/{id}             — 详情 (RBAC: Project Read)
            .route(
                "/v1/projects/{id}",
                web::get().to(project_service::handlers::get_project),
            )
            // PATCH  /v1/projects/{id}             — 部分更新 (RBAC: Project Update)
            .route(
                "/v1/projects/{id}",
                web::patch().to(project_service::handlers::patch_project),
            )
            // DELETE /v1/projects/{id}             — 软删除 (RBAC: Project Delete)
            .route(
                "/v1/projects/{id}",
                web::delete().to(project_service::handlers::delete_project),
            )
    })
    .bind(&bind_addr)?
    .run()
    .await
}