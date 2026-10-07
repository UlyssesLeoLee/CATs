//! report-service 路由表请求级覆盖
//!
//! 2026-10-07 新增。此前本 crate 只有 `smoke.rs` 的 `name_matches_crate` 与
//! `handlers.rs` 里 4 个纯字段级单元测试（Query DTO 构造 / ErrorBody 序列化），
//! **4 条路由零请求级覆盖** —— 没有任何一个请求真的打过这些 handler。
//! 装配走生产的 `handlers::configure_routes`，不在测试里重抄一份路由。
//!
//! **不需要真实数据库**：三个报表端点的执行顺序是
//! `RBAC 鉴权 → from < to 校验 → 才查库`，所以前两级都能在死库下区分开：
//! - 401/403：鉴权拦下
//! - 400 `invalid_request`：鉴权放行、参数校验拦下
//! - 500 `server_error`：两级都过了，卡在 DB
//!
//! 真 PostgreSQL 下的聚合结果验证仍需 CI 的 `e2e (real PostgreSQL)` job。
//!
//! 用宏而不是带泛型参数的 helper：`init_service` 返回的是
//! `Service<actix_http::Request>`，而 `actix-http` 不是本 crate 的直接依赖。

use actix_web::{test as actix_test, web, App};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::Arc;

use cats_rbac::RbacChecker;
use report_service::handlers;

const ORG: &str = "11111111-1111-1111-1111-111111111111";
const PROJECT: &str = "22222222-2222-2222-2222-222222222222";
const WORKSPACE: &str = "33333333-3333-3333-3333-333333333333";
const FROM_OK: &str = "2026-09-01T00:00:00Z";
const TO_OK: &str = "2026-09-30T23:59:59Z";

/// lazy pool: 只解析 URL，不实际建连。端口 1 保证连不通。
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
    let pool_data = web::Data::new(pool);
    let rbac_data = web::Data::new(Arc::new(RbacChecker::new()));
    App::new()
        .configure(move |c| handlers::configure_routes(c, pool_data.clone(), rbac_data.clone()))
}

/// 构造一条合法路径（from < to），便于把"鉴权"与"参数校验"分开测。
fn path(kind: &str) -> String {
    let (key, id) = match kind {
        "usage" => ("org_id", ORG),
        "translation-volume" => ("project_id", PROJECT),
        _ => ("workspace_id", WORKSPACE),
    };
    format!("/v1/reports/{kind}?{key}={id}&from={FROM_OK}&to={TO_OK}")
}

/// 发一个 GET，带可选的 RBAC 头，返回 (状态码, JSON)。
macro_rules! call {
    ($app:expr, $uri:expr, $headers:expr) => {{
        let mut req = actix_test::TestRequest::get().uri($uri);
        for (k, v) in $headers {
            req = req.insert_header((k.to_string(), v.to_string()));
        }
        let resp = actix_test::call_service(&$app, req.to_request()).await;
        let status = resp.status().as_u16();
        let raw = actix_test::read_body(resp).await;
        let body: Value = serde_json::from_slice(&raw).unwrap_or_else(|e| {
            panic!(
                "{} 响应体不是合法 JSON: {e}; raw={}",
                $uri,
                String::from_utf8_lossy(&raw)
            )
        });
        (status, body)
    }};
}

/// RBAC 头：`X-Cats-User-Id` + `X-Cats-Roles`（多角色逗号分隔）。
///
/// 2026-10-07 契约统一。本文件初稿用的是 `Authorization: Bearer cats-role:<roles>`，
/// 那是本 crate 与另外 4 个服务（file / notification / project / task）各自的旧契约；
/// 现在七个服务一律走 `X-Cats-*`，由边缘 envoy 在验签 JWT 后注入。
macro_rules! as_role {
    ($r:expr) => {
        vec![
            (
                "X-Cats-User-Id",
                "00000000-0000-0000-0000-000000000001".to_string(),
            ),
            ("X-Cats-Roles", $r.to_string()),
        ]
    };
}

const REPORT_PATHS: [&str; 3] = ["usage", "translation-volume", "audit-summary"];

// =====================================================================
// 1. /healthz
// =====================================================================
#[actix_web::test]
async fn healthz_returns_200_with_standard_two_key_shape() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    let (status, body) = call!(app, "/healthz", Vec::<(&str, String)>::new());

    assert_eq!(status, 200);
    assert_eq!(body["status"], "ok");
    assert_eq!(body["app"]["name"], "report-service");
    assert_eq!(body["app"]["version"], env!("CARGO_PKG_VERSION"));
    let keys: Vec<String> = body.as_object().expect("object").keys().cloned().collect();
    assert_eq!(keys.len(), 2, "healthz 顶层应恰好 2 键，实得 {keys:?}");
}

/// 未注册路由必须是 404 —— 顺带证明 `configure_routes` 没有多注册东西。
#[actix_web::test]
async fn unknown_route_is_404() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    let resp = actix_test::call_service(
        &app,
        actix_test::TestRequest::get()
            .uri("/v1/reports/nope")
            .to_request(),
    )
    .await;
    assert_eq!(resp.status().as_u16(), 404);
}

// =====================================================================
// 2. 鉴权：三条报表路由都必须挡在未认证 / 无权限的调用方前面
// =====================================================================

/// 无任何 RBAC 头 → 401（此时 query 参数是合法的，确保 401 来自鉴权而不是解析）。
#[actix_web::test]
async fn report_endpoints_reject_missing_credentials_with_401() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    for kind in REPORT_PATHS {
        let (status, body) = call!(app, &path(kind), Vec::<(&str, String)>::new());
        assert_eq!(status, 401, "{kind} 缺凭据应 401，实得 {status} {body}");
        assert_eq!(body["error"], "missing_authorization", "{kind}");
    }
}

/// **回归用例**：旧契约 `Authorization: Bearer cats-role:*` 必须彻底失效。
///
/// 2026-10-07 之前本 crate 认的就是这个格式，但它**没有任何一方在生产** ——
/// `deploy/envoy-mvp.yaml` 里既没有 `jwt_authn` 也没有 `request_headers_to_add`。
/// 结果是：正常流量（真 JWT）全部 401，而调用方自填
/// `Authorization: Bearer cats-role:Sponsor` 反而能冒充 Sponsor。
///
/// 现在契约统一到 `X-Cats-*`（边缘验签后注入），所以只带 `Authorization`
/// 的请求一律匿名 —— 包括旧格式里那个字面量 `cats-role:Sponsor`。
#[actix_web::test]
async fn legacy_cats_role_authorization_is_no_longer_accepted() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    for bad in [
        "Bearer cats-role:Sponsor",
        "Bearer some-jwt-looking-token",
        "Basic abc",
        "cats-role:Sponsor",
    ] {
        let headers = vec![("Authorization", bad.to_string())];
        let (status, body) = call!(app, &path("usage"), headers);
        assert_eq!(
            status, 401,
            "{bad:?} 不该再被当作凭据（它会让人冒充 Sponsor），实得 {status} {body}"
        );
        assert_eq!(body["error"], "missing_authorization", "{bad:?}");
    }
}

/// 普通业务用户没有 Report:Read → 403。
///
/// 这三条用例是**鉴权真的挂在 handler 上**的证据：如果把
/// `rbac::enforce(...)` 从某个 handler 里删掉，对应用例会从 403 变成 500。
#[actix_web::test]
async fn report_endpoints_reject_plain_user_with_403() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    for kind in REPORT_PATHS {
        let (status, body) = call!(app, &path(kind), as_role!("User"));
        assert_eq!(status, 403, "{kind} 用 User 应 403，实得 {status} {body}");
    }
}

// =====================================================================
// 3. 参数校验：`from >= to` 必须在**鉴权之后、查库之前**被拒
// =====================================================================

/// Sponsor 放行后，`from >= to` → 400 invalid_request（而不是 500）。
///
/// 400 说明请求根本没走到 DB：鉴权过了、参数校验拦下了。
#[actix_web::test]
async fn report_endpoints_reject_inverted_range_with_400() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    for kind in REPORT_PATHS {
        let key = match kind {
            "usage" => "org_id",
            "translation-volume" => "project_id",
            _ => "workspace_id",
        };
        let id = match kind {
            "usage" => ORG,
            "translation-volume" => PROJECT,
            _ => WORKSPACE,
        };
        // from == to（相等也算非法）
        let uri = format!("/v1/reports/{kind}?{key}={id}&from={FROM_OK}&to={FROM_OK}");
        let (status, body) = call!(app, &uri, as_role!("Sponsor"));
        assert_eq!(
            status, 400,
            "{kind} from==to 应 400（且不得碰 DB），实得 {status} {body}"
        );
        assert_eq!(body["error"], "invalid_request", "{kind}");
        assert!(body["message"].as_str().unwrap().contains("before"));
    }
}

/// 缺必填 query 参数 → 400（actix 的 Query extractor 在进 handler 前就拒了）。
#[actix_web::test]
async fn report_endpoints_reject_missing_query_param_with_400() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    let uri = format!("/v1/reports/usage?from={FROM_OK}&to={TO_OK}");
    let resp = actix_test::call_service(
        &app,
        actix_test::TestRequest::get()
            .uri(&uri)
            .insert_header(("X-Cats-User-Id", "00000000-0000-0000-0000-000000000001"))
            .insert_header(("X-Cats-Roles", "Sponsor"))
            .to_request(),
    )
    .await;
    assert_eq!(resp.status().as_u16(), 400, "缺 org_id 应 400");
}

// =====================================================================
// 4. 放行到底：Sponsor + 合法区间 → 500（卡在 DB），证明两级闸门都没误伤
// =====================================================================

/// Sponsor 放行、区间合法 → 500 server_error（死库）。
///
/// 500 是这里的**正确**期望：它同时证明「鉴权放行了」且「参数校验放行了」。
#[actix_web::test]
async fn report_endpoints_reach_db_when_authz_and_validation_pass() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    for kind in REPORT_PATHS {
        let (status, body) = call!(app, &path(kind), as_role!("Sponsor"));
        assert_eq!(
            status, 500,
            "{kind} 应通过鉴权与参数校验后卡在 DB，实得 {status} {body}"
        );
        assert_eq!(body["error"], "server_error", "{kind}");
    }
}

/// 角色顺序不影响判定（`User,Sponsor` 与 `Sponsor,User` 同结果）。
#[actix_web::test]
async fn authz_decision_is_independent_of_role_order() {
    let app = actix_test::init_service(make_app(dead_pool())).await;
    let a = call!(app, &path("usage"), as_role!("User,Sponsor"));
    let b = call!(app, &path("usage"), as_role!("Sponsor,User"));
    assert_eq!(a.0, b.0, "同一组角色仅顺序不同，状态码必须相同");
    assert_eq!(a.0, 500, "含 Sponsor 即应放行到 DB");
}
