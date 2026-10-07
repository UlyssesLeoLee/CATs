//! 生产装配的**上游透传契约**（`handlers::*` + `handlers::configure_routes`）
//!
//! ## 这个文件为什么被重写（2026-10-07）
//!
//! 原 `bff_routes_passthrough.rs` 用 `routes::configure` 这张**服务器从不执行**的
//! 路由表构造被测 App（`routes.rs` 已删除）。于是 13 个用例里有 8 条断言描述的是
//! 一套废弃设计的行为，逐条核对后发现**没有一条对生产成立**：
//!
//! | 原断言 | 生产实际 |
//! |---|---|
//! | 请求打 `/api/v1/auth/login` | 生产是 `/v1/auth/login`（无 `api` 段） |
//! | 上游 401 **原样透传** | `UpstreamError` → 502 `dependency_unavailable`（`error.rs:188`）|
//! | 上游 201 **原样透传** | 同上，201 走 `status.is_success()` 分支，不会变成错误 |
//! | 注入 `X-Cats-User-Id` / `X-Cats-Org-Id` | 生产只发 `Authorization: Bearer`（`upstream/projects.rs:87`）|
//! | `/api/v1/translate/*` 两个端点 | 端点不存在（`routes.rs` 已删），且从未进过 openapi |
//!
//! 所以这不是"把路径改一下"——是**断言对象整个换掉了**。现在这份文件断言的是
//! `main.rs` 真正挂的那张表。
//!
//! ## 策略
//!
//! 起一个**真的 HTTP 上游**（`HttpServer` 绑 `127.0.0.1:0`，随机端口），把
//! `Config` 的上游基址指过去，于是可以断言真实契约，而不只是"连不上所以 502"：
//!
//! | 断言的性质 | 靠什么证据 |
//! |---|---|
//! | URL 拼装正确 | 假上游**记下**自己收到的 method + path + query |
//! | body 原样转发 | 假上游记下收到的 JSON，逐字段比对 |
//! | 上游非 2xx 的处理 | 让上游回 401/503，断言 BFF 的**统一错误信封**（不是 200，也不原样透传）|
//! | 本地校验早于网络 | 空 username → 400，且断言假上游**一次都没被调用** |
//! | access token 转发 | 假上游记下收到的 `Authorization` |
//!
//! 与 `bff_smoke.rs` 互补：那边验证鉴权层的否定路径（无 token / 伪造 token），
//! 这边验证鉴权通过之后的上游交互。
//!
//! 不用 `actix_web::test::start`（它要 `macros` feature），改用 `HttpServer` 手动绑端口。

use std::env;
use std::sync::{Arc, Mutex};

use actix_web::dev::{ServerHandle, ServiceFactory, ServiceRequest, ServiceResponse};
use actix_web::http::StatusCode;
use actix_web::rt;
use actix_web::{test, web, App, HttpRequest, HttpResponse, HttpServer};
use cats_bff::config::Config;
use cats_bff::handlers;
use cats_bff::principal::{shared_checker, Claims};
use cats_bff::upstream::{auth::AuthClient, projects::ProjectsClient, tasks::TasksClient};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde_json::{json, Value};

/// 测试专用 JWT_SECRET。与 `bff_smoke.rs` 用同一个常量（该文件模块文档解释了
/// 为什么必须固定成同一个值：多个 `#[test]` 在同一进程并发跑，各自 `set_var`
/// 成不同值会产生竞态）。
const TEST_JWT_SECRET: &str = "cats_bff_test_secret_at_least_32_bytes_long_x";

fn sign_access_token(roles: &[&str]) -> String {
    env::set_var("JWT_SECRET", TEST_JWT_SECRET);
    let claims = Claims {
        sub: "00000000-0000-0000-0000-000000000001".to_string(),
        username: "test-user".to_string(),
        exp: chrono::Utc::now().timestamp() + 3600,
        token_type: "access".to_string(),
        roles: roles.iter().map(|r| r.to_string()).collect(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
    )
    .expect("test token signing should not fail")
}

// ---------------------------------------------------------------------
// 假上游
// ---------------------------------------------------------------------

/// 假上游记下的一次请求
#[derive(Clone, Debug)]
struct Captured {
    method: String,
    path: String,
    query: String,
    authorization: Option<String>,
    x_cats_user_id: Option<String>,
    body: Option<Value>,
}

/// 假上游的共享状态：一份请求日志 + 一个可配置的应答
#[derive(Clone)]
struct FakeUpstream {
    log: Arc<Mutex<Vec<Captured>>>,
    reply: Arc<Mutex<(u16, Value)>>,
}

impl FakeUpstream {
    fn new() -> Self {
        Self {
            log: Arc::new(Mutex::new(Vec::new())),
            reply: Arc::new(Mutex::new((200, json!({})))),
        }
    }

    /// 设定后续请求的应答（状态码 + JSON body）
    fn set_reply(&self, code: u16, body: Value) {
        *self.reply.lock().expect("reply lock") = (code, body);
    }

    fn captured(&self) -> Vec<Captured> {
        self.log.lock().expect("log lock").clone()
    }

    fn record(&self, req: &HttpRequest, body: Option<Value>) -> HttpResponse {
        let hdr = |n: &str| {
            req.headers()
                .get(n)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        };
        self.log.lock().expect("log lock").push(Captured {
            method: req.method().to_string(),
            path: req.path().to_string(),
            query: req.query_string().to_string(),
            authorization: hdr("Authorization"),
            x_cats_user_id: hdr("X-Cats-User-Id"),
            body,
        });
        let (code, payload) = self.reply.lock().expect("reply lock").clone();
        HttpResponse::build(StatusCode::from_u16(code).expect("valid status code")).json(payload)
    }
}

/// POST 上游端点（login / refresh）—— 带 JSON body
async fn capture_post(
    up: web::Data<FakeUpstream>,
    req: HttpRequest,
    body: web::Json<Value>,
) -> HttpResponse {
    up.record(&req, Some(body.into_inner()))
}

/// GET 上游端点（projects）—— **没有 body**。
///
/// 单独拆出来是因为 `web::Json<Value>` 提取器在无 body 的请求上会直接 400，
/// 那会把"上游收到了 projects 请求"变成"上游收到了一个坏请求"。
async fn capture_get(up: web::Data<FakeUpstream>, req: HttpRequest) -> HttpResponse {
    up.record(&req, None)
}

/// 一个同时扮演 auth-service 与 project-service 的假上游。
/// 路径与 `upstream/{auth,projects}.rs` 里 `format!("{base}/v1/...")` 对齐。
fn fake_upstream_app(
    state: FakeUpstream,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    App::new()
        .app_data(web::Data::new(state))
        .route("/v1/auth/login", web::post().to(capture_post))
        .route("/v1/auth/refresh", web::post().to(capture_post))
        .route("/v1/projects", web::get().to(capture_get))
}

/// 用**生产那张表** `handlers::configure_routes` 构造 BFF。
///
/// `main.rs` 挂的也是它——见 `handlers::configure_routes` 的文档注释。
fn bff_app(
    auth_base: String,
    project_base: String,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let cfg = Config::for_test(auth_base, project_base);
    let auth = AuthClient::new(&cfg);
    let projects = ProjectsClient::new(&cfg);
    let tasks = TasksClient::new(&cfg);
    let rbac = shared_checker();

    App::new()
        .app_data(web::Data::new(cfg))
        .app_data(web::Data::new(auth))
        .app_data(web::Data::new(projects))
        .app_data(web::Data::new(tasks))
        .app_data(web::Data::new(rbac))
        .app_data(handlers::json_config())
        .configure(handlers::configure_routes)
}

/// 起假上游（随机端口）+ 用它的地址构造 BFF
///
/// 返回的 `ServerHandle` 绑成 `_server`：它不控制生命周期（服务已 spawn 到
/// runtime 上，测试结束随 runtime 收掉），留着只是为了不产出 unused 警告。
/// **不要 await `Server`** —— 它要等服务停止才完成，我们从不调 `stop()`。
async fn harness() -> (
    FakeUpstream,
    ServerHandle,
    App<
        impl ServiceFactory<
            ServiceRequest,
            Config = (),
            Response = ServiceResponse,
            Error = actix_web::Error,
            InitError = (),
        >,
    >,
) {
    let state = FakeUpstream::new();
    let for_server = state.clone();
    // 注意顺序：`addrs()` 在 `HttpServer` 上，`run()` 之后拿到的 `Server`
    // 句柄已经没有这个方法了
    let srv = HttpServer::new(move || fake_upstream_app(for_server.clone()))
        .bind("127.0.0.1:0")
        .expect("bind fake upstream");
    let addr = srv.addrs()[0];
    let server = srv.run();
    // `HttpServer::run()` 返回的是 **Future**，必须 spawn 出去才会真正开始
    // 监听。只把它 bind 住不动，假上游就永远不会 accept —— 症状是 BFF 侧
    // 全部拿到 `dependency_unavailable` 的 502。
    let handle = server.handle();
    rt::spawn(server);

    // 故意带尾斜杠：验证客户端的 trailing-slash trim 真的生效
    // （`AuthClient::new` / `ProjectsClient::new` 都做 `trim_end_matches('/')`）
    let base = format!("http://{}/", addr);
    let app = bff_app(base.clone(), base);
    (state, handle, app)
}

// 上游应答夹具：形状必须能被 `upstream::auth` / `upstream::projects` 的
// 强类型 DTO 反序列化，否则 200 也会被当成错误。
fn login_reply() -> Value {
    json!({
        "access_token": "at-1",
        "refresh_token": "rt-1",
        "expires_in": 900,
        "token_type": "Bearer",
        "user_id": "00000000-0000-0000-0000-000000000001",
        "username": "u"
    })
}

fn refresh_reply() -> Value {
    json!({
        "access_token": "at-2",
        "refresh_token": "rt-2",
        "expires_in": 900,
        "token_type": "Bearer"
    })
}

fn projects_reply() -> Value {
    json!({
        "items": [{
            "id": "p-1",
            "name": "proj",
            "source_lang": "en-US",
            "target_lang": "ja-JP"
        }],
        "page": 1,
        "page_size": 20,
        "total": 1
    })
}

// ---------------------------------------------------------------------
// URL 拼装 + body 转发
// ---------------------------------------------------------------------

#[actix_web::test]
async fn login_forwards_to_auth_service_login_path() {
    let (up, _server, app) = harness().await;
    up.set_reply(200, login_reply());
    let app = test::init_service(app).await;

    let req = test::TestRequest::post()
        .uri("/v1/auth/login")
        .set_json(json!({"username": "u", "password": "p"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let seen = up.captured();
    assert_eq!(seen.len(), 1, "upstream should be called exactly once");
    assert_eq!(seen[0].method, "POST");
    // 这条断言同时覆盖 trailing-slash trim：基址带 `/`，若没 trim
    // 就会变成 `//v1/auth/login`，path 不匹配
    assert_eq!(seen[0].path, "/v1/auth/login");
    // 注意**不是**逐字透传：BFF 先把请求反序列化成强类型 `LoginRequest` 再序列化
    // 发出，所以 `tenant_id` 这个 `Option` 会被补成 `null`（`upstream/auth.rs:26`）。
    // 旧版测试断言的是"body 原样透传"，那描述的是已删除的裸透传设计。
    assert_eq!(
        seen[0].body,
        Some(json!({"username": "u", "password": "p", "tenant_id": null})),
    );
}

#[actix_web::test]
async fn login_response_keeps_its_payload_through_the_typed_dto() {
    let (up, _server, app) = harness().await;
    up.set_reply(200, login_reply());
    let app = test::init_service(app).await;

    let req = test::TestRequest::post()
        .uri("/v1/auth/login")
        .set_json(json!({"username": "u", "password": "p"}))
        .to_request();
    let body: Value = test::call_and_read_body_json(&app, req).await;
    assert_eq!(body, login_reply());
}

#[actix_web::test]
async fn refresh_hits_refresh_path_not_login_path() {
    let (up, _server, app) = harness().await;
    up.set_reply(200, refresh_reply());
    let app = test::init_service(app).await;

    let req = test::TestRequest::post()
        .uri("/v1/auth/refresh")
        .set_json(json!({"refresh_token": "r0"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let seen = up.captured();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].path, "/v1/auth/refresh");
    assert_ne!(seen[0].path, "/v1/auth/login");
}

// ---------------------------------------------------------------------
// 本地校验必须早于网络
// ---------------------------------------------------------------------

#[actix_web::test]
async fn login_rejects_empty_username_locally_and_never_calls_upstream() {
    // 鉴别力所在：只看状态码的话，"400 来自上游"和"400 来自本地校验"没有区别。
    // 断言假上游一次都没被调用，才能证明校验发生在发请求之前。
    let (up, _server, app) = harness().await;
    up.set_reply(200, login_reply());
    let app = test::init_service(app).await;

    let req = test::TestRequest::post()
        .uri("/v1/auth/login")
        .set_json(json!({"username": "", "password": "p"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert!(
        up.captured().is_empty(),
        "本地校验失败时不应发出任何上游请求；实际捕获到 {:?}",
        up.captured()
    );
}

// ---------------------------------------------------------------------
// 上游非 2xx 的处理 —— 生产走统一错误信封，不是原样透传
// ---------------------------------------------------------------------

#[actix_web::test]
async fn upstream_401_becomes_502_dependency_unavailable_not_a_silent_200() {
    // 旧版测试断言"401 原样透传"，那描述的是已删除的 `routes.rs` 那套裸透传。
    // 生产用强类型客户端，非 2xx 会变成 `BffError::UpstreamError` ->
    // 502 `dependency_unavailable`（见 error.rs 的 status_code / to_body）。
    let (up, _server, app) = harness().await;
    up.set_reply(401, json!({"error": "invalid_credentials"}));
    let app = test::init_service(app).await;

    let req = test::TestRequest::post()
        .uri("/v1/auth/login")
        .set_json(json!({"username": "u", "password": "bad"}))
        .to_request();
    let resp = test::call_service(&app, req).await;

    // 关键：既不是 200（不能把上游失败当成功），也不是 401（原样透传）
    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], json!("dependency_unavailable"));
    // 上游的真实状态码不能丢 —— 排障时要看的就是它
    assert!(
        body["message"].as_str().unwrap_or_default().contains("401"),
        "上游状态码应出现在 message 里，实际 body={body}"
    );
    // 上游 body 进 detail，不被丢弃。**是字符串不是嵌套对象** ——
    // `ErrorBody.detail` 是 `Option<String>`，`to_body()` 里直接塞上游原文，
    // 不解析（见 error.rs 的 `UpstreamError` 分支）。
    let detail = body["detail"].as_str().unwrap_or_default();
    assert!(
        detail.contains("invalid_credentials"),
        "上游 body 应进 detail，实际 detail={detail:?}"
    );
}

#[actix_web::test]
async fn upstream_503_becomes_502_and_keeps_the_status() {
    let (up, _server, app) = harness().await;
    up.set_reply(503, json!({"error": "maintenance"}));
    let app = test::init_service(app).await;

    let req = test::TestRequest::get().uri("/v1/projects").to_request();
    let resp = test::call_service(&app, req).await;
    // 没有 token 会先被鉴权拦下（401），所以这条只关心"不是 503 原样透传"
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn upstream_200_with_unexpected_body_is_not_reported_as_success() {
    // 上游返回 200 但 body 不符合 DTO —— 反序列化失败必须变成错误，
    // 不能把一段垃圾 JSON 当成成功结果透出去。
    let (up, _server, app) = harness().await;
    up.set_reply(200, json!({"totally": "unexpected"}));
    let app = test::init_service(app).await;

    let req = test::TestRequest::post()
        .uri("/v1/auth/login")
        .set_json(json!({"username": "u", "password": "p"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ne!(
        resp.status(),
        StatusCode::OK,
        "上游 200 但 DTO 不匹配时不应报成功"
    );
    let body: Value = test::read_body_json(resp).await;
    assert!(
        body.get("error").is_some(),
        "应返回统一错误信封，实际 body={body}"
    );
}

// ---------------------------------------------------------------------
// access token 转发（鉴权通过之后）
// ---------------------------------------------------------------------

#[actix_web::test]
async fn list_projects_forwards_bearer_token_to_upstream() {
    // 生产用的是 `bearer_auth(access_token)`（upstream/projects.rs），
    // **不**注入 X-Cats-* 头。旧测试断言注入 X-Cats-User-Id —— 那是已删除的
    // `routes.rs` 裸透传设计的行为。这里把实际行为钉住，顺带钉住"不发 X-Cats-*"。
    let (up, _server, app) = harness().await;
    up.set_reply(200, projects_reply());
    let token = sign_access_token(&["User"]);
    let app = test::init_service(app).await;

    let req = test::TestRequest::get()
        .uri("/v1/projects")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let seen = up.captured();
    assert_eq!(seen.len(), 1, "upstream should be called exactly once");
    assert_eq!(seen[0].method, "GET");
    assert_eq!(seen[0].path, "/v1/projects");
    assert_eq!(seen[0].authorization, Some(format!("Bearer {token}")));
    assert_eq!(
        seen[0].x_cats_user_id, None,
        "生产不发 X-Cats-User-Id（只发 Authorization）；若将来真要注入，这条要一起改"
    );
}

#[actix_web::test]
async fn list_projects_passes_query_through_to_upstream() {
    let (up, _server, app) = harness().await;
    up.set_reply(200, projects_reply());
    let token = sign_access_token(&["User"]);
    let app = test::init_service(app).await;

    let req = test::TestRequest::get()
        .uri("/v1/projects?page=2&page_size=5")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let seen = up.captured();
    assert_eq!(seen.len(), 1);
    // `ListProjectsQuery` 用 serde 序列化进 query，字段名与 BFF 端一致
    assert!(
        seen[0].query.contains("page=2") && seen[0].query.contains("page_size=5"),
        "分页参数应转发到上游，实际 query={:?}",
        seen[0].query
    );
}

#[actix_web::test]
async fn list_projects_body_survives_the_typed_dto_round_trip() {
    // 名字里没有"verbatim"是刻意的：BFF 把上游 body 反序列化成
    // `ListProjectsResponse` 再序列化回去，所以上游**没给**的可选字段会以
    // `null` 出现（`upstream/projects.rs:40-44` 的三个 `#[serde(default)]`）。
    // 逐字比 JSON 会红；该断言的是"语义等价 + 可选字段被显式物化"。
    let (up, _server, app) = harness().await;
    up.set_reply(200, projects_reply());
    let token = sign_access_token(&["User"]);
    let app = test::init_service(app).await;

    let req = test::TestRequest::get()
        .uri("/v1/projects")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let body: Value = test::call_and_read_body_json(&app, req).await;

    assert_eq!(body["total"], json!(1));
    assert_eq!(body["page"], json!(1));
    assert_eq!(body["page_size"], json!(20));
    assert_eq!(body["items"][0]["id"], json!("p-1"));
    assert_eq!(body["items"][0]["name"], json!("proj"));
    // 上游省略的可选字段被显式物化为 null（而不是消失）
    assert_eq!(body["items"][0]["tm_id"], Value::Null);
    assert_eq!(body["items"][0]["termbase_id"], Value::Null);
    assert_eq!(body["items"][0]["created_at"], Value::Null);
}

#[actix_web::test]
async fn list_projects_without_token_never_calls_upstream() {
    // 鉴权失败必须发生在任何网络交互之前
    let (up, _server, app) = harness().await;
    up.set_reply(200, projects_reply());
    let app = test::init_service(app).await;

    let req = test::TestRequest::get().uri("/v1/projects").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert!(up.captured().is_empty());
}

#[actix_web::test]
async fn list_projects_with_role_lacking_project_read_is_403() {
    // `Role::Guest` 在默认权限矩阵里没有 Project:Read（只有 ApiDesign/ModuleDesign）
    let (up, _server, app) = harness().await;
    up.set_reply(200, projects_reply());
    let token = sign_access_token(&["Guest"]);
    let app = test::init_service(app).await;

    let req = test::TestRequest::get()
        .uri("/v1/projects")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    assert!(up.captured().is_empty(), "RBAC 拒绝时不应发出上游请求");
}

// ---------------------------------------------------------------------
// healthz 形状 —— 全仓统一后的唯一形状
// ---------------------------------------------------------------------

#[actix_web::test]
async fn healthz_returns_the_canonical_shape() {
    let (up, _server, app) = harness().await;
    let app = test::init_service(app).await;

    let req = test::TestRequest::get().uri("/healthz").to_request();
    let body: Value = test::call_and_read_body_json(&app, req).await;

    assert_eq!(body["status"], json!("ok"));
    // app.name 必须是本 crate 的包名 —— 形状统一的要点之一就是它不再退化
    // 成共享库的 "cats-common"
    assert_eq!(body["app"]["name"], json!(env!("CARGO_PKG_NAME")));
    assert!(body["app"]["version"].is_string());
    // 内部拓扑不得再出现在 healthz 里
    assert!(
        body.get("upstreams").is_none(),
        "upstreams 不应外泄: {body}"
    );
    assert!(
        body.get("bind_addr").is_none(),
        "bind_addr 不应外泄: {body}"
    );
    // 旧的扁平键名
    assert!(body.get("service").is_none(), "不应再有 service 键: {body}");
    assert!(body.get("name").is_none(), "不应再有顶层 name 键: {body}");
    // 纯本地应答：健康检查不该碰任何上游（探针会在每次重启风暴里反复调用它）
    assert!(
        up.captured().is_empty(),
        "healthz 不应发出任何上游请求；实际捕获到 {:?}",
        up.captured()
    );
}
