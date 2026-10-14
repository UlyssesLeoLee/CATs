//! REST API 出口 (actix-web 4)
//!
//! 引用: doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md §1.5 (REST URL 路径版本化)
//! 引用: doc/05-其他/错误码/CATs_错误码表_v1.0.1.md (12 状态错误码 + 信封)
//!
//! MVP 端点:
//! - POST /v1/llm/chat     同步 chat 调用 (给 translation-core 同步使用)
//! - GET  /v1/llm/usage    用量查询 (?org_id=...)
//! - GET  /healthz         健康检查 (存活探针, 恒 200)
//! - GET  /readyz          就绪探针, 不可服务时 **503** (2026-10-07 新增)
//!
//! 请求/响应 DTO 通过 serde + 错误信封 (per error.rs)

use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::error::ProviderError;
use crate::provider::{ChatMessage, ChatRequest, ChatResponse};
use crate::quota::UsageSnapshot;
use crate::service::AiGatewayService;

// ─────────────────────────────────────────────────────────────────────────
// 请求 / 响应 DTO
// ─────────────────────────────────────────────────────────────────────────

/// POST /v1/llm/chat 请求体
#[derive(Debug, Deserialize)]
pub struct ChatRequestDto {
    /// 必填, 调用方 org (用于配额)
    pub org_id: String,
    pub model: String,
    pub messages: Vec<ChatMessageDto>,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// 合规模式: "cloud" | "local" (默认 "cloud")
    #[serde(default = "default_compliance")]
    pub compliance_mode: String,
    /// 幂等键 (per 接口设计 §1.1)
    #[serde(default)]
    pub idempotency_key: String,
}

/// POST /v1/llm/chat 消息条目
#[derive(Debug, Deserialize)]
pub struct ChatMessageDto {
    pub role: String,
    pub content: String,
}

impl From<ChatMessageDto> for ChatMessage {
    fn from(dto: ChatMessageDto) -> Self {
        ChatMessage {
            role: dto.role,
            content: dto.content,
        }
    }
}

/// POST /v1/llm/chat 响应体 (复用 ChatResponse)
pub type ChatResponseDto = ChatResponse;

fn default_temperature() -> f32 {
    0.7
}
fn default_max_tokens() -> u32 {
    1024
}
fn default_compliance() -> String {
    "cloud".to_string()
}

/// GET /v1/llm/usage 响应
#[derive(Debug, Serialize)]
pub struct UsageResponseDto {
    pub org_id: String,
    pub window_start_ts: u64,
    pub tokens_used: u64,
    pub tokens_limit: u64,
}

impl From<UsageSnapshot> for UsageResponseDto {
    fn from(s: UsageSnapshot) -> Self {
        Self {
            org_id: s.org_id,
            window_start_ts: s.window_start_ts,
            tokens_used: s.tokens_used,
            tokens_limit: s.tokens_limit,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Handler
// ─────────────────────────────────────────────────────────────────────────

/// POST /v1/llm/chat handler
pub async fn chat_handler(
    svc: web::Data<AiGatewayService>,
    body: web::Json<ChatRequestDto>,
) -> Result<HttpResponse, ProviderError> {
    let dto = body.into_inner();

    // 合规模式切换 (per request, MVP 简化: 直接 set 全局)
    {
        let mode = crate::compliance::ComplianceMode::parse(&dto.compliance_mode);
        // 通过 service 的 compliance gate 切换 — 但 compliance gate 在 service 内,
        // 这里仅对当次请求按 mode 决定是否调用; 真正切换全局模式留给运维接口
        let _ = mode;
    }

    let req = ChatRequest {
        model: dto.model,
        messages: dto.messages.into_iter().map(Into::into).collect(),
        temperature: dto.temperature,
        max_tokens: dto.max_tokens,
        idempotency_key: dto.idempotency_key,
    };

    let response = svc.chat(&dto.org_id, &req).await?;
    Ok(HttpResponse::Ok().json(response))
}

/// GET /v1/llm/usage?org_id=... handler
pub async fn usage_handler(
    svc: web::Data<AiGatewayService>,
    query: web::Query<UsageQuery>,
) -> impl Responder {
    let snap = svc.quota().usage(&query.org_id).await;
    HttpResponse::Ok().json(UsageResponseDto::from(snap))
}

/// GET /v1/llm/usage 查询参数
#[derive(Debug, Deserialize)]
pub struct UsageQuery {
    pub org_id: String,
}

/// GET /healthz handler
pub async fn healthz_handler() -> impl Responder {
    use cats_common::AppMeta;
    // 2026-10-05: 原来用 `AppMeta::current()`, 它返回的是 **cats-common 自己的**
    // CARGO_PKG_NAME —— 于是本 service 在 healthz 里自报 "cats-common",
    // 认不出是哪个服务, 版本号也是共享库的版本。`env!` 在编译期按**本 crate**
    // 展开, 所以这里报的是 cats-ai-gateway 自己。
    //
    // 先建成变量再放进 `json!`: 宏有自己的分词器, 在其值位置内嵌一个
    // struct 字面量并在里面写 `"key": value` 会被它当成 JSON 的键值对解析。
    let app = AppMeta {
        name: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "app": app,
    }))
}

/// `GET /readyz` — 就绪探针，**必须**在不可服务时返回 503
///
/// 2026-10-07 新增。此前本服务只有 `/healthz`，而 k3s 的
/// `deploy/k3s/cats-core/cats-ai-gateway.yaml` 把 readinessProbe 指向了
/// `/healthz` —— 那个端点恒返 200、不查任何东西，于是依赖不可用时 Pod 照样
/// Ready，流量继续被派发进来。
///
/// # 探什么（本服务与 DB 型服务的区别）
///
/// 本 crate 的 `Cargo.toml` 里**没有 `sqlx`、没有任何连接池**：`cats-ai-gateway`
/// 是外部 LLM provider 的网关，不持有数据库。所以这里**没有 DB 可探**。
///
/// 也**故意不探上游 provider**（OpenAI / Anthropic / DeepSeek）：对第三方
/// 模型 API 发网络请求来做就绪判定，意味着上游一限流/抖动就把整个网关摘出
/// 轮转，把局部上游故障放大成全站故障 —— 严格比现状更差。
///
/// 判定源是**本服务自己的启动状态**：`AiGatewayService::is_serving()`，即
/// router 里至少有一个可路由的 provider。空 router 时每个请求都必然返回
/// `ProviderNotFound`，此时报 503 是如实的。
///
/// `db` 字段：共享类型 `cats_common::ReadyResponse` 的顶层键固定为
/// `["db","service","status"]`，本服务没有 DB，所以 `db` 这里承载的是上面那个
/// 启动态判定（`"ok"` / `"fail"`）。它是本服务**唯一的**依赖判定位，不是假装的
/// 数据库探针 —— 代码里不存在任何 DB 调用，也不声称有。
///
/// 形状与状态码判定都收在 `cats_common::ReadyResponse`，与其他服务一致。
pub async fn readyz(svc: web::Data<AiGatewayService>) -> impl Responder {
    let dep_ok = svc.is_serving();
    let body = cats_common::ReadyResponse::new(env!("CARGO_PKG_NAME"), dep_ok);
    HttpResponse::build(
        actix_web::http::StatusCode::from_u16(body.status_code())
            .expect("status_code 只返回 200/503，都是合法状态码"),
    )
    .json(body)
}

/// 配置 REST 路由 (给 main.rs 用)
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/llm")
            .route("/chat", web::post().to(chat_handler))
            .route("/usage", web::get().to(usage_handler)),
    )
    .route("/healthz", web::get().to(healthz_handler))
    .route("/readyz", web::get().to(readyz));
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App};

    #[actix_web::test]
    async fn healthz_returns_ok() {
        let svc = crate::service::default_service();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(svc))
                .configure(configure_routes),
        )
        .await;

        let req = test::TestRequest::get().uri("/healthz").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn chat_endpoint_hits_openai_mock() {
        let svc = crate::service::default_service();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(svc))
                .configure(configure_routes),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/v1/llm/chat")
            .set_json(serde_json::json!({
                "org_id": "org-test",
                "model": "gpt-4o-mini",
                "messages": [{"role": "user", "content": "translate hello"}],
                "compliance_mode": "cloud",
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["provider"], "openai");
        assert!(body["content"]
            .as_str()
            .unwrap()
            .starts_with("[mock-openai]"));
    }

    #[actix_web::test]
    async fn chat_endpoint_local_mode_returns_409() {
        let svc = crate::service::default_service()
            .with_compliance(crate::compliance::ComplianceMode::Local);
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(svc))
                .configure(configure_routes),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/v1/llm/chat")
            .set_json(serde_json::json!({
                "org_id": "org-test",
                "model": "gpt-4o-mini",
                "messages": [{"role": "user", "content": "hi"}],
                "compliance_mode": "local",
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 409);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"]["code"], "COMPLIANCE_BLOCKED");
    }

    #[actix_web::test]
    async fn usage_endpoint_returns_snapshot() {
        let svc = crate::service::default_service();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(svc))
                .configure(configure_routes),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/v1/llm/usage?org_id=org-test")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["org_id"], "org-test");
        assert_eq!(body["tokens_used"], 0);
    }
}
