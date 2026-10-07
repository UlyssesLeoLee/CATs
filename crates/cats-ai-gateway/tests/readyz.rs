//! `cats-ai-gateway` 的 `/readyz`：不依赖上游 LLM，但必须如实报告"能不能服务"
//!
//! # 这个服务和 DB 型服务的区别
//!
//! `cats-ai-gateway` 的 `Cargo.toml` 里**没有 `sqlx`、没有任何连接池** —— 它是
//! 外部 LLM provider 的网关，不持有数据库。所以这里**没有 DB 可探**。
//!
//! 那 `/readyz` 该探什么？答案是**不能探上游**。对 OpenAI / Anthropic / DeepSeek
//! 发网络请求来做就绪判定，意味着上游一限流或抖动就把整个网关摘出轮转，把
//! 局部上游故障放大成全站故障 —— **严格比现状更差**。
//!
//! 所以判定源是**本服务自己的启动状态**：`Router::is_routable()`，即是否至少
//! 有一个可路由的 provider。router 为空时每个请求都必然 `ProviderNotFound`，
//! 此时报 503 是如实的。
//!
//! body 里的 `db` 字段沿用 `cats_common::ReadyResponse` 的固定顶层键，本服务
//! 用它承载上面那个启动态判定（`"ok"` / `"fail"`）。它**不是**假装有数据库
//! 探测 —— 代码里不存在任何 DB 调用，也不声称有。
//!
//! 这四条用例全部**不需要**真 PostgreSQL，也不需要任何网络。

use actix_web::{http::StatusCode, test as actix_test, web, App};
use cats_ai_gateway::api;
use cats_ai_gateway::provider;
use cats_ai_gateway::quota::QuotaTracker;
use cats_ai_gateway::router::Router;
use cats_ai_gateway::service::AiGatewayService;
use serde_json::Value;

/// 空 router —— 网关"活着"但一个请求也处理不了
fn service_without_any_provider() -> AiGatewayService {
    AiGatewayService::new(Router::new(vec![]), QuotaTracker::new())
}

/// 至少有一个可路由 provider
fn service_with_one_provider() -> AiGatewayService {
    AiGatewayService::new(Router::new(vec![provider::openai()]), QuotaTracker::new())
}

async fn get(path: &str, svc: AiGatewayService) -> (StatusCode, Value) {
    let app = actix_test::init_service(
        App::new()
            .app_data(web::Data::new(svc))
            .configure(api::configure_routes),
    )
    .await;
    let req = actix_test::TestRequest::get().uri(path).to_request();
    let resp = actix_test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = actix_test::read_body_json(resp).await;
    (status, body)
}

#[actix_web::test]
async fn readyz_returns_503_when_no_provider_is_registered() {
    let (status, body) = get("/readyz", service_without_any_provider()).await;
    assert_eq!(
        status,
        StatusCode::SERVICE_UNAVAILABLE,
        "router 为空时每个请求都会 ProviderNotFound，此时报 Ready 等于骗调度器"
    );
    assert_eq!(body["status"], "not_ready");
    assert_eq!(body["db"], "fail");
    assert_eq!(body["service"], "cats-ai-gateway");
}

#[actix_web::test]
async fn readyz_returns_200_when_a_provider_is_registered() {
    let (status, body) = get("/readyz", service_with_one_provider()).await;
    assert_eq!(status, StatusCode::OK, "有可路由 provider 时必须 Ready");
    assert_eq!(body["status"], "ready");
    assert_eq!(body["db"], "ok");
}

#[actix_web::test]
async fn readyz_shape_is_stable_and_shared_across_services() {
    let (_status, body) = get("/readyz", service_with_one_provider()).await;
    let obj = body.as_object().expect("顶层必须是对象");
    let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec!["db", "service", "status"],
        "顶层键必须与 cats_common::ReadyResponse 一致，否则各服务形状会漂移"
    );
}

/// 反向用例：进程活着就该报 200，哪怕一个 provider 都没有。
///
/// 存活与就绪是两回事。让 `/healthz` 依赖 router 状态，等于让上游配置错误
/// 变成无限重启 —— 而重启解决不了配置问题。
#[actix_web::test]
async fn healthz_stays_200_even_when_no_provider_is_registered() {
    let (status, _body) = get("/healthz", service_without_any_provider()).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "存活探针不查依赖：provider 没配好应该报 503（摘流量），不该重启进程"
    );
}
