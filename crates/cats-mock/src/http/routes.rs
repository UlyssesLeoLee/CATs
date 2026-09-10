//! 预制业务路由 (per OpenAPI v1 + 微服务架构书 §4.1)
//!
//! 引用: 设计书 §4.4.3
//!
//! 每个 service 一组路由, 注册时挂到 MockServer 上
//!
//! 默认行为: 返回固定 mock 响应 (200 + 预制 JSON)
//! 可通过 `with_state` 替换成 dynamic factory (e.g. data factory 产出随机 User)

use actix_web::{web, HttpResponse};
use serde_json::json;

use super::response::{MockError, ResponseBuilder};

/// 配置注册 auth-service 路由
pub fn auth_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/auth")
            .route("/login", web::post().to(mock_login))
            .route("/refresh", web::post().to(mock_refresh))
            .route("/logout", web::post().to(mock_logout))
            .route("/me", web::get().to(mock_me)),
    );
}

/// 配置注册 user-service 路由
pub fn user_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/users")
            .route("", web::post().to(mock_create_user))
            .route("/{id}", web::get().to(mock_get_user))
            .route("/{id}", web::put().to(mock_update_user))
            .route("/{id}", web::delete().to(mock_delete_user)),
    );
}

/// 配置注册 project-service 路由
pub fn project_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/projects")
            .route("", web::post().to(mock_create_project))
            .route("", web::get().to(mock_list_projects))
            .route("/{id}", web::get().to(mock_get_project))
            .route("/{id}", web::put().to(mock_update_project))
            .route("/{id}", web::delete().to(mock_delete_project)),
    );
}

/// 配置注册 task-service 路由
pub fn task_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/tasks")
            .route("", web::post().to(mock_create_task))
            .route("", web::get().to(mock_list_tasks))
            .route("/{id}", web::get().to(mock_get_task))
            .route("/{id}/cancel", web::post().to(mock_cancel_task)),
    );
}

/// 配置注册 audit-service 路由
pub fn audit_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/audit")
            .route("/events", web::get().to(mock_list_audit_events))
            .route("/events/{event_id}", web::get().to(mock_get_audit_event)),
    );
}

/// 通用 healthz
pub fn healthz_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/healthz", web::get().to(|| async {
        HttpResponse::Ok().json(json!({"status": "ok"}))
    }));
}

// =====================================================================
// Handler 实现 (返回 mock 响应)
// =====================================================================

async fn mock_login() -> HttpResponse {
    ResponseBuilder::ok(json!({
        "access_token": "mock_access_token_xxxxx",
        "refresh_token": "mock_refresh_token_yyyyy",
        "expires_in": 3600,
        "token_type": "Bearer",
        "user_id": "00000000-0000-0000-0000-000000000001",
        "username": "mock_user",
    }))
}

async fn mock_refresh() -> HttpResponse {
    ResponseBuilder::ok(json!({
        "access_token": "mock_access_token_new",
        "refresh_token": "mock_refresh_token_new",
        "expires_in": 3600,
        "token_type": "Bearer",
    }))
}

async fn mock_logout() -> HttpResponse {
    ResponseBuilder::ok(json!({
        "revoked": true,
        "revoked_at": "2026-08-31T16:00:00Z",
    }))
}

async fn mock_me() -> HttpResponse {
    ResponseBuilder::ok(json!({
        "user_id": "00000000-0000-0000-0000-000000000001",
        "username": "mock_user",
        "email": "mock@example.com",
    }))
}

async fn mock_create_user() -> HttpResponse {
    ResponseBuilder::created(json!({
        "id": "00000000-0000-0000-0000-000000000010",
        "username": "new_user",
        "email": "new@example.com",
        "is_active": true,
        "created_at": "2026-08-31T16:00:00Z",
    }))
}

async fn mock_get_user(path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    if id == "00000000-0000-0000-0000-000000000000" {
        return ResponseBuilder::err(MockError::NotFound);
    }
    ResponseBuilder::ok(json!({
        "id": id,
        "username": "mock_user",
        "email": "mock@example.com",
    }))
}

async fn mock_update_user() -> HttpResponse {
    ResponseBuilder::ok(json!({"updated": true}))
}

async fn mock_delete_user() -> HttpResponse {
    ResponseBuilder::no_content()
}

async fn mock_create_project() -> HttpResponse {
    ResponseBuilder::created(json!({
        "id": "00000000-0000-0000-0000-000000000020",
        "name": "Mock Project",
        "status": "draft",
    }))
}

async fn mock_list_projects() -> HttpResponse {
    ResponseBuilder::ok(json!({
        "items": [],
        "total": 0,
        "page": 1,
        "page_size": 20,
    }))
}

async fn mock_get_project(path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    if id == "missing" {
        return ResponseBuilder::err(MockError::NotFound);
    }
    ResponseBuilder::ok(json!({"id": id, "name": "Mock Project", "status": "active"}))
}

async fn mock_update_project() -> HttpResponse {
    ResponseBuilder::ok(json!({"updated": true}))
}

async fn mock_delete_project() -> HttpResponse {
    ResponseBuilder::no_content()
}

async fn mock_create_task() -> HttpResponse {
    ResponseBuilder::created(json!({
        "id": "00000000-0000-0000-0000-000000000030",
        "task_type": "translate",
        "status": "pending",
    }))
}

async fn mock_list_tasks() -> HttpResponse {
    ResponseBuilder::ok(json!({"items": [], "total": 0}))
}

async fn mock_get_task(path: web::Path<String>) -> HttpResponse {
    ResponseBuilder::ok(json!({
        "id": path.into_inner(),
        "status": "pending",
    }))
}

async fn mock_cancel_task() -> HttpResponse {
    ResponseBuilder::ok(json!({"cancelled": true}))
}

async fn mock_list_audit_events() -> HttpResponse {
    ResponseBuilder::ok(json!({"items": [], "total": 0}))
}

async fn mock_get_audit_event(path: web::Path<String>) -> HttpResponse {
    ResponseBuilder::ok(json!({
        "event_id": path.into_inner(),
        "event_type": "login",
        "outcome": "success",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test as actix_test, App};

    #[actix_web::test]
    async fn auth_login_returns_200() {
        let app = actix_test::init_service(
            App::new().configure(auth_routes),
        )
        .await;
        let req = actix_test::TestRequest::post()
            .uri("/v1/auth/login")
            .set_json(json!({"username": "x", "password": "y"}))
            .to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 200);
    }

    #[actix_web::test]
    async fn user_get_existing_returns_200() {
        let app = actix_test::init_service(
            App::new().configure(user_routes),
        )
        .await;
        let req = actix_test::TestRequest::get()
            .uri("/v1/users/00000000-0000-0000-0000-000000000001")
            .to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 200);
    }

    #[actix_web::test]
    async fn user_get_zero_uuid_returns_404() {
        let app = actix_test::init_service(
            App::new().configure(user_routes),
        )
        .await;
        let req = actix_test::TestRequest::get()
            .uri("/v1/users/00000000-0000-0000-0000-000000000000")
            .to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 404);
    }

    #[actix_web::test]
    async fn project_list_returns_200() {
        let app = actix_test::init_service(
            App::new().configure(project_routes),
        )
        .await;
        let req = actix_test::TestRequest::get()
            .uri("/v1/projects")
            .to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 200);
    }

    #[actix_web::test]
    async fn task_create_returns_201() {
        let app = actix_test::init_service(
            App::new().configure(task_routes),
        )
        .await;
        let req = actix_test::TestRequest::post()
            .uri("/v1/tasks")
            .set_json(json!({"task_type": "translate"}))
            .to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 201);
    }

    #[actix_web::test]
    async fn healthz_returns_200() {
        let app = actix_test::init_service(
            App::new().configure(healthz_routes),
        )
        .await;
        let req = actix_test::TestRequest::get()
            .uri("/healthz")
            .to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 200);
    }

    #[actix_web::test]
    async fn all_routes_register_without_panic() {
        // 注册全部, 不能 panic
        let app = actix_test::init_service(
            App::new()
                .configure(auth_routes)
                .configure(user_routes)
                .configure(project_routes)
                .configure(task_routes)
                .configure(audit_routes)
                .configure(healthz_routes),
        )
        .await;
        // 随便打一个路由验证 server 活着
        let req = actix_test::TestRequest::get().uri("/healthz").to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 200);
    }
}
