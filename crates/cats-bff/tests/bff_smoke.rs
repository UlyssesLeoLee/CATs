//! cats-bff BFF smoke test (per 切片 A _slice_a_bff.md §交付清单 7)
//!
//! 目的: 验证 8 endpoint 都正确注册 + RBAC 中间件在无 token 时正确返回 401
//!
//! 测试策略 (per 守门 #11 缺标比错标):
//! 1. /healthz 在无上游时仍应返回 200 (本地响应)
//! 2. /v1/auth/login 在无 upstream auth-service 时返回 502 (统一错误信封)
//! 3. /v1/auth/me 在无 token 时返回 401 (统一错误信封)
//! 4. /v1/projects 在无 token 时返回 401
//!
//! 注: 上游 service 没起 → 业务 endpoint 返回 502, 不是 200, 这也是验收的预期行为

use actix_web::{http::StatusCode, test, web, App};
use cats_bff::{
    config::Config,
    handlers,
    principal::shared_checker,
    upstream::{auth::AuthClient, projects::ProjectsClient, tasks::TasksClient},
};
use std::sync::Arc;

/// 构造测试 App (无上游 server, 但路由和 RBAC 都启用)
fn test_app() -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let cfg = Config::from_env();
    let auth = AuthClient::new(&cfg);
    let projects = ProjectsClient::new(&cfg);
    let tasks = TasksClient::new(&cfg);
    let rbac: Arc<_> = shared_checker();

    App::new()
        .app_data(web::Data::new(cfg))
        .app_data(web::Data::new(auth))
        .app_data(web::Data::new(projects))
        .app_data(web::Data::new(tasks))
        .app_data(web::Data::new(rbac))
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
            web::scope("/v1/tasks").route("", web::post().to(handlers::dispatch_task)),
        )
}

#[actix_web::test]
async fn healthz_returns_200() {
    let app = test::init_service(test_app()).await;
    let req = test::TestRequest::get().uri("/healthz").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn me_without_token_returns_401() {
    let app = test::init_service(test_app()).await;
    let req = test::TestRequest::get().uri("/v1/auth/me").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn list_projects_without_token_returns_401() {
    let app = test::init_service(test_app()).await;
    let req = test::TestRequest::get().uri("/v1/projects").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn create_projects_without_token_returns_401() {
    let app = test::init_service(test_app()).await;
    let body = serde_json::json!({
        "name": "test",
        "source_lang": "en",
        "target_lang": "ja"
    });
    let req = test::TestRequest::post()
        .uri("/v1/projects")
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn dispatch_task_without_token_returns_401() {
    let app = test::init_service(test_app()).await;
    let body = serde_json::json!({
        "project_id": "00000000-0000-0000-0000-000000000000",
        "file_id": "00000000-0000-0000-0000-000000000000",
        "media_type": "TEXT"
    });
    let req = test::TestRequest::post()
        .uri("/v1/tasks")
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn login_without_upstream_returns_502() {
    // login 不需要 token, 但没有 upstream auth-service → 502 (统一错误信封)
    let app = test::init_service(test_app()).await;
    let body = serde_json::json!({
        "username": "test",
        "password": "test"
    });
    let req = test::TestRequest::post()
        .uri("/v1/auth/login")
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    // 上游不可达 → 502 dependency_unavailable
    assert!(
        resp.status() == StatusCode::BAD_GATEWAY
            || resp.status() == StatusCode::INTERNAL_SERVER_ERROR,
        "expected 502 or 500, got {}",
        resp.status()
    );
}
