//! 错误信封 + 错误类型定义
//!
//! 引用: doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md §1.3 (统一错误信封)
//! 引用: doc/05-其他/错误码/CATs_错误码表_v1.0.1.md §3.5/§3.6/§3.7 (错误码)
//!
//! MVP AI 网关涉及 5 类错误:
//! - VALIDATION_ERROR (400)
//! - PROVIDER_NOT_FOUND (502 / 兼容 UPSTREAM_ERROR, MVP 自定义)
//! - UPSTREAM_ERROR (502) — 重试后仍失败
//! - RATE_LIMITED (429) — 配额超限
//! - COMPLIANCE_BLOCKED (409) — 合规 fail-closed
//! - INTERNAL_ERROR (500)

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 错误码枚举 (跟 cats-common/接口设计 §1.4 对齐, AI 网关 MVP 自定义子集)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    /// 请求体校验失败
    #[serde(rename = "VALIDATION_ERROR")]
    ValidationError,
    /// Provider 不存在 (MVP 自定义, 接口设计未单独定义)
    #[serde(rename = "PROVIDER_NOT_FOUND")]
    ProviderNotFound,
    /// 配额超限 (per org_id)
    #[serde(rename = "RATE_LIMITED")]
    RateLimited,
    /// 依赖的下游服务/模型调用失败
    #[serde(rename = "UPSTREAM_ERROR")]
    UpstreamError,
    /// 合规 fail-closed
    #[serde(rename = "COMPLIANCE_BLOCKED")]
    ComplianceBlocked,
    /// 服务内部错误
    #[serde(rename = "INTERNAL_ERROR")]
    InternalError,
}

impl ErrorCode {
    /// 对应 HTTP 状态码
    pub fn http_status(self) -> u16 {
        match self {
            Self::ValidationError     => 400,
            Self::ProviderNotFound    => 502,
            Self::RateLimited         => 429,
            Self::UpstreamError       => 502,
            Self::ComplianceBlocked   => 409,
            Self::InternalError       => 500,
        }
    }

    /// 对应 gRPC status (per 接口设计 §1.4 映射)
    pub fn grpc_status(self) -> &'static str {
        match self {
            Self::ValidationError   => "INVALID_ARGUMENT",
            Self::ProviderNotFound  => "UNAVAILABLE",
            Self::RateLimited       => "RESOURCE_EXHAUSTED",
            Self::UpstreamError     => "UNAVAILABLE",
            Self::ComplianceBlocked => "FAILED_PRECONDITION",
            Self::InternalError     => "INTERNAL",
        }
    }
}

/// 错误明细
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    /// 错误码 (SCREAMING_SNAKE_CASE)
    pub code: ErrorCode,
    /// 人类可读错误消息
    pub message: String,
    /// 链路追踪 ID (UUID v4)
    pub trace_id: String,
    /// 附加上下文 (per provider / per org 等)
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub details: serde_json::Map<String, serde_json::Value>,
}

/// 错误信封 (per 接口设计 §1.3)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

impl ErrorEnvelope {
    /// 构造错误信封
    pub fn new(code: ErrorCode, message: impl Into<String>, trace_id: impl Into<String>) -> Self {
        Self {
            error: ErrorBody {
                code,
                message: message.into(),
                trace_id: trace_id.into(),
                details: serde_json::Map::new(),
            },
        }
    }

    /// 附加 details KV
    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.error.details.insert(key.into(), value.into());
        self
    }
}

/// AI 网关内部错误类型
#[derive(Debug, Error)]
pub enum ProviderError {
    /// Provider 上游错误 (网络/超时/模型本身)
    #[error("upstream provider {provider} error: {message}")]
    Upstream {
        provider: String,
        message: String,
    },

    /// Provider 抛出的可重试错误 (5xx / timeout)
    #[error("transient provider {provider} error: {message}")]
    Transient {
        provider: String,
        message: String,
    },

    /// 配额超限 (per org_id)
    #[error("rate limited for org {org_id}: {message}")]
    RateLimited {
        org_id: String,
        message: String,
        /// 客户端 Retry-After (秒)
        retry_after_secs: u32,
    },

    /// 合规 fail-closed
    #[error("compliance blocked: {message}")]
    ComplianceBlocked {
        message: String,
        /// 合规模式 ("cloud" / "local")
        mode: String,
    },

    /// Provider 不存在
    #[error("provider not found: {0}")]
    ProviderNotFound(String),

    /// 所有 provider 都失败
    #[error("all providers failed after retry: {0}")]
    AllFailed(String),

    /// 请求体校验失败
    #[error("validation error: {0}")]
    Validation(String),
}

impl ProviderError {
    /// 是否可重试
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Transient { .. })
    }

    /// 转为 ErrorEnvelope (HTTP body 用)
    pub fn to_envelope(&self, trace_id: &str) -> ErrorEnvelope {
        let code = match self {
            Self::Upstream { .. }        => ErrorCode::UpstreamError,
            Self::Transient { .. }       => ErrorCode::UpstreamError, // 重试耗尽后归 UPSTREAM_ERROR
            Self::RateLimited { .. }     => ErrorCode::RateLimited,
            Self::ComplianceBlocked { .. } => ErrorCode::ComplianceBlocked,
            Self::ProviderNotFound(_)    => ErrorCode::ProviderNotFound,
            Self::AllFailed(_)           => ErrorCode::UpstreamError,
            Self::Validation(_)          => ErrorCode::ValidationError,
        };
        let msg = self.to_string();
        let env = ErrorEnvelope::new(code, msg, trace_id.to_string());
        match self {
            Self::RateLimited { org_id, retry_after_secs, .. } => env
                .with_detail("org_id", org_id.clone())
                .with_detail("retry_after_secs", *retry_after_secs as i64),
            Self::ComplianceBlocked { mode, .. } => env
                .with_detail("compliance_mode", mode.clone()),
            Self::ProviderNotFound(p) => env
                .with_detail("provider", p.clone()),
            Self::AllFailed(msg) => env
                .with_detail("reason", msg.clone()),
            _ => env,
        }
    }

    /// HTTP 状态码 (用于 actix-web ResponseError)
    pub fn http_status(&self) -> u16 {
        match self {
            Self::Validation(_)         => 400,
            Self::RateLimited { .. }    => 429,
            Self::ComplianceBlocked { .. } => 409,
            Self::ProviderNotFound(_)   => 502,
            Self::AllFailed(_)          => 502,
            Self::Upstream { .. }       => 502,
            Self::Transient { .. }      => 502,
        }
    }
}

/// 实现 actix-web ResponseError
impl actix_web::ResponseError for ProviderError {
    fn error_response(&self) -> actix_web::HttpResponse {
        let trace_id = uuid::Uuid::new_v4().to_string();
        let envelope = self.to_envelope(&trace_id);
        let status = actix_web::http::StatusCode::from_u16(self.http_status())
            .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);
        let mut resp = actix_web::HttpResponse::build(status);
        if let Self::RateLimited { retry_after_secs, .. } = self {
            resp.insert_header(("Retry-After", retry_after_secs.to_string()));
        }
        resp.json(envelope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_http_status_mapping() {
        assert_eq!(ErrorCode::ValidationError.http_status(), 400);
        assert_eq!(ErrorCode::RateLimited.http_status(), 429);
        assert_eq!(ErrorCode::ComplianceBlocked.http_status(), 409);
        assert_eq!(ErrorCode::UpstreamError.http_status(), 502);
        assert_eq!(ErrorCode::ProviderNotFound.http_status(), 502);
        assert_eq!(ErrorCode::InternalError.http_status(), 500);
    }

    #[test]
    fn error_code_grpc_status_mapping() {
        assert_eq!(ErrorCode::ValidationError.grpc_status(), "INVALID_ARGUMENT");
        assert_eq!(ErrorCode::RateLimited.grpc_status(), "RESOURCE_EXHAUSTED");
        assert_eq!(ErrorCode::ComplianceBlocked.grpc_status(), "FAILED_PRECONDITION");
    }

    #[test]
    fn error_envelope_serializes_to_interface_v1_3_schema() {
        // per 接口设计 §1.3
        let env = ErrorEnvelope::new(ErrorCode::ValidationError, "model is empty", "trace-123")
            .with_detail("field", "model");
        let s = serde_json::to_string(&env).unwrap();
        assert!(s.contains("\"code\":\"VALIDATION_ERROR\""));
        assert!(s.contains("\"trace_id\":\"trace-123\""));
        assert!(s.contains("\"field\":\"model\""));
    }

    #[test]
    fn rate_limited_sets_retry_after() {
        let e = ProviderError::RateLimited {
            org_id: "org-1".into(),
            message: "quota exceeded".into(),
            retry_after_secs: 30,
        };
        assert_eq!(e.http_status(), 429);
        assert!(!e.is_retryable());
    }

    #[test]
    fn transient_is_retryable() {
        let e = ProviderError::Transient {
            provider: "openai".into(),
            message: "5xx".into(),
        };
        assert!(e.is_retryable());
    }
}