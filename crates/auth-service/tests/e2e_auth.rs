//! auth-service 端到端测试 (per 实施前QA v1.3 §2.3 OI-3 M1-Sprint 0 末)
//!
//! 引用: doc/02-基础设计/接口设计/CATs_接口设计书_v2.0.md §3
//! 引用: api/openapi/cats-openapi-v1.yaml
//! 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §3
//!
//! 测试环境:
//! - 真实 WSL PG 18.6 (per INC-002 v1.0)
//! - 数据库: auth_test_db (与 8 业务库隔离)
//! - 启动方式: DATABASE_URL=postgres://...auth_test_db cargo test --test e2e_auth
//!
//! 完成 OI-3 收尾 (M1-Sprint 0 末):
//! - (a) POST /v1/auth/login 成功 → 200 + JWT
//! - (b) POST /v1/auth/login 错密码 → 401
//! - (c) POST /v1/auth/refresh 成功 → 200 + 新 access_token
//! - (d) POST /v1/auth/refresh 错 token → 401
//!
//! 安全约束 (per 2026-08-27 11:06 JST):
//! - 密码经 env var 注入 (TEST_PASSWORD)
//! - 不打印任何 env var 内容

use actix_web::{test as actix_test, web, App};
use auth_service::db;
use auth_service::handlers;
use auth_service::models::{
    ErrorBody, LoginRequest, LoginResponse, RefreshRequest, RefreshResponse,
};
use serde_json::json;
use sqlx::PgPool;
use std::env;
use std::sync::Once;

/// 集成测试环境 (env var 一次性检查)
fn setup_env() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        if env::var("DATABASE_URL").is_err() {
            panic!("DATABASE_URL must be set for e2e tests (e.g. postgres://svc_auth:rgs_dev@localhost:5432/auth_test_db)");
        }
        if env::var("JWT_SECRET").is_err() {
            panic!("JWT_SECRET must be set for e2e tests");
        }
    });
}

/// 创建测试用 PgPool (lazy)
async fn make_pool() -> PgPool {
    db::build_pool()
        .await
        .expect("build_pool failed (check DATABASE_URL)")
}

/// 创建测试用种子用户 (用 TEST_PASSWORD env var)
async fn ensure_test_user(pool: &PgPool) -> String {
    let password = env::var("TEST_PASSWORD")
        .expect("TEST_PASSWORD must be set (per 2026-08-27 11:06 JST 安全约束)");
    let username = "e2e_test_user";
    let created = db::ensure_seed_user(pool, username, &password)
        .await
        .expect("ensure_seed_user failed");
    if created {
        eprintln!("[e2e setup] created seed user");
    } else {
        eprintln!("[e2e setup] seed user already exists");
    }
    username.to_string()
}

/// 删除测试用种子用户 (清理)
async fn cleanup_test_user(pool: &PgPool, username: &str) {
    sqlx::query("DELETE FROM users_credential WHERE username = $1")
        .bind(username)
        .execute(pool)
        .await
        .expect("cleanup failed");
}

/// 构造 actix App 用于测试 (与 main.rs 一致)
fn make_app(
    pool: PgPool,
) -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let pool_data = web::Data::new(pool);
    App::new()
        .app_data(pool_data)
        .route("/healthz", web::get().to(handlers::healthz))
        .route("/v1/auth/login", web::post().to(handlers::login))
        .route("/v1/auth/refresh", web::post().to(handlers::refresh))
}

// === (a) POST /v1/auth/login 成功 → 200 + JWT ===
#[actix_web::test]
async fn e2e_login_success_returns_200_with_jwt() {
    setup_env();
    let pool = make_pool().await;
    let username = ensure_test_user(&pool).await;

    let app = actix_test::init_service(make_app(pool.clone())).await;
    let password = env::var("TEST_PASSWORD").expect("TEST_PASSWORD");
    let req = actix_test::TestRequest::post()
        .uri("/v1/auth/login")
        .set_json(LoginRequest {
            username: username.clone(),
            password: password.clone(),
        })
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(
        resp.status().as_u16(),
        200,
        "login should succeed with correct credentials"
    );
    let body: LoginResponse = actix_test::read_body_json(resp).await;
    assert!(!body.access_token.is_empty());
    assert!(!body.refresh_token.is_empty());
    assert_eq!(body.token_type, "Bearer");
    assert_eq!(body.expires_in, 3600);
    assert_eq!(body.username, username);

    cleanup_test_user(&pool, &username).await;
}

// === (b) POST /v1/auth/login 错密码 → 401 ===
#[actix_web::test]
async fn e2e_login_wrong_password_returns_401() {
    setup_env();
    let pool = make_pool().await;
    let username = ensure_test_user(&pool).await;

    let app = actix_test::init_service(make_app(pool.clone())).await;
    let req = actix_test::TestRequest::post()
        .uri("/v1/auth/login")
        .set_json(LoginRequest {
            username: username.clone(),
            password: "definitely_wrong_password_123!@#".to_string(),
        })
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(
        resp.status().as_u16(),
        401,
        "login should fail with wrong password"
    );
    let body: ErrorBody = actix_test::read_body_json(resp).await;
    assert_eq!(body.error, "invalid_credentials");

    cleanup_test_user(&pool, &username).await;
}

// === (c) POST /v1/auth/refresh 成功 → 200 + 新 access_token ===
#[actix_web::test]
async fn e2e_refresh_success_returns_new_access_token() {
    setup_env();
    let pool = make_pool().await;
    let username = ensure_test_user(&pool).await;
    let password = env::var("TEST_PASSWORD").expect("TEST_PASSWORD");

    // 先 login 拿 refresh_token
    let app = actix_test::init_service(make_app(pool.clone())).await;
    let login_req = actix_test::TestRequest::post()
        .uri("/v1/auth/login")
        .set_json(LoginRequest {
            username: username.clone(),
            password: password.clone(),
        })
        .to_request();
    let login_resp = actix_test::call_service(&app, login_req).await;
    assert_eq!(login_resp.status().as_u16(), 200);
    let login_body: LoginResponse = actix_test::read_body_json(login_resp).await;

    // refresh
    let refresh_req = actix_test::TestRequest::post()
        .uri("/v1/auth/refresh")
        .set_json(RefreshRequest {
            refresh_token: login_body.refresh_token.clone(),
        })
        .to_request();
    let refresh_resp = actix_test::call_service(&app, refresh_req).await;
    assert_eq!(
        refresh_resp.status().as_u16(),
        200,
        "refresh should succeed with valid refresh token"
    );
    let refresh_body: RefreshResponse = actix_test::read_body_json(refresh_resp).await;
    assert!(!refresh_body.access_token.is_empty());
    assert_ne!(
        refresh_body.access_token, login_body.access_token,
        "refresh should issue new access token"
    );
    assert_eq!(refresh_body.token_type, "Bearer");
    assert_eq!(refresh_body.expires_in, 3600);

    cleanup_test_user(&pool, &username).await;
}

// === (d) POST /v1/auth/refresh 错 token → 401 ===
#[actix_web::test]
async fn e2e_refresh_invalid_token_returns_401() {
    setup_env();
    let pool = make_pool().await;

    let app = actix_test::init_service(make_app(pool.clone())).await;
    let req = actix_test::TestRequest::post()
        .uri("/v1/auth/refresh")
        .set_json(RefreshRequest {
            refresh_token: "not.a.valid.jwt.token".to_string(),
        })
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(
        resp.status().as_u16(),
        401,
        "refresh should fail with invalid token"
    );
    let body: ErrorBody = actix_test::read_body_json(resp).await;
    assert_eq!(body.error, "invalid_token");
}

// === (bonus) GET /healthz → 200 ===
#[actix_web::test]
async fn e2e_healthz_returns_200() {
    setup_env();
    let pool = make_pool().await;

    let app = actix_test::init_service(make_app(pool.clone())).await;
    let req = actix_test::TestRequest::get().uri("/healthz").to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = actix_test::read_body_json(resp).await;
    assert_eq!(body["status"], json!("ok"));
    assert_eq!(body["service"], json!("auth-service"));
}
