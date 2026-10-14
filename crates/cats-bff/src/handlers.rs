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

use crate::error::{BffError, BffResult};
use crate::principal::Principal;
use crate::upstream::auth::{AuthClient, LoginRequest, LogoutRequest, RefreshRequest};
use crate::upstream::projects::{CreateProjectRequest, ListProjectsQuery, ProjectsClient};
use crate::upstream::tasks::{DispatchTaskRequest, TasksClient};

use actix_web::{web, HttpRequest, HttpResponse};
use cats_rbac::{Action, RbacChecker, Resource};
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

pub async fn healthz_handler() -> HttpResponse {
    // 2026-10-07: 原来这里还吐 `bind_addr` 和 5 个上游 URL（`upstreams`）。
    // 全仓没有任何代码或脚本读这两个字段，且它们是内部拓扑，形状统一后自然
    // 消失。`cfg: web::Data<Config>` 参数随之去掉 —— 去掉后 `use crate::config::Config;`
    // 也就没有别的用处了，一并删除（否则 clippy -D warnings 报 unused）。
    let app = cats_common::AppMeta {
        name: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "app": app,
    }))
}

// =====================================================================
// 生产装配 —— `main.rs` 与集成测试共用（单一事实来源）
// =====================================================================
//
// 2026-10-07: 这段路由表原来内联在 `main.rs` 里，于是集成测试
// (`tests/bff_routes_passthrough.rs`) 只能去挂另一张表 (`routes::configure`，
// 已被删除) —— 13 个用例证明的是一张服务器从不执行的分支。现在装配在这里，
// `main.rs` 与测试挂的是同一张表。形状与 audit-service 的同名 `configure` 一致。

/// 注册全部路由。`main.rs` 与集成测试都必须用它，**不要**各自再写一张表。
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/healthz", web::get().to(healthz_handler))
        .service(
            web::scope("/v1/auth")
                .route("/login", web::post().to(login))
                .route("/refresh", web::post().to(refresh))
                .route("/logout", web::post().to(logout))
                .route("/me", web::get().to(me)),
        )
        .service(
            web::scope("/v1/projects")
                .route("", web::get().to(list_projects))
                .route("", web::post().to(create_project)),
        )
        .service(web::scope("/v1/tasks").route("", web::post().to(dispatch_task)));
}

/// 请求体解析失败时的统一 400 响应体。
///
/// 同样是生产装配的一部分：`app_data` 只能挂在 `App` 上而不是 `ServiceConfig`，
/// 所以它没法进 `configure_routes`，就以工厂形式共享。测试与生产用同一个。
pub fn json_config() -> web::JsonConfig {
    web::JsonConfig::default().error_handler(|err, _req| {
        actix_web::error::InternalError::from_response(
            err,
            HttpResponse::BadRequest().json(crate::error::ErrorBody {
                error: crate::error::ErrorCode::InvalidPayload,
                message: "invalid request body".to_string(),
                detail: None,
            }),
        )
        .into()
    })
}
