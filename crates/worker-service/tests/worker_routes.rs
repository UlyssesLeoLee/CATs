//! worker-service 路由表请求级覆盖
//!
//! 2026-10-07 新增。此前本 crate 只有 `smoke.rs` 的 `name_matches_crate` 与
//! `state.rs` / `scheduler.rs` 的字段级断言，`/healthz`、`/readyz`、
//! `POST /v1/worker/tick` 三条路由**零请求级覆盖**（per BACKEND_STATUS §4.1t
//! 记录的"反向缺口"）。
//!
//! 装配走生产的 `handlers::configure_routes`，不在测试里重抄一份路由 ——
//! 重抄的话 `main.rs` 哪天少注册一条，这里仍然全绿。
//!
//! **不需要真实数据库**：
//! - `/healthz` 无依赖
//! - `/readyz` 的 DB 探活打在连不通的地址，断言的是"失败时如实报告"
//! - `POST /v1/worker/tick` 的鉴权在碰 DB **之前**完成，401/403 两条都能区分
//!   "被鉴权拦下"与"进了业务逻辑但库连不上"（后者固定 500）
//!
//! 真 PostgreSQL 下的 `tick()` 行为验证仍需 CI 的 `e2e (real PostgreSQL)` job。
//!
//! 这里用宏而不是带泛型参数的 helper 函数：`init_service` 返回的
//! `Service<actix_http::Request>`，而 `actix-http` 不是本 crate 的直接依赖，
//! 在签名里写它会解析失败。宏在调用点就地展开，不涉及命名那个类型。

use actix_web::{test as actix_test, web, App};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use worker_service::handlers;
use worker_service::state::AppState;

/// lazy pool: 只解析 URL，不实际建连（与 `state.rs` 同模式）。
/// 端口 1 保证连不通 —— 于是 `/readyz` 必然走 db=fail 分支，
/// `tick()` 必然走到 `claim fail`。
fn dead_pool() -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(200))
        .connect_lazy("postgres://u:p@127.0.0.1:1/none")
        .expect("connect_lazy should not dial")
}

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
    let state = web::Data::new(AppState::new(pool.clone()));
    let pool_data = web::Data::new(pool);
    App::new().configure(move |c| handlers::configure_routes(c, state.clone(), pool_data.clone()))
}

/// GET 并返回 (状态码, JSON)。
macro_rules! get_json {
    ($app:expr, $path:expr) => {{
        let resp = actix_test::call_service(
            &$app,
            actix_test::TestRequest::get().uri($path).to_request(),
        )
        .await;
        let status = resp.status().as_u16();
        let raw = actix_test::read_body(resp).await;
        let body: Value = serde_json::from_slice(&raw).unwrap_or_else(|e| {
            panic!(
                "{} 响应体不是合法 JSON: {e}; raw={}",
                $path,
                String::from_utf8_lossy(&raw)
            )
        });
        (status, body)
    }};
}

/// POST tick，返回 (状态码, body 里的 `error` 码)。调用者须持有至少一个角色。
macro_rules! tick_as {
    ($app:expr, $roles:expr) => {{
        let req = actix_test::TestRequest::post()
            .uri("/v1/worker/tick")
            .insert_header(("X-Cats-User-Id", uuid::Uuid::new_v4().to_string()))
            .insert_header(("X-Cats-Roles", $roles));
        let resp = actix_test::call_service(&$app, req.to_request()).await;
        let status = resp.status().as_u16();
        let raw = actix_test::read_body(resp).await;
        let code = serde_json::from_slice::<Value>(&raw)
            .ok()
            .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
            .unwrap_or_else(|| "<no error field>".to_string());
        (status, code)
    }};
}

// =====================================================================
// 1. /healthz — 统一形状 {status, app:{name, version}}
// =====================================================================
#[actix_web::test]
async fn healthz_returns_200_with_standard_two_key_shape() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    let (status, body) = get_json!(app, "/healthz");

    assert_eq!(status, 200);
    assert_eq!(body["status"], "ok");
    assert_eq!(body["app"]["name"], "worker-service");
    assert_eq!(body["app"]["version"], env!("CARGO_PKG_VERSION"));

    // 顶层**恰好**两个键：多一个键（例如把 bind_addr / upstreams 泄漏进来）
    // 同样会让外部监控的解析器失配。
    let keys: Vec<String> = body
        .as_object()
        .expect("healthz body is object")
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        keys.len(),
        2,
        "healthz 顶层应恰好 status + app 两个键，实得 {keys:?}"
    );
}

// =====================================================================
// 2. /readyz — DB 连不上时必须 503（否则 k8s 会把坏 Pod 判为 Ready）
// =====================================================================
#[actix_web::test]
async fn readyz_returns_503_when_db_unreachable() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    let (status, body) = get_json!(app, "/readyz");

    // 2026-10-07 行为变更：原来这里是 200（只在 body 写 db:"fail"），
    // 而 deploy/k3s/cats-core/worker-service.yaml:33 正是拿 /readyz 当
    // readinessProbe —— 只看状态码，库挂掉的 Pod 会被判 Ready 继续接流量。
    assert_eq!(
        status, 503,
        "DB 连不上时就绪探针必须失败，否则故障静默放大；body={body}"
    );
    // 响应体保持不变，监控仍可同时读状态码和 body。
    assert_eq!(body["status"], "not_ready");
    assert_eq!(body["db"], "fail");
    assert_eq!(body["service"], "worker-service");
}

// =====================================================================
// 3. tick 的鉴权链
// =====================================================================

/// 无 `X-Cats-User-Id` → 401 missing_authorization。
#[actix_web::test]
async fn tick_without_user_id_is_401_missing_authorization() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    let resp = actix_test::call_service(
        &app,
        actix_test::TestRequest::post()
            .uri("/v1/worker/tick")
            .insert_header(("X-Cats-Roles", "Sponsor"))
            .to_request(),
    )
    .await;
    assert_eq!(resp.status().as_u16(), 401);
    let body: Value = serde_json::from_slice(&actix_test::read_body(resp).await).unwrap();
    assert_eq!(body["error"], "missing_authorization");
}

/// `X-Cats-User-Id` 不是合法 UUID → 401 invalid_token。
#[actix_web::test]
async fn tick_with_malformed_user_id_is_401_invalid_token() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    let resp = actix_test::call_service(
        &app,
        actix_test::TestRequest::post()
            .uri("/v1/worker/tick")
            .insert_header(("X-Cats-User-Id", "not-a-uuid"))
            .insert_header(("X-Cats-Roles", "Sponsor"))
            .to_request(),
    )
    .await;
    assert_eq!(resp.status().as_u16(), 401);
    let body: Value = serde_json::from_slice(&actix_test::read_body(resp).await).unwrap();
    assert_eq!(body["error"], "invalid_token");
}

/// 只有 user_id、没有角色头 → 401 unauthorized。
#[actix_web::test]
async fn tick_without_roles_is_401_unauthorized() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    let resp = actix_test::call_service(
        &app,
        actix_test::TestRequest::post()
            .uri("/v1/worker/tick")
            .insert_header(("X-Cats-User-Id", uuid::Uuid::new_v4().to_string()))
            .to_request(),
    )
    .await;
    assert_eq!(resp.status().as_u16(), 401);
    let body: Value = serde_json::from_slice(&actix_test::read_body(resp).await).unwrap();
    assert_eq!(body["error"], "unauthorized");
}

/// 角色名无法解析 → 角色集为空 → 同样 401。
#[actix_web::test]
async fn tick_with_unparseable_role_is_401_unauthorized() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    let (status, code) = tick_as!(app, "NoSuchRole");
    assert_eq!((status, code.as_str()), (401, "unauthorized"));
}

/// **回归用例**：普通业务用户**不得**触发调度。
///
/// 2026-10-07 修复前这里是 **500** —— 原实现把
/// `&[Role::Sponsor, Role::RustLead, Role::SRELead]` 当成"允许的角色"传给了
/// `require_roles`，而该函数的第二个参数语义是"**调用者**的角色"，
/// 于是循环实际在问"角色 Sponsor 有没有 Task:Update"（有，首个迭代即 Ok），
/// 调用者是谁完全不影响结果。任何非空角色头都能进 `tick()`。
#[actix_web::test]
async fn tick_as_plain_user_is_403_not_permitted() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    let (status, code) = tick_as!(app, "User");
    assert_eq!(
        (status, code.as_str()),
        (403, "operation_not_permitted"),
        "User 不在 tick 白名单内，必须 403；实得 {status} {code}"
    );
}

/// **回归用例**：Guest 同样不得触发调度（修复前也是 500）。
#[actix_web::test]
async fn tick_as_guest_is_403_not_permitted() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    let (status, code) = tick_as!(app, "Guest");
    assert_eq!((status, code.as_str()), (403, "operation_not_permitted"));
}

/// DBA Lead 有 Audit/Report 全权但不在 tick 白名单 → 403。
#[actix_web::test]
async fn tick_as_database_lead_is_403_not_permitted() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    let (status, code) = tick_as!(app, "DatabaseLead");
    assert_eq!((status, code.as_str()), (403, "operation_not_permitted"));
}

/// Sponsor 在白名单内 → 鉴权放行，于是请求一路走到 `tick()` 并因死库失败。
/// 500 是这里的**正确**期望：它证明鉴权没有拦住它。
#[actix_web::test]
async fn tick_as_sponsor_passes_authz_and_reaches_scheduler() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    let (status, code) = tick_as!(app, "Sponsor");
    assert_eq!(
        (status, code.as_str()),
        (500, "internal_error"),
        "Sponsor 应通过鉴权并进入 tick()，随后因库不可达失败"
    );
}

/// RustLead 在白名单内但**没有** `Task:Update` 权限 —— 它仍然应当放行。
///
/// 这条用例把"判定依据是白名单"与"判定依据是权限矩阵"区分开：若改回用
/// `require_roles(&checker, &caller_roles, Task, Update)`，本用例会变成 403。
#[actix_web::test]
async fn tick_as_rustlead_passes_allowlist_even_without_task_update_perm() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    let (status, code) = tick_as!(app, "RustLead");
    assert_eq!(
        (status, code.as_str()),
        (500, "internal_error"),
        "RustLead 在白名单内，判定依据应是白名单而非权限矩阵"
    );
}

/// 多角色里只要含白名单角色即放行（`User,Sponsor` → 进 tick()）。
#[actix_web::test]
async fn tick_accepts_caller_holding_any_one_allowlisted_role() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    let (status, code) = tick_as!(app, "User,Sponsor");
    assert_eq!((status, code.as_str()), (500, "internal_error"));
}

/// 角色顺序不影响判定（`Guest,User` 与 `User,Guest` 同结果，且都被拒）。
#[actix_web::test]
async fn tick_decision_is_independent_of_role_order() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    let guest_first = tick_as!(app, "Guest,User");
    let user_first = tick_as!(app, "User,Guest");
    assert_eq!(
        guest_first, user_first,
        "同一组角色仅顺序不同，判定必须相同"
    );
    assert_eq!(guest_first.0, 403);
}

/// 未注册的路由必须是 404 —— 顺带证明 `configure_routes` 没有多注册东西。
#[actix_web::test]
async fn unknown_route_is_404() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    let resp = actix_test::call_service(
        &app,
        actix_test::TestRequest::get()
            .uri("/v1/worker/nope")
            .to_request(),
    )
    .await;
    assert_eq!(resp.status().as_u16(), 404);
}
