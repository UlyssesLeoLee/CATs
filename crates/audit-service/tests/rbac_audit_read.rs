//! audit-service 端点的授权行为 —— **真驱动 handler**，不是只测矩阵
//!
//! ## 为什么是这个形状
//!
//! 第一版（本文件初稿）只直接调 `require_roles(&checker, &[Role::X], ...)`
//! 然后断言 Ok/Err。它有 7 个用例、全绿，但**鉴别力接近零**：如果有人把
//! `handlers::list_audit_logs` 里的 `Resource::Audit` 改成 `Resource::Project`、
//! 或者干脆把整个 `require_roles` 调用删掉，这 7 条用例**依然全绿**。
//! 它验证的是 cats-rbac 的矩阵快照，跟 audit-service 的 handler 无关。
//!
//! 现在改成用 `actix_web::test` 真的把 HTTP 请求打进 handler，并且
//! **路由表直接取自 `handlers::configure`** —— 那是 `main.rs` 用的同一个函数。
//! 这样"服务对外暴露什么"只有一个事实来源，测试不可能验证到别的东西。
//!
//! ## 被钉住的行为
//!
//! | 场景 | 期望 | 依据 |
//! |---|---|---|
//! | 无 `X-Cats-User-Id` | 401 `missing_authorization` | `extract_user_id_and_roles` |
//! | user_id 非 UUID | 401 `invalid_token` | 同上 |
//! | 有 user_id 无 roles | 401 `unauthorized` | 同上 |
//! | User 读审计 | 403 `operation_not_permitted` | 矩阵：User 的 Read 不含 Audit |
//! | Guest 读审计 | **401** `unauthenticated` | `Role::Guest` 定义即「未登录」，单独判定时短路（lib.rs:404）|
//! | `User,Guest` → 401 / `Guest,User` → 403 | 同组角色**仅顺序不同** | 已知瑕疵：`require_roles` 只保留最后一个错误 |
//! | `Guest,DatabaseLead` | **放行** | Guest 不是集合级否决；DatabaseLead 单独够格即 `return Ok` |
//! | DatabaseLead 读审计 | **放行**，落到 DB 层 | 矩阵：DatabaseLead 对 Audit/Report 有 Read/Audit/Approve |
//! | 无凭据 POST `/test` | 401 | 2026-10-05 补的鉴权，此前该端点**零校验** |
//!
//! ## 关于"放行"怎么断言
//!
//! 放行路径会真的去查库，而测试连的是 `127.0.0.1:1`（仓库既有约定：一个
//! 确定关闭的端口，`connect_lazy` 不拨号，第一次查询立刻 ECONNREFUSED）。
//! 所以断言的是**它走到了哪一层**，而不是成功：
//!
//! ```text
//! 401 / 403  → 被鉴权挡在门外（没碰到 DB）
//! 500 internal_error → 过了鉴权、进了 DB、DB 不可达
//! ```
//!
//! 如果哪天有人删掉 handler 里的 `require_roles`，DatabaseLead 那条会从
//! 500 变成 500（还是 internal_error）——所以反过来更有价值的是：**User 那条
//! 从 403 变成 500**，这正是"鉴权被摘掉"的信号。
//!
//! 依据: doc/05-其他/管理/CATs_权限决策书_v1.0.md §3（角色 × 资源 × 动作）
//! 实现: `crates/cats-rbac/src/lib.rs` `default_permissions()`

use actix_web::{test, web, App};
use audit_service::handlers;
use audit_service::state::AppState;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

/// 一个确定关闭的端口。`connect_lazy` 只解析 URL 不拨号，
/// 真正查询时才 ECONNREFUSED —— 于是"到达 DB 层"和"被鉴权挡住"可区分。
const DEAD_URL: &str = "postgres://invalid:invalid@127.0.0.1:1/invalid";

/// app_data 里的两样东西：`handlers` 与 `main.rs` 注册的完全一致。
fn fixtures() -> (web::Data<AppState>, web::Data<sqlx::PgPool>) {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy(DEAD_URL)
        .expect("connect_lazy must not dial");
    (
        web::Data::new(AppState::new(pool.clone())),
        web::Data::new(pool),
    )
}

/// 一组够格的凭据（矩阵里 DatabaseLead 对 Audit/Report 有 Read/Audit/Approve）
fn db_lead_headers() -> (String, String) {
    (uuid::Uuid::new_v4().to_string(), "DatabaseLead".to_string())
}

fn list_uri() -> String {
    format!("/v1/audit-logs?org_id={}", uuid::Uuid::new_v4())
}

// ------------------------------------------------------------------
// 探活
// ------------------------------------------------------------------

/// `/healthz` 必须自报**本 crate**。
///
/// 这一条是回归钉子：本轮刚修过 10 处 `/healthz` 自报 `"cats-common"` 的
/// 问题（其他 crate 的包名被硬编码进去）。这里断言 `env!("CARGO_PKG_NAME")`
/// 真的落到了响应体里 —— 改错了会红。
#[actix_web::test]
async fn healthz_reports_this_crate_not_a_shared_one() {
    let (state, pool) = fixtures();
    let app = test::init_service(
        App::new()
            .app_data(state)
            .app_data(pool)
            .configure(handlers::configure),
    )
    .await;

    let resp = test::call_service(&app, test::TestRequest::get().uri("/healthz").to_request()).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(
        body["app"]["name"], "audit-service",
        "/healthz 必须自报 audit-service —— 若变成 \"cats-common\"，\
         说明有人改回了硬编码的其它 crate 名"
    );
    assert_eq!(body["status"], "ok");
}

// ------------------------------------------------------------------
// 凭据校验（extract_user_id_and_roles）
// ------------------------------------------------------------------

/// 缺 `X-Cats-User-Id` → 401
#[actix_web::test]
async fn list_without_user_id_header_is_401() {
    let (state, pool) = fixtures();
    let app = test::init_service(
        App::new()
            .app_data(state)
            .app_data(pool)
            .configure(handlers::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&list_uri())
        .insert_header(("X-Cats-Roles", "Sponsor"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "missing_authorization");
}

/// `X-Cats-User-Id` 不是合法 UUID → 401 invalid_token
#[actix_web::test]
async fn list_with_malformed_user_id_is_401() {
    let (state, pool) = fixtures();
    let app = test::init_service(
        App::new()
            .app_data(state)
            .app_data(pool)
            .configure(handlers::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&list_uri())
        .insert_header(("X-Cats-User-Id", "not-a-uuid"))
        .insert_header(("X-Cats-Roles", "Sponsor"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "invalid_token");
}

/// 有 user_id 但 roles 解析后为空 → 401 unauthorized
///
/// 注意 `X-Cats-Roles` 传的是**一个不存在的角色名**而不是空串：
/// `parse_role` 会把它过滤掉，`roles.is_empty()` 成立 —— 这是真实网关可能
/// 发出的情况（JWT 里带了个本服务不认识的角色）。
#[actix_web::test]
async fn list_with_unparseable_roles_is_401() {
    let (state, pool) = fixtures();
    let app = test::init_service(
        App::new()
            .app_data(state)
            .app_data(pool)
            .configure(handlers::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&list_uri())
        .insert_header(("X-Cats-User-Id", uuid::Uuid::new_v4().to_string()))
        .insert_header(("X-Cats-Roles", "NoSuchRole"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "unauthorized");
}

/// 缺 `org_id` 查询参数 → 400（`web::Query<ListQuery>` 的契约，handler 压根没跑）
#[actix_web::test]
async fn list_without_org_id_is_400() {
    let (state, pool) = fixtures();
    let app = test::init_service(
        App::new()
            .app_data(state)
            .app_data(pool)
            .configure(handlers::configure),
    )
    .await;

    let (uid, roles) = db_lead_headers();
    let req = test::TestRequest::get()
        .uri("/v1/audit-logs")
        .insert_header(("X-Cats-User-Id", uid))
        .insert_header(("X-Cats-Roles", roles))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}

// ------------------------------------------------------------------
// 权限矩阵 —— 拒绝侧
// ------------------------------------------------------------------

/// 普通 User 的 Read 范围**不含 Audit** → 403
///
/// 这是"鉴权真的挂在 handler 上"的关键证据：如果有人把
/// `require_roles(...)` 从 `list_audit_logs` 里删掉，这条会从 403 变成 500。
#[actix_web::test]
async fn list_as_plain_user_is_403() {
    let (state, pool) = fixtures();
    let app = test::init_service(
        App::new()
            .app_data(state)
            .app_data(pool)
            .configure(handlers::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&list_uri())
        .insert_header(("X-Cats-User-Id", uuid::Uuid::new_v4().to_string()))
        .insert_header(("X-Cats-Roles", "User"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "operation_not_permitted");
}

/// Guest 读审计 → **401**（不是 403）
///
/// 2026-10-05 首版这里断言的是 403，实测拿到 401。查下来**是断言写错了，
/// 不是代码错了**：`cats-rbac/src/lib.rs:404` 在权限循环之前就短路了
///
/// ```text
/// if user_roles.is_empty() || user_roles.contains(&Role::Guest) {
///     return Err(RbacError::Unauthenticated);   // → 401
/// }
/// ```
///
/// `Role::Guest` 的定义注释就写着「未登录」（lib.rs:53），且这个语义在
/// 全仓库是一致的：report / project / task / notification / file 五个
/// service 的 `AuthContext::is_authenticated()` 都是
/// `!roles.is_empty() && !roles.contains(&Role::Guest)`，cats-rbac 自己也
/// 有 `test_guest_cannot_access_user_resource` 断言 `Unauthenticated`。
/// 所以 Guest 拿 401（未登录）而不是 403（登录了但没权限）是对的。
#[actix_web::test]
async fn list_as_guest_is_401_unauthenticated() {
    let (state, pool) = fixtures();
    let app = test::init_service(
        App::new()
            .app_data(state)
            .app_data(pool)
            .configure(handlers::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&list_uri())
        .insert_header(("X-Cats-User-Id", uuid::Uuid::new_v4().to_string()))
        .insert_header(("X-Cats-Roles", "Guest"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "unauthorized");
}

/// `User,Guest` → 401（两个角色单独都不够格，且**最后一个**错误来自 Guest）
///
/// ## 这里的 401 是个「顺序产物」，不是设计意图
///
/// `require_roles`（cats-rbac/src/service_helpers.rs:104）把角色集合
/// **拆开、逐个单独判定**：
///
/// ```text
/// for role in allowed_roles {
///     match checker.check_roles(&[*role], resource, action).await { ... }
/// }
/// ```
///
/// 对 `User,Guest`：
///   1. `check_roles(&[User])` → Forbidden   （User 的 Read 不含 Audit）
///   2. `check_roles(&[Guest])` → Unauthenticated（Guest = 未登录）
///   3. 两个都不过 → 取**最后一个**错误 = Unauthenticated → 401
///
/// 只要把顺序反过来（`Guest,User`），最后留下的错误就变成 Forbidden → **403**。
/// 下面紧跟一条用例把这个顺序敏感性钉住。
#[actix_web::test]
async fn list_as_user_plus_guest_is_401() {
    let (state, pool) = fixtures();
    let app = test::init_service(
        App::new()
            .app_data(state)
            .app_data(pool)
            .configure(handlers::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&list_uri())
        .insert_header(("X-Cats-User-Id", uuid::Uuid::new_v4().to_string()))
        .insert_header(("X-Cats-Roles", "User,Guest"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

/// 同一组角色、换个顺序 → **403** 而不是 401
///
/// 与上一条构成对照，证明状态码取决于 `X-Cats-Roles` 里的**顺序**：
/// `require_roles` 只保留最后一次迭代的错误。
///
/// 这是一个**已知的实现瑕疵**，不是设计意图。两个后果：
///   - 同一个用户、同样的权限，HTTP 状态码可能随 header 顺序变化；
///   - 401（未登录）对一个其实已登录的调用方是误导性的，会把人引去查
///     认证链而不是权限矩阵。
///
/// 本用例是**特征化测试**：它记录当前行为，让漂移可见。若将来有人修
/// `require_roles`（例如「Forbidden 优先于 Unauthenticated」），这条会红，
/// 那时应当**连同注释一起**改掉，而不是让测试自动跟着变。
#[actix_web::test]
async fn list_as_guest_plus_user_is_403_same_roles_reversed_order() {
    let (state, pool) = fixtures();
    let app = test::init_service(
        App::new()
            .app_data(state)
            .app_data(pool)
            .configure(handlers::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&list_uri())
        .insert_header(("X-Cats-User-Id", uuid::Uuid::new_v4().to_string()))
        .insert_header(("X-Cats-Roles", "Guest,User"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        403,
        "Guest,User 与 User,Guest 是一组相同的角色，只是顺序不同；\
         本用例记录 require_roles 只保留最后一个错误导致的状态码差异"
    );
}

/// `Guest,DatabaseLead` → **放行**（Guest 不构成否决）
///
/// 这条纠正了本文件第二版的一个错误判断。当时以为「集合里有 Guest 就短路
/// 成 401」，实测是放行 —— 因为 `require_roles` 把集合拆开逐个单独判定，
/// DatabaseLead 单独就满足 Audit/Read，于是直接 `return Ok(())`，
/// **根本没走到 Guest 那一轮**。
///
/// 换句话说 `check_roles` 里那句 Guest 短路（lib.rs:404）只在
/// 「单独检查 Guest」时生效，它**不是集合级否决**。
#[actix_web::test]
async fn list_as_guest_plus_database_lead_passes_authz_guest_is_not_a_veto() {
    let (state, pool) = fixtures();
    let app = test::init_service(
        App::new()
            .app_data(state)
            .app_data(pool)
            .configure(handlers::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&list_uri())
        .insert_header(("X-Cats-User-Id", uuid::Uuid::new_v4().to_string()))
        .insert_header(("X-Cats-Roles", "Guest,DatabaseLead"))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_ne!(resp.status(), 401, "DatabaseLead 单独就够格，不该被 401");
    assert_ne!(resp.status(), 403, "同上，不该被判 Forbidden");
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(
        body["error"], "internal_error",
        "放行后应当走到 db::list_by_org 并因 DEAD_URL 失败"
    );
}

// ------------------------------------------------------------------
// 权限矩阵 —— 放行侧
// ------------------------------------------------------------------

/// DatabaseLead 对 Audit 有 Read → 过鉴权，落到 DB 层（此处 DB 不可达 → 500）
#[actix_web::test]
async fn list_as_database_lead_passes_authz() {
    let (state, pool) = fixtures();
    let app = test::init_service(
        App::new()
            .app_data(state)
            .app_data(pool)
            .configure(handlers::configure),
    )
    .await;

    let (uid, roles) = db_lead_headers();
    let req = test::TestRequest::get()
        .uri(&list_uri())
        .insert_header(("X-Cats-User-Id", uid))
        .insert_header(("X-Cats-Roles", roles))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_ne!(
        resp.status(),
        401,
        "DatabaseLead 具备 Audit/Read，不该被凭据校验挡下"
    );
    assert_ne!(
        resp.status(),
        403,
        "DatabaseLead 对 Audit/Report 有 Read/Audit/Approve，\
         若被判 403 说明矩阵或 handler 的资源/动作对不上"
    );

    // 500 + internal_error = 已过鉴权、进了 DB 层、DB 不可达（DEAD_URL）
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(
        body["error"], "internal_error",
        "放行路径应当走到 db::list_by_org 并因 DEAD_URL 失败；\
         实际 error={} —— 若不是 internal_error，说明它没到 DB 层",
        body["error"]
    );
}

/// `User,DatabaseLead` 多角色 → 任一命中即放行（与上面 `User,Guest` 构成对照）
#[actix_web::test]
async fn list_as_user_plus_database_lead_passes_authz() {
    let (state, pool) = fixtures();
    let app = test::init_service(
        App::new()
            .app_data(state)
            .app_data(pool)
            .configure(handlers::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&list_uri())
        .insert_header(("X-Cats-User-Id", uuid::Uuid::new_v4().to_string()))
        .insert_header(("X-Cats-Roles", "User,DatabaseLead"))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_ne!(resp.status(), 403);
    assert_ne!(resp.status(), 401);
}

// ------------------------------------------------------------------
// POST /v1/audit-logs/test —— 2026-10-05 补的鉴权
// ------------------------------------------------------------------

/// **无凭据写审计表 → 401**
///
/// 这条在补鉴权之前会走到 DB 层返回 500。它是那处修复是否真的生效的证据：
/// 一个审计服务如果留着一个免鉴权的写入口，审计记录本身就可以被伪造。
#[actix_web::test]
async fn test_ingest_without_credentials_is_401() {
    let (state, pool) = fixtures();
    let app = test::init_service(
        App::new()
            .app_data(state)
            .app_data(pool)
            .configure(handlers::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/v1/audit-logs/test")
        .set_json(serde_json::json!({ "hello": "world" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        401,
        "免凭据就能往 audit_logs 写行 = 审计记录可被伪造"
    );
}

/// 普通 User 连写审计的权限也没有（`Action::Audit` 比 Read 更严）→ 403
#[actix_web::test]
async fn test_ingest_as_plain_user_is_403() {
    let (state, pool) = fixtures();
    let app = test::init_service(
        App::new()
            .app_data(state)
            .app_data(pool)
            .configure(handlers::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/v1/audit-logs/test")
        .insert_header(("X-Cats-User-Id", uuid::Uuid::new_v4().to_string()))
        .insert_header(("X-Cats-Roles", "User"))
        .set_json(serde_json::json!({ "hello": "world" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
}

/// DatabaseLead 对 Audit 有 Audit 权限 → 过鉴权，落到 DB 层
#[actix_web::test]
async fn test_ingest_as_database_lead_passes_authz() {
    let (state, pool) = fixtures();
    let app = test::init_service(
        App::new()
            .app_data(state)
            .app_data(pool)
            .configure(handlers::configure),
    )
    .await;

    let (uid, roles) = db_lead_headers();
    let req = test::TestRequest::post()
        .uri("/v1/audit-logs/test")
        .insert_header(("X-Cats-User-Id", uid))
        .insert_header(("X-Cats-Roles", roles))
        .set_json(serde_json::json!({ "hello": "world" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_ne!(resp.status(), 401);
    assert_ne!(
        resp.status(),
        403,
        "DatabaseLead 具备 Audit/Audit，不该被拒"
    );
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "internal_error");
}
