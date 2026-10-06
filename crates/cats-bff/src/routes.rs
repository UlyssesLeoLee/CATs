//! BFF HTTP 路由（actix-web 4）
//!
//! **未接入状态（不参与编译）**
//!
//! 本文件由 `feat/mvp-final-*` 抢救进仓（commit `68fe10b`），`lib.rs` 未声明
//! ## 2026-10-05 已接入编译（第 3 步 / 共 3 步 —— 本 crate 孤儿清零）
//!
//! 原文写着"未接入"与"接入前需先做适配改造"。本步处理的四处：
//!
//! | 原代码 | 现在 | 为什么 |
//! |---|---|---|
//! | `crate::upstream::UpstreamClient` | `crate::upstream_passthrough::UpstreamClient` | `UpstreamClient` 是第 1 步接进 `upstream_passthrough` 的；`upstream/` 目录模块里从来只有三个强类型客户端 |
//! | `MatchTMRequest` | `MatchTmRequest` | prost 按 Rust 命名规范规范化了（见 `grpc_clients.rs` 头）|
//! | `BffError::BadRequest` | `BffError::InvalidRequest` | 变体不存在；两者同样映射 400 |
//! | 依赖 `crate::grpc_clients::*` | 已在第 2 步接入 | **所以步骤顺序被调整**：原计划 routes 在前，但它硬依赖 grpc_clients |
//!
//! 下面这段原文保留，它说明的仍是事实：本文件与 `main.rs` 走的那套
//! `handlers::*` + 强类型客户端是两条并存的设计。
//!
//! 当前生效的路由注册在 `main.rs`，走
//! `handlers::*` + `upstream::{auth,projects,tasks}` 那套强类型客户端。
//!
//! 本文件属于另一条平行设计（`/api/v1/*` 前缀 + 裸透传），其依赖的
//! `crate::upstream::UpstreamClient` 与 `crate::grpc_clients` 同为未接入文件，
//! 接入前需先做适配改造。详见 `upstream_passthrough.rs` 头部的说明。
//!
//! 端点清单（per 任务规范 + 接口设计书 v2.0 §3.5）：
//!
//! | Method | Path                          | 类型        | 下游                                  |
//! |--------|-------------------------------|-------------|---------------------------------------|
//! | GET    | `/healthz`                    | 本地        | —                                     |
//! | POST   | `/api/v1/auth/login`          | 透传        | auth-service `/v1/auth/login`         |
//! | POST   | `/api/v1/auth/refresh`        | 透传        | auth-service `/v1/auth/refresh`       |
//! | GET    | `/api/v1/projects`            | 透传        | project-service `/v1/projects`        |
//! | POST   | `/api/v1/translate/lookup`    | gRPC        | translation-core `MatchTM`            |
//! | POST   | `/api/v1/translate/commit`    | 占位        | proto `UpdateTMMatch` 暂未实装       |

use actix_web::{web, HttpRequest, HttpResponse};
use cats_proto::cats::v1::{LanguageCode, MatchTmRequest};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;

use crate::error::BffError;
use crate::grpc_clients::{TmCommitAck, TmLookupResponse, TranslationClient};
use crate::upstream_passthrough::UpstreamClient;

/// `GET /healthz` — 存活/就绪探针复用（K8s `livenessProbe` + `readinessProbe`）
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
}

pub async fn healthz() -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok",
        service: "cats-bff",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// `POST /api/v1/auth/login` — 透传到 auth-service
pub async fn auth_login(
    body: web::Json<Value>,
    upstream: web::Data<UpstreamClient>,
) -> Result<HttpResponse, BffError> {
    let (status, json) = upstream.forward_auth_login(&body.into_inner()).await?;
    Ok(proxy_status(status, json))
}

/// `POST /api/v1/auth/refresh` — 透传到 auth-service
pub async fn auth_refresh(
    body: web::Json<Value>,
    upstream: web::Data<UpstreamClient>,
) -> Result<HttpResponse, BffError> {
    let (status, json) = upstream.forward_auth_refresh(&body.into_inner()).await?;
    Ok(proxy_status(status, json))
}

/// `GET /api/v1/projects` — 透传 + X-Cats-* header 注入
pub async fn list_projects(
    req: HttpRequest,
    upstream: web::Data<UpstreamClient>,
) -> Result<HttpResponse, BffError> {
    let user_id = req
        .headers()
        .get("X-Cats-User-Id")
        .and_then(|h| h.to_str().ok());
    let org_id = req
        .headers()
        .get("X-Cats-Org-Id")
        .and_then(|h| h.to_str().ok());
    let auth_header = req
        .headers()
        .get(actix_web::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());
    let (status, json) = upstream
        .forward_project_list(user_id, org_id, auth_header)
        .await?;
    Ok(proxy_status(status, json))
}

/// `/api/v1/translate/lookup` HTTP 请求体
#[derive(Debug, Deserialize)]
pub struct LookupRequest {
    pub tenant_id: String,
    pub project_id: String,
    pub source_text: String,
    /// 源语言 BCP-47 字符串（如 `"en-US"`）；BFF 映射到 proto `LanguageCode` 枚举
    pub source_lang: String,
    /// 目标语言 BCP-47
    pub target_lang: String,
    /// 相似度阈值（0.0-1.0），缺省 0.7
    #[serde(default = "default_threshold")]
    pub threshold: f32,
}

fn default_threshold() -> f32 {
    0.7
}

/// `POST /api/v1/translate/lookup` — gRPC `MatchTM`
pub async fn translate_lookup(
    body: web::Json<LookupRequest>,
    grpc: web::Data<TranslationClient>,
) -> Result<HttpResponse, BffError> {
    let req = body.into_inner();
    if req.source_text.is_empty() {
        return Err(BffError::InvalidRequest(
            "source_text must not be empty".into(),
        ));
    }
    info!(
        tenant_id = %req.tenant_id,
        project_id = %req.project_id,
        source_lang = %req.source_lang,
        target_lang = %req.target_lang,
        "translate_lookup"
    );
    let proto_req = MatchTmRequest {
        tenant_id: req.tenant_id,
        project_id: req.project_id,
        source_text: req.source_text,
        // proto3 枚举的 Rust 字段类型是 `i32`（prost 的映射规则），不是
        // `LanguageCode` 本身 —— 与 translation-core 接线时同一个坑。
        source_lang: LanguageCode::from_str_name(&req.source_lang).unwrap_or_default() as i32,
        target_lang: LanguageCode::from_str_name(&req.target_lang).unwrap_or_default() as i32,
        threshold: req.threshold,
    };
    let resp: TmLookupResponse = grpc.match_tm(proto_req).await?;
    Ok(HttpResponse::Ok().json(resp))
}

/// `POST /api/v1/translate/commit` — proto 暂未实装
///
/// 任务规范要求 `TmService/Update`，但 `proto/cats/v1/translation_core.proto`
/// 当前没有 `UpdateTMMatch` / `CommitTM` RPC。BFF 阶段返回 501 Not Implemented，
/// 等 v1.1 加 RPC 后实装（详见 `TODO.md` §1）。
pub async fn translate_commit(body: web::Json<Value>) -> HttpResponse {
    info!(
        body = %body,
        "translate_commit called (proto UpdateTMMatch RPC pending v1.1)"
    );
    HttpResponse::NotImplemented().json(TmCommitAck {
        accepted: false,
        note: "proto UpdateTMMatch RPC pending v1.1; BFF endpoint is a stub",
    })
}

/// 把上游 HTTP status + JSON body 透传给客户端
///
/// 2026-10-05 接线：原签名收 `actix_web::http::StatusCode`，但
/// `UpstreamClient::forward_*` 返回的是 `reqwest::StatusCode` —— 两条平行设计
/// 各自 import 了不同的 HTTP crate，编译器在这里才把这件事挑出来。
///
/// 改成直接收 `reqwest::StatusCode` 并在函数内转换，这样三个调用点都不用改。
/// 转换走 `from_u16`（两边都是 u16 表示），失败时退回 502 —— 上游给出一个
/// actix 表示不了的状态码时，`proxy_status` 只会看到 `None`，不 panic。
fn proxy_status(status: reqwest::StatusCode, json: Value) -> HttpResponse {
    let code = actix_web::http::StatusCode::from_u16(status.as_u16())
        .unwrap_or(actix_web::http::StatusCode::BAD_GATEWAY);
    let mut builder = HttpResponse::build(code);
    builder.json(json)
}

/// 注册全部路由（lib.rs 调用 + 测试 bind 用）
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/healthz", web::get().to(healthz))
        .route("/api/v1/auth/login", web::post().to(auth_login))
        .route("/api/v1/auth/refresh", web::post().to(auth_refresh))
        .route("/api/v1/projects", web::get().to(list_projects))
        .route("/api/v1/translate/lookup", web::post().to(translate_lookup))
        .route("/api/v1/translate/commit", web::post().to(translate_commit));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_threshold_is_0_7() {
        assert!((default_threshold() - 0.7).abs() < f32::EPSILON);
    }
}
