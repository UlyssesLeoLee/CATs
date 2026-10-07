//! `cats-bff` 入口
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: api/openapi/cats-openapi-v1.0.1.yaml §paths
//!
//! M1 阶段: 8 业务 endpoint (per 切片 A _slice_a_bff.md §交付清单 1+6)
//! - POST /v1/auth/login
//! - POST /v1/auth/refresh
//! - POST /v1/auth/logout
//! - GET  /v1/auth/me
//! - GET  /v1/projects
//! - POST /v1/projects
//! - POST /v1/tasks
//! - GET  /healthz

use actix_web::{web, App, HttpServer};
use cats_bff::{
    config::Config,
    handlers,
    principal::shared_checker,
    upstream::{auth::AuthClient, projects::ProjectsClient, tasks::TasksClient},
};
use std::sync::Arc;
use tracing::info;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    cats_common::init_tracing();
    let cfg = Config::from_env();
    let bind_addr = cfg.bind_addr.clone();
    info!(
        bind_addr = %bind_addr,
        auth_service_url = %cfg.auth_service_url,
        project_service_url = %cfg.project_service_url,
        task_service_url = %cfg.task_service_url,
        "starting cats-bff"
    );

    let auth_client = AuthClient::new(&cfg);
    let projects_client = ProjectsClient::new(&cfg);
    let tasks_client = TasksClient::new(&cfg);
    let rbac_checker: Arc<_> = shared_checker();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(cfg.clone()))
            .app_data(web::Data::new(auth_client.clone()))
            .app_data(web::Data::new(projects_client.clone()))
            .app_data(web::Data::new(tasks_client.clone()))
            .app_data(web::Data::new(rbac_checker.clone()))
            .app_data(handlers::json_config())
            // ---- 8 endpoint ----
            // 路由表在 handlers::configure_routes 里，与集成测试共用同一张
            // （原先内联在这里，测试只能去挂另一张没人用的表，见该函数注释）
            .configure(handlers::configure_routes)
    })
    .bind(&bind_addr)?
    .run()
    .await
}
