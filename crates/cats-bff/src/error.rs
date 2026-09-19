//! `cats-bff` 统一错误类型 + JSON 错误响应
//!
//! BFF 自身不持有业务真相，所有错误归类为以下 3 类：
//! - `Upstream(String)` — 下游 service（auth / project / translation-core）调用失败
//! - `BadRequest(String)` — 客户端请求校验失败（参数缺失/JSON 格式错）
//! - `Unauthorized(String)` — 缺失/无效 JWT（透传场景下，401 是上游业务错误）
//!
//! HTTP 状态码映射（per 接口设计书 v2.0 §3.5.2）：
//! - `BadRequest`   → 400
//! - `Unauthorized` → 401
//! - `Upstream`     → 502（依赖不可用）/ 504（超时）
//!
//! 错误响应 body 沿用接口书 §3.5.1 v2.0+2 三字段 schema：
//! ```json
//! { "error": "<CODE>", "message": "<human>", "detail": "<optional context>" }
//! ```

use actix_web::{http::StatusCode as HttpStatus, HttpResponse, ResponseError};
use serde::Serialize;
use thiserror::Error;

/// BFF 业务错误
#[derive(Debug, Error)]
pub enum BffError {
    /// 上游 service 调用失败（502 Bad Gateway）
    #[error("upstream error: {0}")]
    Upstream(String),

    /// 上游超时（504 Gateway Timeout）
    #[error("upstream timeout: {0}")]
    UpstreamTimeout(String),

    /// 客户端请求校验失败（400）
    #[error("bad request: {0}")]
    BadRequest(String),

    /// 鉴权失败（401）
    #[error("unauthorized: {0}")]
    Unauthorized(String),
}

/// 错误响应 body（per 接口设计书 v2.0+2 §3.5.1）
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    /// 业务错误码（snake_case）；BFF 阶段先用 4 个聚合码 + 透传上游 error.code
    pub error: String,
    /// 人类可读消息（en-US 前缀，per 错误码表 §2.1）
    pub message: String,
    /// 可选上下文；不暴露 secret / PII
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl BffError {
    /// 构造 `ErrorBody` JSON 序列化体
    pub fn to_body(&self) -> ErrorBody {
        match self {
            Self::BadRequest(msg) => ErrorBody {
                error: "VALIDATION_ERROR".to_string(),
                message: msg.clone(),
                detail: None,
            },
            Self::Unauthorized(msg) => ErrorBody {
                error: "UNAUTHORIZED".to_string(),
                message: msg.clone(),
                detail: None,
            },
            Self::Upstream(msg) => ErrorBody {
                error: "UPSTREAM_ERROR".to_string(),
                message: "upstream service call failed".to_string(),
                detail: Some(msg.clone()),
            },
            Self::UpstreamTimeout(msg) => ErrorBody {
                error: "UPSTREAM_TIMEOUT".to_string(),
                message: "upstream service call timed out".to_string(),
                detail: Some(msg.clone()),
            },
        }
    }
}

impl ResponseError for BffError {
    fn status_code(&self) -> HttpStatus {
        match self {
            Self::BadRequest(_) => HttpStatus::BAD_REQUEST,
            Self::Unauthorized(_) => HttpStatus::UNAUTHORIZED,
            Self::Upstream(_) => HttpStatus::BAD_GATEWAY,
            Self::UpstreamTimeout(_) => HttpStatus::GATEWAY_TIMEOUT,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(self.to_body())
    }
}

impl From<reqwest::Error> for BffError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            Self::UpstreamTimeout(format!("reqwest timeout: {e}"))
        } else {
            Self::Upstream(format!("reqwest: {e}"))
        }
    }
}

impl From<tonic::transport::Error> for BffError {
    fn from(e: tonic::transport::Error) -> Self {
        Self::Upstream(format!("tonic transport: {e}"))
    }
}

impl From<tonic::Status> for BffError {
    fn from(s: tonic::Status) -> Self {
        // 业务错误从 status.code() 映射；保守按 upstream 502 处理（MVP 阶段不做 gRPC status → http 精细映射，
        // 真要映射见接口设计书 v2.0 §3.5.2）
        Self::Upstream(format!("tonic status: code={:?} message={}", s.code(), s.message()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_request_maps_to_400() {
        let e = BffError::BadRequest("missing field".into());
        assert_eq!(e.status_code(), HttpStatus::BAD_REQUEST);
        let body = e.to_body();
        assert_eq!(body.error, "VALIDATION_ERROR");
        assert_eq!(body.message, "missing field");
        assert!(body.detail.is_none());
    }

    #[test]
    fn unauthorized_maps_to_401() {
        let e = BffError::Unauthorized("token missing".into());
        assert_eq!(e.status_code(), HttpStatus::UNAUTHORIZED);
        assert_eq!(e.to_body().error, "UNAUTHORIZED");
    }

    #[test]
    fn upstream_maps_to_502() {
        let e = BffError::Upstream("connection refused".into());
        assert_eq!(e.status_code(), HttpStatus::BAD_GATEWAY);
        assert_eq!(e.to_body().error, "UPSTREAM_ERROR");
        assert_eq!(e.to_body().detail.as_deref(), Some("connection refused"));
    }

    #[test]
    fn upstream_timeout_maps_to_504() {
        let e = BffError::UpstreamTimeout("after 5s".into());
        assert_eq!(e.status_code(), HttpStatus::GATEWAY_TIMEOUT);
        assert_eq!(e.to_body().error, "UPSTREAM_TIMEOUT");
    }
}