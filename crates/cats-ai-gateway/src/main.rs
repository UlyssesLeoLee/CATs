//! `cats-ai-gateway` 入口
//!
//! 启动 REST (actix-web) + gRPC (tonic) 双协议服务器
//!
//! 引用: doc/02-基础设计/技术选型/CATs_Rust技术选型书_v1.0.md §5.1/§5.3
//! 引用: doc/02-基础设计/决策/CATs_ADR-009_mTLS服务间通信_v1.0.md (MVP 阶段不启用 mTLS)
//!
//! 端口:
//! - REST_BIND_ADDR (默认 0.0.0.0:8090)
//! - GRPC_BIND_ADDR (默认 0.0.0.0:50061)

use actix_web::{web, App, HttpServer};
use cats_common::AppMeta;
use std::env;
use tonic::transport::Server;
use tracing::info;

use cats_ai_gateway::api::{configure_routes, healthz_handler};
use cats_ai_gateway::compliance::ComplianceMode;
use cats_ai_gateway::error::ProviderError;
use cats_ai_gateway::provider::{anthropic, deepseek, gemini, openai, ChatMessage};
use cats_ai_gateway::proto::llm::v1::{
    llm_gateway_server::{LlmGateway, LlmGatewayServer},
    ChatRequest as ProtoChatRequest, ChatResponse as ProtoChatResponse,
};
use cats_ai_gateway::quota::QuotaTracker;
use cats_ai_gateway::router::Router;
use cats_ai_gateway::service::AiGatewayService;

/// 业务配置
#[derive(Debug, Clone)]
struct Config {
    rest_bind_addr: String,
    grpc_bind_addr: String,
    /// 启动时合规模式 (MVP 默认 Cloud, 可由 env 切到 Local)
    compliance_mode: ComplianceMode,
}

impl Config {
    fn from_env() -> Self {
        let rest_bind_addr = env::var("REST_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8090".to_string());
        let grpc_bind_addr = env::var("GRPC_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:50061".to_string());
        let compliance_mode = env::var("COMPLIANCE_MODE")
            .map(|s| ComplianceMode::parse(&s))
            .unwrap_or(ComplianceMode::Cloud);
        Self { rest_bind_addr, grpc_bind_addr, compliance_mode }
    }
}

/// gRPC server 实现 (转 REST 等价逻辑)
struct GrpcGatewayServer {
    svc: std::sync::Arc<AiGatewayService>,
}

#[tonic::async_trait]
impl LlmGateway for GrpcGatewayServer {
    async fn chat(
        &self,
        request: tonic::Request<ProtoChatRequest>,
    ) -> Result<tonic::Response<ProtoChatResponse>, tonic::Status> {
        let req = request.into_inner();
        let domain_req = cats_ai_gateway::provider::ChatRequest {
            model: req.model.clone(),
            messages: req.messages.into_iter().map(|m| ChatMessage {
                role: m.role,
                content: m.content,
            }).collect(),
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            idempotency_key: req.idempotency_key,
        };

        match self.svc.chat(&req.org_id, &domain_req).await {
            Ok(resp) => {
                let proto = ProtoChatResponse {
                    id: resp.id,
                    model: resp.model,
                    provider: resp.provider,
                    content: resp.content,
                    prompt_tokens: resp.prompt_tokens,
                    completion_tokens: resp.completion_tokens,
                    total_tokens: resp.total_tokens,
                    fallback_chain: resp.fallback_chain,
                };
                Ok(tonic::Response::new(proto))
            }
            Err(e) => Err(tonic_status_from_provider_error(&e)),
        }
    }

    // 注: ChatStream MVP 阶段未实现 (per 2026-09-19 落地任务), proto 中已注释
    // 不再实现 chat_stream — 待 Sprint 3 补
}

/// ProviderError → tonic::Status 转换 (per 接口设计 §1.4)
fn tonic_status_from_provider_error(e: &ProviderError) -> tonic::Status {
    use tonic::Code;
    let code = match e {
        ProviderError::Validation(_)         => Code::InvalidArgument,
        ProviderError::RateLimited { .. }    => Code::ResourceExhausted,
        ProviderError::ComplianceBlocked { .. } => Code::FailedPrecondition,
        ProviderError::ProviderNotFound(_)   => Code::Unavailable,
        ProviderError::AllFailed(_)          => Code::Unavailable,
        ProviderError::Upstream { .. }       => Code::Unavailable,
        ProviderError::Transient { .. }      => Code::Unavailable,
    };
    tonic::Status::new(code, e.to_string())
}

/// 健康检查 JSON (含 gRPC server 状态)
#[derive(serde::Serialize)]
struct HealthResponse {
    status: &'static str,
    app: AppMeta,
}

async fn healthz() -> actix_web::HttpResponse {
    actix_web::HttpResponse::Ok().json(HealthResponse {
        status: "ok",
        app: AppMeta::current(),
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    cats_common::init_tracing();
    let cfg = Config::from_env();
    info!(
        rest_addr = %cfg.rest_bind_addr,
        grpc_addr = %cfg.grpc_bind_addr,
        compliance = %cfg.compliance_mode.as_str(),
        "starting cats-ai-gateway"
    );

    // 构造 service
    let router = Router::new(vec![openai(), anthropic(), gemini(), deepseek()]);
    let svc = AiGatewayService::new(router, QuotaTracker::new())
        .with_compliance(cfg.compliance_mode);
    let svc_arc = std::sync::Arc::new(svc);

    // gRPC server (在后台 tokio task 里)
    let grpc_svc = svc_arc.clone();
    let grpc_addr: std::net::SocketAddr = cfg.grpc_bind_addr.parse().expect("invalid GRPC_BIND_ADDR");
    tokio::spawn(async move {
        let server = GrpcGatewayServer { svc: grpc_svc };
        let result = Server::builder()
            .add_service(LlmGatewayServer::new(server))
            .serve(grpc_addr)
            .await;
        if let Err(e) = result {
            tracing::error!(error = %e, "gRPC server failed");
        }
    });

    // REST server (actix-web, 主线程)
    let svc_data = web::Data::from(svc_arc);
    HttpServer::new(move || {
        App::new()
            .app_data(svc_data.clone())
            .route("/healthz", actix_web::web::get().to(healthz))
            .configure(configure_routes)
    })
    .bind(&cfg.rest_bind_addr)?
    .run()
    .await
}