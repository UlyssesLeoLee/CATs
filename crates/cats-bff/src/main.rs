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
    upstream::{
        auth::AuthClient,
        projects::ProjectsClient,
        tasks::TasksClient,
    },
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
            .app_data(web::JsonConfig::default().error_handler(|err, _req| {
                actix_web::error::InternalError::from_response(
                    err,
                    actix_web::HttpResponse::BadRequest()
                        .json(cats_bff::error::ErrorBody {
                            error: cats_bff::error::ErrorCode::InvalidPayload,
                            message: "invalid request body".to_string(),
                            detail: None,
                        }),
                )
                .into()
            }))
            // ---- 8 endpoint ----
            .route("/healthz", web::get().to(handlers::healthz_handler))
            .service(
                web::scope("/v1/auth")
                    .route("/login", web::post().to(handlers::login))
                    .route("/refresh", web::post().to(handlers::refresh))
                    .route("/logout", web::post().to(handlers::logout))
                    .route("/me", web::get().to(handlers::me)),
            )
            .service(
                web::scope("/v1/projects")
                    .route("", web::get().to(handlers::list_projects))
                    .route("", web::post().to(handlers::create_project)),
            )
            .service(
                web::scope("/v1/tasks")
                    .route("", web::post().to(handlers::dispatch_task)),
            )
    })
    .bind(&bind_addr)?
    .run()
    .await
}
