//! cats-bff 统一错误信封 (per 接口设计书 v2.0 §1.3 + OpenAPI v1.0.1 ErrorBody schema)
//!
//! 设计:
//! - 字段: error (snake_case code) + message (i18n 前缀, 默认 en-US) + detail (可选上下文)
//! - HTTP status 由 error code 决定 (per 错误码表 v1.0 §3)
//! - 上游 service 失败 → 502 dependency_unavailable / 504 dependency_timeout
//! - BFF 自身失败 → 500 server_error / 400 invalid_request
//!
//! 与 crates/auth-service/src/models.rs ErrorBody (3 字段) 保持一致 (per OpenAPI v1.0.1 §schemas.ErrorBody)

use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::Serialize;
use std::fmt;
use thiserror::Error;

/// 错误码 (snake_case, machine-readable, per 错误码表 v1.0 §3 28 条)
///
/// 真实落地以本枚举 + 错误码表为 single source of truth (per §6.3 实施要求)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    // 客户端请求错误
    InvalidRequest,
    InvalidPayload,
    InvalidHeader,
    InvalidQuery,
    UnsupportedVersion,
    // 鉴权错误
    InvalidCredentials,
    InvalidToken,
    TokenExpired,
    TokenRevoked,
    InvalidTokenType,
    MissingAuthorization,
    InvalidAuthorizationScheme,
    UserInactive,
    // 资源错误
    UserNotFound,
    TokenNotFound,
    ResourceNotFound,
    UsernameConflict,
    EmailConflict,
    TokenAlreadyRevoked,
    // 限流 / 配额
    RateLimited,
    QuotaExceeded,
    // 服务端错误
    ServerError,
    ServerMisconfigured,
    DependencyUnavailable,
    DependencyTimeout,
    // 业务规则
    PasswordMismatch,
    OperationNotPermitted,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "server_error".to_string())
            .leak() // 静态字符串, 生命周期 = 'static
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// BFF 错误响应体 (3 字段, per OpenAPI v1.0.1 ErrorBody schema)
#[derive(Debug, Clone, Serialize)]
pub struct ErrorBody {
    pub error: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// BFF 错误类型 (聚合上游失败 / RBAC 拒绝 / 解析失败 / 业务错误)
#[derive(Debug, Error)]
pub enum BffError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("unauthorized: {0}")]
    Unauthorized(&'static str),

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("dependency unavailable: {0}")]
    DependencyUnavailable(String),

    #[error("dependency timeout")]
    DependencyTimeout,

    #[error("upstream error (status={status}): {body}")]
    UpstreamError { status: u16, body: String },

    #[error("server misconfigured: {0}")]
    ServerMisconfigured(String),

    #[error("internal server error: {0}")]
    Internal(String),
}

impl BffError {
    fn to_body(&self) -> ErrorBody {
        match self {
            BffError::InvalidRequest(m) => ErrorBody {
                error: ErrorCode::InvalidRequest,
                message: m.clone(),
                detail: None,
            },
            BffError::Unauthorized(code) => ErrorBody {
                error: match *code {
                    "missing_authorization" => ErrorCode::MissingAuthorization,
                    "invalid_token" => ErrorCode::InvalidToken,
                    "token_expired" => ErrorCode::TokenExpired,
                    "invalid_token_type" => ErrorCode::InvalidTokenType,
                    "invalid_credentials" => ErrorCode::InvalidCredentials,
                    _ => ErrorCode::MissingAuthorization,
                },
                message: "authentication required".to_string(),
                detail: None,
            },
            BffError::Forbidden(m) => ErrorBody {
                error: ErrorCode::OperationNotPermitted,
                message: "operation not permitted".to_string(),
                detail: Some(m.clone()),
            },
            BffError::NotFound(m) => ErrorBody {
                error: ErrorCode::ResourceNotFound,
                message: "resource not found".to_string(),
                detail: Some(m.clone()),
            },
            BffError::Conflict(m) => ErrorBody {
                error: ErrorCode::UsernameConflict,
                message: "resource conflict".to_string(),
                detail: Some(m.clone()),
            },
            BffError::DependencyUnavailable(m) => ErrorBody {
                error: ErrorCode::DependencyUnavailable,
                message: "upstream service unavailable".to_string(),
                detail: Some(m.clone()),
            },
            BffError::DependencyTimeout => ErrorBody {
                error: ErrorCode::DependencyTimeout,
                message: "upstream service timeout".to_string(),
                detail: None,
            },
            BffError::UpstreamError { status, body } => ErrorBody {
                error: ErrorCode::DependencyUnavailable,
                message: format!("upstream returned {status}"),
                detail: Some(body.clone()),
            },
            BffError::ServerMisconfigured(m) => ErrorBody {
                error: ErrorCode::ServerMisconfigured,
                message: "server misconfigured".to_string(),
                detail: Some(m.clone()),
            },
            BffError::Internal(m) => ErrorBody {
                error: ErrorCode::ServerError,
                message: "internal server error".to_string(),
                detail: Some(m.clone()),
            },
        }
    }
}

impl ResponseError for BffError {
    fn status_code(&self) -> StatusCode {
        match self {
            BffError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            BffError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            BffError::Forbidden(_) => StatusCode::FORBIDDEN,
            BffError::NotFound(_) => StatusCode::NOT_FOUND,
            BffError::Conflict(_) => StatusCode::CONFLICT,
            BffError::DependencyUnavailable(_)
            | BffError::UpstreamError { .. }
            | BffError::DependencyTimeout => StatusCode::BAD_GATEWAY,
            BffError::ServerMisconfigured(_) => StatusCode::INTERNAL_SERVER_ERROR,
            BffError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(self.to_body())
    }
}

impl From<reqwest::Error> for BffError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            BffError::DependencyTimeout
        } else if err.is_connect() {
            BffError::DependencyUnavailable(err.to_string())
        } else {
            BffError::DependencyUnavailable(err.to_string())
        }
    }
}

impl From<serde_json::Error> for BffError {
    fn from(err: serde_json::Error) -> Self {
        BffError::InvalidRequest(format!("invalid json: {err}"))
    }
}

pub type BffResult<T> = Result<T, BffError>;
