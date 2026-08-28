//! user-service T-02 实战落地 e2e 测试
//!
//! 引用: doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0+1.md §2 T-02
//!
//! 覆盖:
//! - healthz → 200
//! - POST /v1/users 创建 → 201 + profile
//! - GET  /v1/users/{id} 命中 → 200 + 字段一致
//! - GET  /v1/users/{id} 不存在 → 404 user_not_found
//! - PUT  /v1/users/{id} 部分更新 → 200 + 新值
//!
//! 完成判据 (per Sprint 1 §2 T-02):
//! ① cargo build -p user-service exit 0
//! ② cargo test -p user-service 5/5 通过 (本文件)
//! ③ healthz e2e 1/1 通过 (curl → 200)
//! ④ 用户 CRUD 最小用例 2/2 通过 (创建 → 读取 → 更新)
//! ⑤ cats-kit crate 全 workspace cargo build exit 0 (per 已有 cats-common, 复用)

// 注: ⑤ cats-kit 实际是 cats-common (已存在, 复用), 详见 commit message

use actix_web::{test as actix_test, web, App};
use serde_json::json;
use sqlx::PgPool;
use std::env;
use std::sync::Once;
use uuid::Uuid;

use user_service::handlers;
use user_service::models::{CreateUserRequest, GetUserResponse, UpdateUserRequest};

/// 集成测试环境 (env var 一次性检查)
fn setup_env() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        if env::var("DATABASE_URL").is_err() {
            panic!("DATABASE_URL must be set for e2e tests (e.g. postgres://svc_user:rgs_dev@localhost:5432/user_test_db)");
        }
    });
}

async fn make_pool() -> PgPool {
    user_service::db::build_pool()
        .await
        .expect("build_pool failed (check DATABASE_URL)")
}

fn unique_test_user_id() -> uuid::Uuid {
    Uuid::new_v4()
}

async fn cleanup_test_user(pool: &PgPool, user_id: uuid::Uuid) {
    sqlx::query("DELETE FROM user_profile WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await
        .ok();
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
        .route("/v1/users", web::post().to(handlers::create_user))
        .route("/v1/users/{id}", web::get().to(handlers::get_user))
        .route("/v1/users/{id}", web::put().to(handlers::update_user))
}

// =====================================================================
// 1. healthz
// =====================================================================
#[actix_web::test]
async fn e2e_healthz_returns_200() {
    setup_env();
    let pool = make_pool().await;
    let app = actix_test::init_service(make_app(pool)).await;
    let req = actix_test::TestRequest::get().uri("/healthz").to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = actix_test::read_body_json(resp).await;
    assert_eq!(body["status"], json!("ok"));
    assert_eq!(body["name"], json!("user-service"));
    assert!(body["version"].as_str().unwrap().starts_with("0.1."));
}

// =====================================================================
// 2. POST /v1/users 创建 → 201
// =====================================================================
#[actix_web::test]
async fn e2e_create_user_returns_201() {
    setup_env();
    let pool = make_pool().await;
    let user_id = unique_test_user_id();

    let app = actix_test::init_service(make_app(pool.clone())).await;
    let req = actix_test::TestRequest::post()
        .uri("/v1/users")
        .set_json(CreateUserRequest {
            user_id,
            display_name: "Alice T02".to_string(),
            email: Some(format!("alice-t02-{user_id}@cats.example")),
            avatar_url: None,
            locale: "ja-JP".to_string(),
            timezone: "Asia/Tokyo".to_string(),
        })
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 201, "create should return 201");
    let body: GetUserResponse = actix_test::read_body_json(resp).await;
    assert_eq!(body.user_id, user_id.to_string());
    assert_eq!(body.display_name, "Alice T02");
    assert!(body.email.is_some());

    cleanup_test_user(&pool, user_id).await;
}

// =====================================================================
// 3. GET /v1/users/{id} 命中
// =====================================================================
#[actix_web::test]
async fn e2e_get_user_by_id_returns_200() {
    setup_env();
    let pool = make_pool().await;
    let user_id = unique_test_user_id();
    let email = format!("bob-t02-{user_id}@cats.example");

    let app = actix_test::init_service(make_app(pool.clone())).await;

    // 先 create
    let create_req = actix_test::TestRequest::post()
        .uri("/v1/users")
        .set_json(CreateUserRequest {
            user_id,
            display_name: "Bob T02".to_string(),
            email: Some(email.clone()),
            avatar_url: Some("https://cdn.cats.example/avatar/bob.png".to_string()),
            locale: "en-US".to_string(),
            timezone: "America/Los_Angeles".to_string(),
        })
        .to_request();
    let create_resp = actix_test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status().as_u16(), 201);
    let created: GetUserResponse = actix_test::read_body_json(create_resp).await;
    let id = created.id.clone();

    // 再 get by id
    let get_req = actix_test::TestRequest::get()
        .uri(&format!("/v1/users/{id}"))
        .to_request();
    let get_resp = actix_test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status().as_u16(), 200);
    let fetched: GetUserResponse = actix_test::read_body_json(get_resp).await;
    assert_eq!(fetched.id, id);
    assert_eq!(fetched.user_id, user_id.to_string());
    assert_eq!(fetched.display_name, "Bob T02");
    assert_eq!(fetched.email.as_deref(), Some(email.as_str()));
    assert_eq!(fetched.locale, "en-US");
    assert_eq!(fetched.timezone, "America/Los_Angeles");

    cleanup_test_user(&pool, user_id).await;
}

// =====================================================================
// 4. GET /v1/users/{id} 不存在 → 404 user_not_found
// =====================================================================
#[actix_web::test]
async fn e2e_get_user_not_found_returns_404() {
    setup_env();
    let pool = make_pool().await;
    let app = actix_test::init_service(make_app(pool)).await;

    let non_existing = Uuid::new_v4();
    let req = actix_test::TestRequest::get()
        .uri(&format!("/v1/users/{non_existing}"))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
    let body: serde_json::Value = actix_test::read_body_json(resp).await;
    assert_eq!(body["error"], json!("user_not_found"));
}

// =====================================================================
// 5. PUT /v1/users/{id} 部分更新
// =====================================================================
#[actix_web::test]
async fn e2e_update_user_partial_returns_200() {
    setup_env();
    let pool = make_pool().await;
    let user_id = unique_test_user_id();

    let app = actix_test::init_service(make_app(pool.clone())).await;

    // create
    let create_req = actix_test::TestRequest::post()
        .uri("/v1/users")
        .set_json(CreateUserRequest {
            user_id,
            display_name: "Carol T02".to_string(),
            email: Some(format!("carol-t02-{user_id}@cats.example")),
            avatar_url: None,
            locale: "ja-JP".to_string(),
            timezone: "Asia/Tokyo".to_string(),
        })
        .to_request();
    let create_resp = actix_test::call_service(&app, create_req).await;
    let created: GetUserResponse = actix_test::read_body_json(create_resp).await;
    let id = created.id.clone();

    // update 部分字段 (display_name + timezone)
    let update_req = actix_test::TestRequest::put()
        .uri(&format!("/v1/users/{id}"))
        .set_json(UpdateUserRequest {
            display_name: Some("Carol T02 (updated)".to_string()),
            email: None,
            avatar_url: None,
            locale: None,
            timezone: Some("Europe/London".to_string()),
        })
        .to_request();
    let update_resp = actix_test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status().as_u16(), 200);
    let updated: GetUserResponse = actix_test::read_body_json(update_resp).await;
    assert_eq!(updated.display_name, "Carol T02 (updated)");
    assert_eq!(updated.timezone, "Europe/London");
    // email 保持不变
    assert!(updated.email.is_some());

    cleanup_test_user(&pool, user_id).await;
}
