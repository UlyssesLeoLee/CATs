//! `/api/v1/*` 那套路由的运行时覆盖（`routes.rs`）
//!
//! 2026-10-05 接入编译后，`routes.rs` 一直是**零运行时覆盖**：crate 里
//! 唯一跑 HTTP 的集成测试 `bff_smoke.rs` 打的是 `main.rs` 实际注册的那套
//! `handlers::*` 路由表（`/v1/*`），打不到 `/api/v1/*`。也就是说这 178 行
//! 只做过类型检查。
//!
//! 这套测试起一个**真的 HTTP 上游**（`HttpServer` 绑 `127.0.0.1:0`，
//! 随机端口），把 `UpstreamClient` 的两个基址指过去，于是可以断言真实契约，
//! 而不只是"连不上所以 502"：
//!
//! | 断言的性质 | 靠什么证据 |
//! |---|---|
//! | URL 拼装正确 | 假上游**记下**自己收到的 method + path |
//! | body 原样透传 | 假上游记下收到的 JSON，逐字段比对 |
//! | 上游状态码原样透传 | 让上游回 401/503，断言 BFF **不是** 200/502 |
//! | `X-Cats-*` header 注入 | 假上游记下收到的 header 列表 |
//!
//! 与 `bff_smoke.rs` 的"DEAD_URL + 断言走到哪一层"策略互补：那边验证
//! 鉴权层，这条验证透传层。
//!
//! **gRPC 不需要真服务**：`translate_lookup` 的 `source_text` 空值校验发生在
//! 调用 `TranslationClient` 之前，用 `connect_lazy()` 的 channel（不建立连接）
//! 就能满足 `web::Data<TranslationClient>` 的提取。
//!
//! 不用 `actix_web::test::start`（它要 `macros` feature），改用 `HttpServer`
//! 手动绑端口。

use std::sync::{Arc, Mutex};

use actix_web::dev::{ServerHandle, ServiceFactory, ServiceRequest, ServiceResponse};
use actix_web::http::StatusCode;
use actix_web::rt;
use actix_web::{test, web, App, HttpRequest, HttpResponse, HttpServer};
use cats_bff::config::Config;
use cats_bff::grpc_clients::TranslationClient;
use cats_bff::routes;
use cats_bff::upstream_passthrough::UpstreamClient;
use serde_json::{json, Value};

// ---------------------------------------------------------------------
// 假上游
// ---------------------------------------------------------------------

/// 假上游记下的一次请求
#[derive(Clone, Debug)]
struct Captured {
    method: String,
    path: String,
    user_id: Option<String>,
    org_id: Option<String>,
    authorization: Option<String>,
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
            reply: Arc::new(Mutex::new((200, json!({"ok": true})))),
        }
    }

    /// 设定后续请求的应答（状态码 + JSON body）
    fn set_reply(&self, code: u16, body: Value) {
        *self.reply.lock().expect("reply lock") = (code, body);
    }

    fn captured(&self) -> Vec<Captured> {
        self.log.lock().expect("log lock").clone()
    }
}

fn header_str(req: &HttpRequest, name: &str) -> Option<String> {
    req.headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

impl FakeUpstream {
    fn record(&self, req: &HttpRequest, body: Option<Value>) -> HttpResponse {
        self.log.lock().expect("log lock").push(Captured {
            method: req.method().to_string(),
            path: req.path().to_string(),
            user_id: header_str(req, "X-Cats-User-Id"),
            org_id: header_str(req, "X-Cats-Org-Id"),
            authorization: header_str(req, "Authorization"),
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
/// 单独拆出来是因为 `web::Json<Value>` 提取器在无 body 的请求上会直接
/// 400，那会把"上游收到了 projects 请求"这件事变成"上游收到了一个坏请求"。
async fn capture_get(up: web::Data<FakeUpstream>, req: HttpRequest) -> HttpResponse {
    up.record(&req, None)
}

/// 一个同时扮演 auth-service 与 project-service 的假上游
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

/// 用 `routes::configure` 这张路由表构造 BFF
///
/// **注意它不是生产路由表。** 原文写的是"唯一的"，不成立：
/// `main.rs:63-76` 注册的是另一套（`handlers::*` + `/v1/*`，无 `/api/v1` 前缀），
/// 而 `routes::configure` 的唯一调用者就是这个测试文件。
/// 于是本文件里针对透传路径的断言**不能**用来推断生产行为 —— 它们证明的是
/// 这张表自己能用。生产那套 `handlers::*` 的对应行为目前没有集成测试覆盖。
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
    let upstream = UpstreamClient::new(&cfg).expect("UpstreamClient::new");

    // `connect_lazy()` 只解析 URL，不建立连接 —— gRPC 真服务不在本测试范围
    let channel = tonic::transport::Channel::from_shared("http://127.0.0.1:1".to_string())
        .expect("valid grpc url")
        .connect_lazy();
    let grpc = TranslationClient::from_channel(channel);

    App::new()
        .app_data(web::Data::new(upstream))
        .app_data(web::Data::new(grpc))
        .configure(routes::configure)
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
    // 全部拿到 `dependency_timeout` 的 502。
    let handle = server.handle();
    rt::spawn(server);

    // 故意带尾斜杠：验证 `UpstreamClient` 的 trailing-slash trim 真的生效
    let base = format!("http://{}/", addr);
    let app = bff_app(base.clone(), base);
    (state, handle, app)
}

/// 仓库既有约定：127.0.0.1:1 是 DEAD_URL（audit-service 的测试也用它）
fn dead_base() -> String {
    "http://127.0.0.1:1".to_string()
}

// ---------------------------------------------------------------------
// URL 拼装 + body 透传
// ---------------------------------------------------------------------

#[actix_web::test]
async fn login_forwards_to_auth_service_v1_login_path() {
    let (up, _server, app) = harness().await;
    up.set_reply(200, json!({"access_token": "tok-123"}));
    let app = test::init_service(app).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
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
    assert_eq!(
        seen[0].body,
        Some(json!({"username": "u", "password": "p"}))
    );
}

#[actix_web::test]
async fn login_response_body_is_passed_through_verbatim() {
    let (up, _server, app) = harness().await;
    up.set_reply(200, json!({"access_token": "tok-123", "expires_in": 900}));
    let app = test::init_service(app).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(json!({"username": "u", "password": "p"}))
        .to_request();
    let body: Value = test::call_and_read_body_json(&app, req).await;
    assert_eq!(body, json!({"access_token": "tok-123", "expires_in": 900}));
}

#[actix_web::test]
async fn refresh_hits_refresh_path_not_login_path() {
    let (up, _server, app) = harness().await;
    up.set_reply(200, json!({"access_token": "tok-456"}));
    let app = test::init_service(app).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
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
// 状态码透传 —— 这一组是 proxy_status 的鉴别力所在
// ---------------------------------------------------------------------

#[actix_web::test]
async fn upstream_401_is_passed_through_not_flattened_to_200() {
    let (up, _server, app) = harness().await;
    up.set_reply(401, json!({"error": "invalid_credentials"}));
    let app = test::init_service(app).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(json!({"username": "u", "password": "bad"}))
        .to_request();
    let resp = test::call_service(&app, req).await;

    // 关键：不是 200 也不是 502。`proxy_status` 若退化成"永远 200"或
    // "上游非 2xx 一律 502"，这条就红
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body, json!({"error": "invalid_credentials"}));
}

#[actix_web::test]
async fn upstream_503_is_passed_through_unchanged() {
    let (up, _server, app) = harness().await;
    up.set_reply(503, json!({"error": "maintenance"}));
    let app = test::init_service(app).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/projects")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[actix_web::test]
async fn upstream_201_is_passed_through_unchanged() {
    let (up, _server, app) = harness().await;
    up.set_reply(201, json!({"id": "p-1"}));
    let app = test::init_service(app).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/projects")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
}

// ---------------------------------------------------------------------
// X-Cats-* header 注入
// ---------------------------------------------------------------------

#[actix_web::test]
async fn list_projects_injects_cats_headers_from_request() {
    let (up, _server, app) = harness().await;
    up.set_reply(200, json!({"projects": []}));
    let app = test::init_service(app).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/projects")
        .insert_header(("X-Cats-User-Id", "user-42"))
        .insert_header(("X-Cats-Org-Id", "org-7"))
        .insert_header(("Authorization", "Bearer abc"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let seen = up.captured();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].path, "/v1/projects");
    assert_eq!(seen[0].user_id.as_deref(), Some("user-42"));
    assert_eq!(seen[0].org_id.as_deref(), Some("org-7"));
    assert_eq!(seen[0].authorization.as_deref(), Some("Bearer abc"));
}

#[actix_web::test]
async fn list_projects_does_not_invent_headers_when_absent() {
    let (up, _server, app) = harness().await;
    up.set_reply(200, json!({"projects": []}));
    let app = test::init_service(app).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/projects")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let seen = up.captured();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].body, None);
    // 不发空值 header：若 `forward_project_list` 从"缺省不发"改成
    // 总是塞一个空 header，这里会红
    assert_eq!(seen[0].user_id, None);
    assert_eq!(seen[0].org_id, None);
    assert_eq!(seen[0].authorization, None);
}

// ---------------------------------------------------------------------
// 上游不可达 / 本地路由
// ---------------------------------------------------------------------

#[actix_web::test]
async fn login_returns_502_when_upstream_unreachable() {
    let app = test::init_service(bff_app(dead_base(), dead_base())).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(json!({"username": "u", "password": "p"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
}

#[actix_web::test]
async fn list_projects_returns_502_when_upstream_unreachable() {
    let app = test::init_service(bff_app(dead_base(), dead_base())).await;
    let req = test::TestRequest::get()
        .uri("/api/v1/projects")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
}

#[actix_web::test]
async fn healthz_is_local_and_200_without_any_upstream() {
    // 注意这条**只对 `routes::healthz` 成立**，不能当成生产行为：
    //   - 它不碰任何上游，确实恒 200；
    //   - 但生产应答的是 `handlers::healthz_handler`（main.rs:63），那个
    //     handler 读 `web::Data<Config>`，会吐出 `bind_addr` 和 5 个上游 URL。
    // 两者 payload 不同（这个没有 version 之外的字段、那个有 upstreams）。
    let app = test::init_service(bff_app(dead_base(), dead_base())).await;
    let req = test::TestRequest::get().uri("/healthz").to_request();
    let body: Value = test::call_and_read_body_json(&app, req).await;
    assert_eq!(body["status"], json!("ok"));
    assert_eq!(body["service"], json!("cats-bff"));
}

// ---------------------------------------------------------------------
// translate 两端点（gRPC 不在范围，只验本地逻辑）
// ---------------------------------------------------------------------

#[actix_web::test]
async fn translate_lookup_rejects_empty_source_text_before_calling_grpc() {
    // gRPC 用的是 DEAD_URL 的 channel：若空值校验没拦住，这里会变成 502
    let app = test::init_service(bff_app(dead_base(), dead_base())).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/translate/lookup")
        .set_json(json!({
            "tenant_id": "t1",
            "project_id": "p1",
            "source_text": "",
            "source_lang": "en-US",
            "target_lang": "ja-JP"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    // 400 而不是 502 —— 证明请求死在本地校验，没有走到 gRPC
    assert_ne!(resp.status(), StatusCode::BAD_GATEWAY);
}

#[actix_web::test]
async fn translate_commit_returns_501_stub_ack() {
    let app = test::init_service(bff_app(dead_base(), dead_base())).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/translate/commit")
        .set_json(json!({"tm_id": "x"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["accepted"], json!(false));
    assert!(body["note"]
        .as_str()
        .unwrap_or_default()
        .contains("pending v1.1"));
}
