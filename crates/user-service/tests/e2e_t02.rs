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
    // 2026-10-07: 挂生产的 `handlers::configure_routes`，不再自己抄一份路由
    // （原来 4 = 4 完全一致，所以改之前不是假绿；per BACKEND_STATUS §4.1t）
    App::new().configure(move |c| handlers::configure_routes(c, pool_data.clone()))
}

// =====================================================================
// 关于身份头（2026-10-07）
//
// 本服务三条业务路由在 §4.1x 之前**完全没有鉴权**，之后统一走
// `cats_rbac::service_helpers::extract_user_id_and_roles` —— 缺 `X-Cats-*`
// 就该 401。e2e 用例原先不带这两个头，于是 `e2e (real PostgreSQL)` 里
// 4 个用例全红（实测 `left: 401`，CI run 112794634614）。
//
// **是测试错了，不是代码错了。** 生产链路里请求必然经过 envoy：
// `jwt_authn` 验签后按 `claim_to_headers` 注入 `sub → X-Cats-User-Id`、
// `roles_csv → X-Cats-Roles`，而路由上的 `request_headers_to_remove` 会先
// 剥掉客户端自带的同名头。所以"不带身份头"在生产中不可能发生 ——
// 发生的是 401，而这正是期望行为。
//
// 给测试开后门（比如加一个 `#[cfg(test)]` 跳过鉴权的分支）会让**真正缺鉴权
// 的回归测试不出来** —— 那才是更贵的错误。所以这里老老实实把头带上。
//
// ---------------------------------------------------------------------
// 为什么角色是 Sponsor 而不是 User（这是个待拍板的发现，不是随手选的）
//
// `route_to_resource_action` 把三条路由映射成：
//   POST /v1/users        → (User, Create)
//   GET  /v1/users/{id}   → (User, Read)
//   PUT  /v1/users/{id}   → (User, Update)
//
// 而 `cats_rbac::default_permissions()` 里 `Role::User` 对 `Resource::User`
// **只有 Read**（lib.rs:283-296）—— 没有 Create，也没有 Update。
// 逐个角色核过：Sponsor 全权；ArchitectLead 只有 Read；RustLead 只管
// Service；DatabaseLead 对 User 只有 Read；QualityLead 只有 Read；
// ProjectLead 只管 Sprint/Decision/Risk/Gap；SRELead 只管 Alert/KafkaTopic/
// K8sResource；Guest 只读公开资源。
//
// ⇒ **`User × Create` 和 `User × Update` 在当前矩阵下只有 Sponsor 能过。**
// crate 内已有单测 `plain_user_cannot_create_users` 把这条钉住了。
//
// 也就是说：普通用户既不能自助注册、也不能改自己的档案。对一个要商业化
// 的产品来说这大概率不是本意，但**改权限矩阵是设计决策，不在修 CI 的范围
// 内**，已单独记入 §4.1u 待拍板。这里用 Sponsor 让 e2e 覆盖到真正的 CRUD 路径，
// 并且把上面这段原因写在文件里，免得下一个读的人以为 Sponsor 是随手选的。
// ---------------------------------------------------------------------
// =====================================================================

// =====================================================================
// 1. healthz
// =====================================================================
#[actix_web::test]
#[ignore = "e2e-needs-real-pg: requires DATABASE_URL + JWT_SECRET (real PG); run with -- --ignored"]
async fn e2e_healthz_returns_200() {
    setup_env();
    let pool = make_pool().await;
    let app = actix_test::init_service(make_app(pool)).await;
    let req = actix_test::TestRequest::get().uri("/healthz").to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = actix_test::read_body_json(resp).await;
    assert_eq!(body["status"], json!("ok"));
    // 形状 per BACKEND_STATUS §4.1m 统一后的全仓唯一形状
    assert_eq!(body["app"]["name"], json!(env!("CARGO_PKG_NAME")));
    assert!(body["app"]["version"].as_str().unwrap().starts_with("0.1."));
}

// =====================================================================
// 2. POST /v1/users 创建 → 201
// =====================================================================
#[actix_web::test]
#[ignore = "e2e-needs-real-pg: requires DATABASE_URL + JWT_SECRET (real PG); run with -- --ignored"]
async fn e2e_create_user_returns_201() {
    setup_env();
    let pool = make_pool().await;
    let user_id = unique_test_user_id();

    let app = actix_test::init_service(make_app(pool.clone())).await;
    let req = actix_test::TestRequest::post()
        .uri("/v1/users")
        // 调用者就是被创建的那个用户自己
        .insert_header(("X-Cats-User-Id", user_id.to_string()))
        .insert_header(("X-Cats-Roles", "Sponsor"))
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
#[ignore = "e2e-needs-real-pg: requires DATABASE_URL + JWT_SECRET (real PG); run with -- --ignored"]
async fn e2e_get_user_by_id_returns_200() {
    setup_env();
    let pool = make_pool().await;
    let user_id = unique_test_user_id();
    let email = format!("bob-t02-{user_id}@cats.example");

    let app = actix_test::init_service(make_app(pool.clone())).await;

    // 先 create
    let create_req = actix_test::TestRequest::post()
        .uri("/v1/users")
        .insert_header(("X-Cats-User-Id", user_id.to_string()))
        .insert_header(("X-Cats-Roles", "Sponsor"))
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
        .insert_header(("X-Cats-User-Id", user_id.to_string()))
        .insert_header(("X-Cats-Roles", "Sponsor"))
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
#[ignore = "e2e-needs-real-pg: requires DATABASE_URL + JWT_SECRET (real PG); run with -- --ignored"]
async fn e2e_get_user_not_found_returns_404() {
    setup_env();
    let pool = make_pool().await;
    let app = actix_test::init_service(make_app(pool)).await;

    let non_existing = Uuid::new_v4();
    let req = actix_test::TestRequest::get()
        .uri(&format!("/v1/users/{non_existing}"))
        .insert_header(("X-Cats-User-Id", Uuid::new_v4().to_string()))
        .insert_header(("X-Cats-Roles", "Sponsor"))
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
#[ignore = "e2e-needs-real-pg: requires DATABASE_URL + JWT_SECRET (real PG); run with -- --ignored"]
async fn e2e_update_user_partial_returns_200() {
    setup_env();
    let pool = make_pool().await;
    let user_id = unique_test_user_id();

    let app = actix_test::init_service(make_app(pool.clone())).await;

    // create
    let create_req = actix_test::TestRequest::post()
        .uri("/v1/users")
        .insert_header(("X-Cats-User-Id", user_id.to_string()))
        .insert_header(("X-Cats-Roles", "Sponsor"))
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
    assert_eq!(
        create_resp.status().as_u16(),
        201,
        "create must succeed before update, got {}",
        create_resp.status()
    );
    let created: GetUserResponse = actix_test::read_body_json(create_resp).await;
    let id = created.id.clone();

    // update 部分字段 (display_name + timezone)
    let update_req = actix_test::TestRequest::put()
        .uri(&format!("/v1/users/{id}"))
        .insert_header(("X-Cats-User-Id", user_id.to_string()))
        .insert_header(("X-Cats-Roles", "Sponsor"))
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
