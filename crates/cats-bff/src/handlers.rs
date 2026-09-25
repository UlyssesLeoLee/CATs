//! cats-bff 业务 handler (per 切片 A _slice_a_bff.md §交付清单 3)
//!
//! 8 endpoint:
//!   POST /v1/auth/login
//!   POST /v1/auth/refresh
//!   POST /v1/auth/logout
//!   GET  /v1/auth/me
//!   GET  /v1/projects
//!   POST /v1/projects
//!   POST /v1/tasks
//!   GET  /healthz (本地, 在 main.rs 注册)

use crate::config::Config;
use crate::error::{BffError, BffResult};
use crate::principal::Principal;
use crate::upstream::auth::{AuthClient, LoginRequest, RefreshRequest, LogoutRequest};
use crate::upstream::projects::{ProjectsClient, ListProjectsQuery, CreateProjectRequest};
use crate::upstream::tasks::{TasksClient, DispatchTaskRequest};

use cats_rbac::{Action, Resource, RbacChecker};
use actix_web::{web, HttpRequest, HttpResponse};
use std::sync::Arc;

// ---- POST /v1/auth/login ----

pub async fn login(
    auth: web::Data<AuthClient>,
    body: web::Json<LoginRequest>,
) -> BffResult<HttpResponse> {
    let body = body.into_inner();
    if body.username.is_empty() || body.password.is_empty() {
        return Err(BffError::InvalidRequest(
            "username and password required".to_string(),
        ));
    }
    let resp = auth.login(&body).await?;
    Ok(HttpResponse::Ok().json(resp))
}

// ---- POST /v1/auth/refresh ----

pub async fn refresh(
    auth: web::Data<AuthClient>,
    body: web::Json<RefreshRequest>,
) -> BffResult<HttpResponse> {
    let resp = auth.refresh(&body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(resp))
}

// ---- POST /v1/auth/logout ----

pub async fn logout(
    auth: web::Data<AuthClient>,
    body: web::Json<LogoutRequest>,
    req: HttpRequest,
) -> BffResult<HttpResponse> {
    let access_token = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    let resp = auth
        .logout(&body.into_inner(), access_token.as_deref())
        .await?;
    Ok(HttpResponse::Ok().json(resp))
}

// ---- GET /v1/auth/me ----

pub async fn me(
    auth: web::Data<AuthClient>,
    principal: Principal,
    _checker: web::Data<Arc<RbacChecker>>,
) -> BffResult<HttpResponse> {
    let resp = auth.me(&principal.access_token).await?;
    Ok(HttpResponse::Ok().json(resp))
}

// ---- GET /v1/projects ----

pub async fn list_projects(
    projects: web::Data<ProjectsClient>,
    principal: Principal,
    checker: web::Data<Arc<RbacChecker>>,
    query: web::Query<ListProjectsQuery>,
) -> BffResult<HttpResponse> {
    principal
        .check(checker.get_ref(), Resource::Project, Action::Read)
        .await?;
    let resp = projects
        .list(&principal.access_token, &query.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(resp))
}

// ---- POST /v1/projects ----

pub async fn create_project(
    projects: web::Data<ProjectsClient>,
    principal: Principal,
    checker: web::Data<Arc<RbacChecker>>,
    body: web::Json<CreateProjectRequest>,
    req: HttpRequest,
) -> BffResult<HttpResponse> {
    principal
        .check(checker.get_ref(), Resource::Project, Action::Create)
        .await?;
    let idem = req
        .headers()
        .get("Idempotency-Key")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let resp = projects
        .create(&principal.access_token, &body.into_inner(), idem.as_deref())
        .await?;
    Ok(HttpResponse::Created().json(resp))
}

// ---- POST /v1/tasks ----

pub async fn dispatch_task(
    tasks: web::Data<TasksClient>,
    principal: Principal,
    checker: web::Data<Arc<RbacChecker>>,
    body: web::Json<DispatchTaskRequest>,
) -> BffResult<HttpResponse> {
    principal
        .check(checker.get_ref(), Resource::Task, Action::Create)
        .await?;
    let resp = tasks
        .dispatch(&principal.access_token, &body.into_inner())
        .await?;
    Ok(HttpResponse::Accepted().json(resp))
}

// ---- GET /healthz ---- (per §1, 在 main.rs 注册)

pub async fn healthz_handler(cfg: web::Data<Config>) -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "cats-bff",
        "version": env!("CARGO_PKG_VERSION"),
        "bind_addr": cfg.bind_addr,
        "upstreams": {
            "auth_service_url": cfg.auth_service_url,
            "user_service_url": cfg.user_service_url,
            "project_service_url": cfg.project_service_url,
            "task_service_url": cfg.task_service_url,
            "translation_core_url": cfg.translation_core_url,
        }
    }))
}
