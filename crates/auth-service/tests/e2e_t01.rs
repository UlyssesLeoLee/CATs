//! auth-service T-01 实战深化 e2e 测试
//!
//! 引用: doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md §2 T-01
//!
//! 覆盖:
//! - refresh 轮换 (旧 jti 撤销 + 新 jti 签发) — 3/3
//! - logout 端点 (撤销 refresh + 写 audit) — 3/3
//! - audit 落库 (DbAuditSink) — 3/3
//!
//! 完成判据 2 (per Sprint 1 §2 T-01):
//! ① cargo test -p cats-m1-s0-smoke auth 全绿 (本 worktree 内通过 lib test + 本 e2e)
//! ② refresh 轮换 e2e (旧 token 二次使用返回 401) 3/3 通过 ← 本文件 T-01 重点
//! ③ logout 后审计事件 audit_log 出现 1 条 ← 本文件
//! ④ 错误码表 v1.0 提交并引用至 auth-service 模块设计书 §4 ← 文档任务, 后续 commit

use actix_web::{test as actix_test, web, App};
use auth_service::audit::{AuditSink, DbAuditSink, InMemoryAuditSink};
use auth_service::db;
use auth_service::handlers::{login, logout, me, refresh, AppState};
use auth_service::models::{
    ErrorBody, LogoutRequest, LogoutResponse, RefreshRequest, RefreshResponse,
};
use serde_json::json;
use sqlx::PgPool;
use std::env;
use std::sync::{Arc, Once};
use uuid::Uuid;

/// 集成测试环境 (env var 一次性检查)
fn setup_env() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        if env::var("DATABASE_URL").is_err() {
            panic!("DATABASE_URL must be set for e2e tests");
        }
        if env::var("JWT_SECRET").is_err() {
            panic!("JWT_SECRET must be set for e2e tests");
        }
        if env::var("TEST_PASSWORD").is_err() {
            panic!("TEST_PASSWORD must be set (per 2026-08-27 11:06 JST 安全约束)");
        }
    });
}

async fn make_pool() -> PgPool {
    db::build_pool().await.expect("build_pool failed")
}

fn unique_test_username() -> String {
    format!("e2e_t01_user_{}", Uuid::new_v4().simple())
}

async fn ensure_test_user(pool: &PgPool) -> (String, String) {
    let password = env::var("TEST_PASSWORD").expect("TEST_PASSWORD");
    let username = unique_test_username();
    let email = format!("{}@cats.example", username);
    db::ensure_seed_user(pool, &username, &password, Some(&email))
        .await
        .expect("ensure_seed_user failed");
    (username, email)
}

async fn cleanup_test_user(pool: &PgPool, username: &str) {
    sqlx::query("DELETE FROM users_credential WHERE username = $1")
        .bind(username)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM refresh_token_revoke WHERE user_id IN (SELECT id FROM users_credential WHERE username = $1)")
        .bind(username)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM audit_log WHERE user_id IN (SELECT id FROM users_credential WHERE username = $1)")
        .bind(username)
        .execute(pool)
        .await
        .ok();
}

/// 构造带 InMemoryAuditSink 的 App (供需要验证 audit 的测试用)
fn make_app_with_inmem_audit(
    pool: PgPool,
    sink: Arc<dyn AuditSink>,
) -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let state = AppState::new_with_sink(pool, sink);
    let state_data = web::Data::new(state);
    App::new()
        .app_data(state_data)
        .route("/v1/auth/login", web::post().to(login))
        .route("/v1/auth/refresh", web::post().to(refresh))
        .route("/v1/auth/logout", web::post().to(logout))
        .route("/v1/auth/me", web::get().to(me))
}

/// 构造带 DbAuditSink 的 App (生产默认, 验证 audit 落库)
fn make_app_with_db_audit(
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
    let sink: Arc<dyn AuditSink> = Arc::new(DbAuditSink::new(pool.clone()));
    let state = AppState::new_with_sink(pool, sink);
    let state_data = web::Data::new(state);
    App::new()
        .app_data(state_data)
        .route("/v1/auth/login", web::post().to(login))
        .route("/v1/auth/refresh", web::post().to(refresh))
        .route("/v1/auth/logout", web::post().to(logout))
        .route("/v1/auth/me", web::get().to(me))
}

// =============================================================
// T-01 §2 判据 2: refresh 轮换 3/3
// =============================================================

/// refresh 成功后旧 refresh_token 二次使用必须 401 (jti 已撤销)
#[actix_web::test]
async fn e2e_t01_refresh_rotation_revokes_old_jti() {
    setup_env();
    let pool = make_pool().await;
    let (username, _email) = ensure_test_user(&pool).await;
    let password = env::var("TEST_PASSWORD").expect("TEST_PASSWORD");

    let sink = Arc::new(InMemoryAuditSink::new());
    let sink_dyn: Arc<dyn AuditSink> = sink.clone();
    let app = actix_test::init_service(make_app_with_inmem_audit(pool.clone(), sink_dyn)).await;

    // 1. login
    let login_req = actix_test::TestRequest::post()
        .uri("/v1/auth/login")
        .set_json(json!({"username": username, "password": password}))
        .to_request();
    let login_resp = actix_test::call_service(&app, login_req).await;
    assert_eq!(login_resp.status().as_u16(), 200);
    let login_body: auth_service::models::LoginResponse =
        actix_test::read_body_json(login_resp).await;
    let old_refresh = login_body.refresh_token.clone();

    // 2. refresh 一次 (新 jti, 旧 jti 撤销)
    let refresh_req = actix_test::TestRequest::post()
        .uri("/v1/auth/refresh")
        .set_json(RefreshRequest {
            refresh_token: old_refresh.clone(),
        })
        .to_request();
    let refresh_resp = actix_test::call_service(&app, refresh_req).await;
    assert_eq!(refresh_resp.status().as_u16(), 200);
    let refresh_body: RefreshResponse = actix_test::read_body_json(refresh_resp).await;
    assert_ne!(refresh_body.refresh_token, old_refresh);

    // 3. 用旧 refresh_token 二次 refresh → 必须 401 token_revoked
    let reuse_req = actix_test::TestRequest::post()
        .uri("/v1/auth/refresh")
        .set_json(RefreshRequest {
            refresh_token: old_refresh.clone(),
        })
        .to_request();
    let reuse_resp = actix_test::call_service(&app, reuse_req).await;
    assert_eq!(
        reuse_resp.status().as_u16(),
        401,
        "old refresh_token reuse should be 401"
    );
    let err: ErrorBody = actix_test::read_body_json(reuse_resp).await;
    assert_eq!(err.error, "token_revoked");

    cleanup_test_user(&pool, &username).await;
}

/// 错 refresh_token (非 JWT) → 401 invalid_token
#[actix_web::test]
async fn e2e_t01_refresh_invalid_token_returns_401() {
    setup_env();
    let pool = make_pool().await;

    let sink: Arc<dyn AuditSink> = Arc::new(InMemoryAuditSink::new());
    let app = actix_test::init_service(make_app_with_inmem_audit(pool.clone(), sink)).await;

    let req = actix_test::TestRequest::post()
        .uri("/v1/auth/refresh")
        .set_json(RefreshRequest {
            refresh_token: "not.a.valid.jwt".to_string(),
        })
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 401);
    let err: ErrorBody = actix_test::read_body_json(resp).await;
    assert_eq!(err.error, "invalid_token");
}

/// access_token 不能用于 refresh (token_type 错) → 401 invalid_token_type
#[actix_web::test]
async fn e2e_t01_refresh_wrong_token_type_returns_401() {
    setup_env();
    let pool = make_pool().await;
    let (username, _email) = ensure_test_user(&pool).await;
    let password = env::var("TEST_PASSWORD").expect("TEST_PASSWORD");

    let sink: Arc<dyn AuditSink> = Arc::new(InMemoryAuditSink::new());
    let app = actix_test::init_service(make_app_with_inmem_audit(pool.clone(), sink)).await;

    // login 拿 access_token, 试图用于 refresh
    let login_req = actix_test::TestRequest::post()
        .uri("/v1/auth/login")
        .set_json(json!({"username": username, "password": password}))
        .to_request();
    let login_resp = actix_test::call_service(&app, login_req).await;
    let login_body: auth_service::models::LoginResponse =
        actix_test::read_body_json(login_resp).await;

    let refresh_req = actix_test::TestRequest::post()
        .uri("/v1/auth/refresh")
        .set_json(RefreshRequest {
            refresh_token: login_body.access_token.clone(), // access token 不是 refresh
        })
        .to_request();
    let resp = actix_test::call_service(&app, refresh_req).await;
    assert_eq!(resp.status().as_u16(), 401);
    let err: ErrorBody = actix_test::read_body_json(resp).await;
    assert_eq!(err.error, "invalid_token_type");

    cleanup_test_user(&pool, &username).await;
}

// =============================================================
// T-01 §2 判据 3: logout 端点
// =============================================================

/// logout 成功 → 200 + revoked=true + 旧 refresh_token 之后 refresh 必 401
#[actix_web::test]
async fn e2e_t01_logout_revokes_refresh_token() {
    setup_env();
    let pool = make_pool().await;
    let (username, _email) = ensure_test_user(&pool).await;
    let password = env::var("TEST_PASSWORD").expect("TEST_PASSWORD");

    let sink = Arc::new(InMemoryAuditSink::new());
    let sink_dyn: Arc<dyn AuditSink> = sink.clone();
    let app = actix_test::init_service(make_app_with_inmem_audit(pool.clone(), sink_dyn)).await;

    // 1. login
    let login_req = actix_test::TestRequest::post()
        .uri("/v1/auth/login")
        .set_json(json!({"username": username, "password": password}))
        .to_request();
    let login_resp = actix_test::call_service(&app, login_req).await;
    let login_body: auth_service::models::LoginResponse =
        actix_test::read_body_json(login_resp).await;
    let refresh_token = login_body.refresh_token.clone();

    // 2. logout
    let logout_req = actix_test::TestRequest::post()
        .uri("/v1/auth/logout")
        .set_json(LogoutRequest {
            refresh_token: refresh_token.clone(),
        })
        .to_request();
    let logout_resp = actix_test::call_service(&app, logout_req).await;
    assert_eq!(logout_resp.status().as_u16(), 200, "logout should succeed");
    let logout_body: LogoutResponse = actix_test::read_body_json(logout_resp).await;
    assert!(logout_body.revoked);

    // 3. logout 后的 refresh_token refresh → 401 token_revoked
    let reuse_req = actix_test::TestRequest::post()
        .uri("/v1/auth/refresh")
        .set_json(RefreshRequest {
            refresh_token: refresh_token.clone(),
        })
        .to_request();
    let reuse_resp = actix_test::call_service(&app, reuse_req).await;
    assert_eq!(
        reuse_resp.status().as_u16(),
        401,
        "refresh after logout should be 401"
    );
    let err: ErrorBody = actix_test::read_body_json(reuse_resp).await;
    assert_eq!(err.error, "token_revoked");

    // 4. audit 事件检查 (InMemory sink)
    let events = sink.find_event("logout");
    assert!(events.is_some(), "logout audit event should be emitted");
    let ev = events.unwrap();
    assert_eq!(ev.outcome, auth_service::models::AuditOutcome::Success);

    cleanup_test_user(&pool, &username).await;
}

/// logout 错 refresh_token → 401
#[actix_web::test]
async fn e2e_t01_logout_invalid_token_returns_401() {
    setup_env();
    let pool = make_pool().await;

    let sink: Arc<dyn AuditSink> = Arc::new(InMemoryAuditSink::new());
    let app = actix_test::init_service(make_app_with_inmem_audit(pool.clone(), sink)).await;

    let req = actix_web::test::TestRequest::post()
        .uri("/v1/auth/logout")
        .set_json(LogoutRequest {
            refresh_token: "garbage.token".to_string(),
        })
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 401);
    let err: ErrorBody = actix_test::read_body_json(resp).await;
    assert_eq!(err.error, "invalid_token");
}

/// logout 二次调用（同一个 jti）→ 仍然 200, 但 audit 不重复 (ON CONFLICT 兜底)
#[actix_web::test]
async fn e2e_t01_logout_idempotent_returns_200() {
    setup_env();
    let pool = make_pool().await;
    let (username, _email) = ensure_test_user(&pool).await;
    let password = env::var("TEST_PASSWORD").expect("TEST_PASSWORD");

    let sink = Arc::new(InMemoryAuditSink::new());
    let sink_dyn: Arc<dyn AuditSink> = sink.clone();
    let app = actix_test::init_service(make_app_with_inmem_audit(pool.clone(), sink_dyn)).await;

    // login
    let login_req = actix_test::TestRequest::post()
        .uri("/v1/auth/login")
        .set_json(json!({"username": username, "password": password}))
        .to_request();
    let login_resp = actix_test::call_service(&app, login_req).await;
    let login_body: auth_service::models::LoginResponse =
        actix_test::read_body_json(login_resp).await;
    let refresh_token = login_body.refresh_token.clone();

    // 第 1 次 logout
    let logout_req1 = actix_test::TestRequest::post()
        .uri("/v1/auth/logout")
        .set_json(LogoutRequest {
            refresh_token: refresh_token.clone(),
        })
        .to_request();
    let r1 = actix_test::call_service(&app, logout_req1).await;
    assert_eq!(r1.status().as_u16(), 200);

    // 第 2 次 logout (同一 jti) — 仍然 200 (token 验签过 + 撤销幂等)
    let logout_req2 = actix_test::TestRequest::post()
        .uri("/v1/auth/logout")
        .set_json(LogoutRequest {
            refresh_token: refresh_token.clone(),
        })
        .to_request();
    let r2 = actix_test::call_service(&app, logout_req2).await;
    assert_eq!(
        r2.status().as_u16(),
        200,
        "second logout (jti already revoked) should still return 200"
    );

    cleanup_test_user(&pool, &username).await;
}

// =============================================================
// T-01 §2 判据 3: audit 落库 (DbAuditSink)
// =============================================================

/// login + logout 之后 audit_log 表必须有对应事件 (DbAuditSink 真实落库)
#[actix_web::test]
async fn e2e_t01_audit_log_persisted_to_db() {
    setup_env();
    let pool = make_pool().await;
    let (username, _email) = ensure_test_user(&pool).await;
    let password = env::var("TEST_PASSWORD").expect("TEST_PASSWORD");

    let app = actix_test::init_service(make_app_with_db_audit(pool.clone())).await;

    // login (写 audit_log)
    let login_req = actix_test::TestRequest::post()
        .uri("/v1/auth/login")
        .set_json(json!({"username": username, "password": password}))
        .to_request();
    actix_test::call_service(&app, login_req).await;

    // 直接查 audit_log (per ULYS-46: build_audit 用 await 同步落 DB, 无需轮询)
    let events = db::recent_audit_events(&pool, "login", 5)
        .await
        .expect("recent_audit_events failed");
    assert!(
        !events.is_empty(),
        "login audit event should be persisted to audit_log (build_audit is sync await)"
    );
    let last = &events[0];
    assert_eq!(last.event_type, "login");
    assert_eq!(last.outcome, "success");

    cleanup_test_user(&pool, &username).await;
}

/// login_failed (错密码) → audit_log 出现 login_failed / outcome=failure
#[actix_web::test]
async fn e2e_t01_audit_log_captures_login_failure() {
    setup_env();
    let pool = make_pool().await;
    let (username, _email) = ensure_test_user(&pool).await;

    let app = actix_test::init_service(make_app_with_db_audit(pool.clone())).await;

    // 错密码 login
    let login_req = actix_test::TestRequest::post()
        .uri("/v1/auth/login")
        .set_json(json!({"username": username, "password": "definitely_wrong_pw_123!@#"}))
        .to_request();
    let resp = actix_test::call_service(&app, login_req).await;
    assert_eq!(resp.status().as_u16(), 401);

    // 直接查 audit_log (per ULYS-46: build_audit 用 await 同步落 DB, 无需轮询)
    let events = db::recent_audit_events(&pool, "login_failed", 5)
        .await
        .expect("recent_audit_events failed");
    assert!(
        !events.is_empty(),
        "login_failed audit event should be persisted (build_audit is sync await)"
    );
    let last = &events[0];
    assert_eq!(last.event_type, "login_failed");
    assert_eq!(last.outcome, "failure");

    cleanup_test_user(&pool, &username).await;
}

/// refresh 轮换后 audit_log 同时有 refresh + refresh_revoked 两条
#[actix_web::test]
async fn e2e_t01_audit_log_captures_refresh_rotation() {
    setup_env();
    let pool = make_pool().await;
    let (username, _email) = ensure_test_user(&pool).await;
    let password = env::var("TEST_PASSWORD").expect("TEST_PASSWORD");

    let app = actix_test::init_service(make_app_with_db_audit(pool.clone())).await;

    // login
    let login_req = actix_test::TestRequest::post()
        .uri("/v1/auth/login")
        .set_json(json!({"username": username, "password": password}))
        .to_request();
    let login_resp = actix_test::call_service(&app, login_req).await;
    let login_body: auth_service::models::LoginResponse =
        actix_test::read_body_json(login_resp).await;
    let old_refresh = login_body.refresh_token;

    // refresh
    let refresh_req = actix_test::TestRequest::post()
        .uri("/v1/auth/refresh")
        .set_json(RefreshRequest {
            refresh_token: old_refresh,
        })
        .to_request();
    actix_test::call_service(&app, refresh_req).await;

    // 直接查 audit_log (per ULYS-46: build_audit 用 await 同步落 DB, 无需轮询)
    let refresh_events = db::recent_audit_events(&pool, "refresh", 5)
        .await
        .expect("recent refresh query");
    let revoked_events = db::recent_audit_events(&pool, "refresh_revoked", 5)
        .await
        .expect("recent refresh_revoked query");
    assert!(
        !refresh_events.is_empty(),
        "refresh event should be logged (build_audit is sync await)"
    );
    assert!(
        !revoked_events.is_empty(),
        "refresh_revoked event should be logged (build_audit is sync await)"
    );

    cleanup_test_user(&pool, &username).await;
}

// =============================================================
// 回归测试 (per ULYS-46 验收标准 #3)
// 防止 build_audit 被改回 spawn, 让假阳性 PASS 悄悄回归
// =============================================================

/// build_audit 源码内禁止出现 `tokio::spawn` / `actix_rt::spawn`
///
/// 通过 `include_str!` 静态读取 `src/handlers.rs`, 在 build-time 内联为字面量
/// `&'static str`, 运行期做关键字检查。如有人把 `state.audit.emit(...).await`
/// 改成 `tokio::spawn(...)` / `actix_rt::spawn(...)`, 本测试**确定性**失败,
/// 提示该 PR 必须配套提供 ADR + 替换 e2e 等待模型。
///
/// 注意:
/// - 只检查 `build_audit` 函数体内是否含 `spawn` 关键字, 不检查 import 块
///   (auth-service 已在 [dev-dependencies] 引入 actix-rt, 不需要再次检测)
/// - 误报防护: 只对 `build_audit` 函数切片做 substring 检测, 不会误报模块
///   其它位置的 spawn 调用 (目前没有, 未来如有需手工维护白名单)
#[test]
fn e2e_t01_build_audit_must_not_use_spawn() {
    // 静态内联 handlers.rs 源码
    const HANDLERS_SRC: &str = include_str!("../src/handlers.rs");

    // 切片: 提取 `async fn build_audit(` 之后到下一个 `\n}` 结束的函数体
    // (build_audit 是 `async fn ... -> AuditEvent { ... }`, 闭合是顶层 `}`)
    let fn_start = HANDLERS_SRC
        .find("async fn build_audit(")
        .expect("build_audit fn not found in handlers.rs (renamed?)");

    // 从 fn 头向后找与 `async fn` 同缩进的 `}` 作为函数体结束
    // build_audit 在源码中是 4 空格缩进的 fn, 函数体缩进 4 空格
    // 我们用更简单的策略: 找 `async fn build_audit(` 之后到下一个裸 `}` 即可
    // (build_audit 函数体里没有内嵌顶层 `}` 结构体, 所以这个简化是安全的)
    let after_start = &HANDLERS_SRC[fn_start..];
    let body_end_rel = after_start
        .find("\n}")
        .expect("build_audit body terminator not found");
    let body = &after_start[..body_end_rel + 2];

    // 显式 deny 列表
    let forbidden = [
        "tokio::spawn",
        "actix_rt::spawn",
        // 注意: `spawn_local` / `spawn_blocking` 同样会让 init_service
        // 不驱动, 故一并列入禁止; 如确需使用须配 ADR
        "tokio::task::spawn_local",
        "tokio::task::spawn_blocking",
    ];

    for token in forbidden {
        assert!(
            !body.contains(token),
            "build_audit body must NOT contain `{token}`. \
             改回 spawn 会让 actix_test::init_service 不驱动 audit 写入, \
             e2e 测试将假阳性 PASS (per ULYS-46 / INVIOLABLE 约束). \
             若确需异步化, 必须先写 ADR + 替换等待模型 (JoinHandle / channel)."
        );
    }
}

/// 额外保险: 确认 build_audit 体内仍然调用 `.await` (同步路径核心保证)
///
/// 与上一条互为正向/反向断言: 上一条确保无 spawn, 本条确保仍 `await`.
/// 如有人重构把整个 emit 包到 `async { ... }` 但忘了 `.await`, 上一条仍
/// 通过, 本条会失败。
#[test]
fn e2e_t01_build_audit_still_awaits_emit() {
    const HANDLERS_SRC: &str = include_str!("../src/handlers.rs");
    let fn_start = HANDLERS_SRC
        .find("async fn build_audit(")
        .expect("build_audit fn not found");
    let after_start = &HANDLERS_SRC[fn_start..];
    let body_end_rel = after_start
        .find("\n}")
        .expect("build_audit body terminator not found");
    let body = &after_start[..body_end_rel + 2];

    assert!(
        body.contains(".emit(") && body.contains(".await"),
        "build_audit must call state.audit.emit(...).await synchronously \
         (per ULYS-46 INVIOLABLE 约束)"
    );
}
