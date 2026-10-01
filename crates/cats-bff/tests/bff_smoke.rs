//! cats-bff BFF smoke test (per 切片 A _slice_a_bff.md §交付清单 7)
//!
//! 目的: 验证 8 endpoint 都正确注册 + RBAC 中间件在无 token 时正确返回 401
//!
//! 测试策略 (per 守门 #11 缺标比错标):
//! 1. /healthz 在无上游时仍应返回 200 (本地响应)
//! 2. /v1/auth/login 在无 upstream auth-service 时返回 502 (统一错误信封)
//! 3. /v1/auth/me 在无 token 时返回 401 (统一错误信封)
//! 4. /v1/projects 在无 token 时返回 401
//! 5. /v1/auth/me 在"签名与 JWT_SECRET 不匹配"的伪造 token 下返回 401 (per ULYS-149 审核修复)
//! 6. /v1/auth/me 在"结构完全不是 JWT"的垃圾字符串下返回 401
//! 7. /v1/auth/me 在正确签名 + 未过期的 token 下能通过验签这一关 (不是 401)
//! 8. /v1/auth/me 在"签名正确但 token_type=refresh"的 token 下返回 401 (per PR #12 review 补充)
//! 9. /v1/auth/me 在"签名正确但 payload 里根本没有 token_type 字段"的 token 下返回 401
//!    (per token_type 校验完整性复核, 2026-09-23; 模拟 fix 上线前签发的旧 token)
//!
//! 注: 上游 service 没起 → 业务 endpoint 返回 502, 不是 200, 这也是验收的预期行为
//!
//! 关于测试里的 JWT_SECRET (per ULYS-149 审核修复新增测试, 2026-09-23):
//! 下面 5/6/7/8/9 五个测试都会读写进程级环境变量 `JWT_SECRET`。`cargo test` 默认在同一
//! 进程内多线程并发跑各个 `#[test]`，为避免几个测试互相覆写导致 flaky，全部统一设成
//! 同一个常量 `TEST_JWT_SECRET`——设成相同值即使并发写入也不产生竞争 (最终值恒定)。
//! "伪造签名"测试签发时刻意用另一个不同的常量, 与 JWT_SECRET 当前是什么值无关。

use actix_web::{http::StatusCode, test, web, App};
use cats_bff::{
    config::Config,
    handlers,
    principal::{shared_checker, Claims},
    upstream::{auth::AuthClient, projects::ProjectsClient, tasks::TasksClient},
};
use jsonwebtoken::{encode, EncodingKey, Header};
use std::env;
use std::sync::Arc;

/// 测试专用 JWT_SECRET (per 上方模块文档 "关于测试里的 JWT_SECRET")
const TEST_JWT_SECRET: &str = "cats_bff_test_secret_at_least_32_bytes_long_x";
/// 故意用一个不同的密钥签发"伪造"token, 用于验证验签会拒绝密钥不匹配的情况
const WRONG_JWT_SECRET: &str = "a_completely_different_secret_the_bff_never_sees";

fn sign_test_token(secret: &str, exp_offset_secs: i64, token_type: &str) -> String {
    let claims = Claims {
        sub: "00000000-0000-0000-0000-000000000001".to_string(),
        username: "test-user".to_string(),
        exp: chrono::Utc::now().timestamp() + exp_offset_secs,
        token_type: token_type.to_string(),
        roles: vec![],
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("test token signing should not fail")
}

/// 模拟"payload 里根本没有 token_type 字段"的 token（例如 fix 上线前旧版 auth-service
/// 签发、或客户端缓存下来的旧 token）。故意不用 `cats_bff::principal::Claims`（它现在
/// 要求这个字段），单独定义一个更窄的结构体来构造这种"字段缺失"的边界情况。
#[derive(serde::Serialize)]
struct ClaimsMissingTokenType {
    sub: String,
    username: String,
    exp: i64,
}

fn sign_token_missing_token_type(secret: &str, exp_offset_secs: i64) -> String {
    let claims = ClaimsMissingTokenType {
        sub: "00000000-0000-0000-0000-000000000001".to_string(),
        username: "test-user".to_string(),
        exp: chrono::Utc::now().timestamp() + exp_offset_secs,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("test token signing should not fail")
}

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

// ---- ULYS-149 审核修复: 否定路径测试 (伪造/篡改 token 必须被拒绝) ----

#[actix_web::test]
async fn me_with_forged_signature_returns_401() {
    // BFF 只信任用 JWT_SECRET 签的 token；用另一个密钥签出的 token
    // 即使 payload 字段（sub/username/exp）看起来完全合法，也必须被拒绝。
    env::set_var("JWT_SECRET", TEST_JWT_SECRET);
    let forged = sign_test_token(WRONG_JWT_SECRET, 3600, "access");

    let app = test::init_service(test_app()).await;
    let req = test::TestRequest::get()
        .uri("/v1/auth/me")
        .insert_header(("Authorization", format!("Bearer {forged}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn me_with_garbage_token_returns_401() {
    // 结构上完全不是 JWT 的字符串（不是三段 base64url，无从谈起签名）也必须被拒绝，
    // 而不是被当成"缺失字段"之类的宽松解析放行。
    env::set_var("JWT_SECRET", TEST_JWT_SECRET);

    let app = test::init_service(test_app()).await;
    let req = test::TestRequest::get()
        .uri("/v1/auth/me")
        .insert_header(("Authorization", "Bearer not.a.jwt"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn me_with_expired_token_returns_401() {
    // 签名正确但已过期的 token 也必须被拒绝 (验证 validate_exp 生效)。
    env::set_var("JWT_SECRET", TEST_JWT_SECRET);
    let expired = sign_test_token(TEST_JWT_SECRET, -3600, "access");

    let app = test::init_service(test_app()).await;
    let req = test::TestRequest::get()
        .uri("/v1/auth/me")
        .insert_header(("Authorization", format!("Bearer {expired}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn me_with_valid_signed_token_passes_auth_gate() {
    // 用与 BFF 相同的 JWT_SECRET 正确签名、未过期的 token 应当能通过 Principal::require_auth
    // 这一关（不是 401）。测试环境没有起真正的 auth-service upstream，
    // 所以请求会在 handler 内部转发失败，最终变成 502/500——这恰好证明请求
    // 已经越过了鉴权层，走到了业务逻辑，与 login_without_upstream_returns_502 是同一模式。
    env::set_var("JWT_SECRET", TEST_JWT_SECRET);
    let valid = sign_test_token(TEST_JWT_SECRET, 3600, "access");

    let app = test::init_service(test_app()).await;
    let req = test::TestRequest::get()
        .uri("/v1/auth/me")
        .insert_header(("Authorization", format!("Bearer {valid}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ne!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "correctly signed, unexpired token must not be rejected as unauthorized"
    );
}

#[actix_web::test]
async fn me_with_refresh_token_returns_401() {
    // token_type=refresh 的 token 即使签名正确、未过期，也不能当 access token 用
    // (per PR #12 review: refresh token 泄漏不应扩大到能打所有受保护端点)。
    env::set_var("JWT_SECRET", TEST_JWT_SECRET);
    let refresh = sign_test_token(TEST_JWT_SECRET, 3600, "refresh");

    let app = test::init_service(test_app()).await;
    let req = test::TestRequest::get()
        .uri("/v1/auth/me")
        .insert_header(("Authorization", format!("Bearer {refresh}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "a refresh token must not be accepted as a bearer access token on a protected endpoint"
    );
}

#[actix_web::test]
async fn me_with_token_missing_token_type_returns_401() {
    // 正确密钥签名、未过期，但 payload 里根本没有 token_type 字段的 token
    // （例如 fix 上线前签发的旧 token）必须被拒绝，而不是因为 serde 反序列化
    // 宽松处理缺失字段而被静默放行。
    env::set_var("JWT_SECRET", TEST_JWT_SECRET);
    let missing = sign_token_missing_token_type(TEST_JWT_SECRET, 3600);

    let app = test::init_service(test_app()).await;
    let req = test::TestRequest::get()
        .uri("/v1/auth/me")
        .insert_header(("Authorization", format!("Bearer {missing}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "a token missing the token_type field must not be silently accepted"
    );
}
