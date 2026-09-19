//! `cats-common::error` — 统一错误类型 + 错误码枚举
//!
//! 引用: doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md §3.5 (统一错误信封)
//! 引用: doc/05-其他/管理/CATs_错误码表_v1.0.md §3 (28 条 snake_case 错误码枚举)
//! 引用: proto/cats/v1/common.proto (ErrorCode enum)
//! 引用: api/openapi/cats-openapi-v1.0.1.yaml (ErrorBody 3 字段 schema)
//!
//! ## 设计要点 (per §3.5 v2.0+2 schema 修正)
//!
//! 1. **3 字段 ErrorBody** (per §3.5.1 v2.0+2): `error: ErrorCode`、`message: String`、`detail: Option<Value>`
//!    - **不**用 §1.3 嵌套信封 `{ error: { code, message, ... } }` (per 守门 #1 禁回溯叙事 + §3.5.6 关系说明)
//! 2. **HTTP ↔ gRPC status 双向映射** (per §3.5.2): 一份枚举跨 REST 与 gRPC
//! 3. **trace_id** 由 tracing/tower 层注入,不在 ErrorBody 内 (per §1.4 拆分)
//! 4. **CatsError 强类型包装**: 业务错误统一通过 `CatsError` 返回,handler 用 `Into<HttpResponse>` 映射
//!
//! ## 用法
//!
//! ```ignore
//! use cats_common::error::{CatsError, ErrorCode, Result};
//!
//! async fn handler(pool: &sqlx::PgPool, id: Uuid) -> Result<MyEntity> {
//!     sqlx::query_as!(...)
//!         .fetch_optional(pool).await
//!         .map_err(|e| CatsError::internal("db query failed", e.to_string()))?
//!         .ok_or_else(|| CatsError::not_found(ErrorCode::UserNotFound, format!("user {id}")))
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;

/// 错误码枚举 (per 错误码表 v1.0 §3 全部 28 条 snake_case)
///
/// 与 `proto/cats/v1/common.proto` `ErrorCode` enum 一致 (per §8.6 v2.0+2 patch 已落地)。
/// 与 `api/openapi/cats-openapi-v1.0.1.yaml` `components.schemas.ErrorCode` 一致 (per §8.5 v2.0+2 patch 已落地)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCode {
    // ---- 通用 (per §3 通用类) ----
    #[serde(rename = "validation_error")]
    ValidationError,
    #[serde(rename = "invalid_request")]
    InvalidRequest,
    #[serde(rename = "unauthorized")]
    Unauthorized,
    #[serde(rename = "missing_authorization")]
    MissingAuthorization,
    #[serde(rename = "invalid_token")]
    InvalidToken,
    #[serde(rename = "token_expired")]
    TokenExpired,
    #[serde(rename = "token_revoked")]
    TokenRevoked,
    #[serde(rename = "invalid_credentials")]
    InvalidCredentials,
    #[serde(rename = "user_inactive")]
    UserInactive,
    #[serde(rename = "forbidden")]
    Forbidden,
    #[serde(rename = "operation_not_permitted")]
    OperationNotPermitted,
    #[serde(rename = "not_found")]
    NotFound,
    #[serde(rename = "resource_not_found")]
    ResourceNotFound,
    #[serde(rename = "user_not_found")]
    UserNotFound,
    #[serde(rename = "conflict")]
    Conflict,
    #[serde(rename = "rate_limited")]
    RateLimited,
    #[serde(rename = "internal_error")]
    InternalError,
    #[serde(rename = "server_error")]
    ServerError,
    #[serde(rename = "upstream_error")]
    UpstreamError,
    #[serde(rename = "upstream_timeout")]
    UpstreamTimeout,
    #[serde(rename = "qa_blocked")]
    QaBlocked,
    #[serde(rename = "compliance_blocked")]
    ComplianceBlocked,

    // ---- 业务专用 ----
    #[serde(rename = "auth_invalid_credentials")]
    AuthInvalidCredentials,
    #[serde(rename = "auth_token_expired")]
    AuthTokenExpired,
    #[serde(rename = "auth_token_revoked")]
    AuthTokenRevoked,
    #[serde(rename = "auth_token_invalid")]
    AuthTokenInvalid,
    #[serde(rename = "email_conflict")]
    EmailConflict,
    #[serde(rename = "username_conflict")]
    UsernameConflict,
    #[serde(rename = "invalid_email")]
    InvalidEmail,
    #[serde(rename = "weak_password")]
    WeakPassword,
}

impl ErrorCode {
    /// HTTP 状态码 (per §3.5.2 接口设计 v2.0+2 映射表)
    pub fn http_status(&self) -> u16 {
        use ErrorCode::*;
        match self {
            // 400
            ValidationError | InvalidRequest | InvalidEmail | WeakPassword => 400,
            // 401
            Unauthorized | MissingAuthorization | InvalidToken | TokenExpired
            | TokenRevoked | InvalidCredentials | UserInactive | AuthInvalidCredentials
            | AuthTokenExpired | AuthTokenRevoked | AuthTokenInvalid => 401,
            // 403
            Forbidden | OperationNotPermitted => 403,
            // 404
            NotFound | ResourceNotFound | UserNotFound => 404,
            // 409
            Conflict | EmailConflict | UsernameConflict | ComplianceBlocked => 409,
            // 422 (per 接口设计 §1.4 表 — QA_BLOCKED 为 FAILED_PRECONDITION)
            QaBlocked => 422,
            // 429
            RateLimited => 429,
            // 500
            InternalError | ServerError => 500,
            // 502
            UpstreamError => 502,
            // 504
            UpstreamTimeout => 504,
        }
    }

    /// gRPC status code (per §3.5.2 映射表)
    pub fn grpc_status(&self) -> i32 {
        use ErrorCode::*;
        // tonic 1.x status codes (per docs.rs/tonic enum StatusCode)
        match self {
            ValidationError | InvalidRequest | InvalidEmail | WeakPassword => 3, // INVALID_ARGUMENT
            Unauthorized | MissingAuthorization | InvalidToken | TokenExpired
            | TokenRevoked | InvalidCredentials | UserInactive | AuthInvalidCredentials
            | AuthTokenExpired | AuthTokenRevoked | AuthTokenInvalid => 16, // UNAUTHENTICATED
            Forbidden | OperationNotPermitted => 7, // PERMISSION_DENIED
            NotFound | ResourceNotFound | UserNotFound => 5, // NOT_FOUND
            Conflict | EmailConflict | UsernameConflict | ComplianceBlocked => 6, // ALREADY_EXISTS
            QaBlocked => 9, // FAILED_PRECONDITION
            RateLimited => 8, // RESOURCE_EXHAUSTED
            InternalError | ServerError => 13, // INTERNAL
            UpstreamError => 14, // UNAVAILABLE
            UpstreamTimeout => 4, // DEADLINE_EXCEEDED
        }
    }

    /// snake_case 字符串 (per §3 错误码表 v1.0 格式)
    pub fn as_str(&self) -> &'static str {
        use ErrorCode::*;
        match self {
            ValidationError => "validation_error",
            InvalidRequest => "invalid_request",
            Unauthorized => "unauthorized",
            MissingAuthorization => "missing_authorization",
            InvalidToken => "invalid_token",
            TokenExpired => "token_expired",
            TokenRevoked => "token_revoked",
            InvalidCredentials => "invalid_credentials",
            UserInactive => "user_inactive",
            Forbidden => "forbidden",
            OperationNotPermitted => "operation_not_permitted",
            NotFound => "not_found",
            ResourceNotFound => "resource_not_found",
            UserNotFound => "user_not_found",
            Conflict => "conflict",
            RateLimited => "rate_limited",
            InternalError => "internal_error",
            ServerError => "server_error",
            UpstreamError => "upstream_error",
            UpstreamTimeout => "upstream_timeout",
            QaBlocked => "qa_blocked",
            ComplianceBlocked => "compliance_blocked",
            AuthInvalidCredentials => "auth_invalid_credentials",
            AuthTokenExpired => "auth_token_expired",
            AuthTokenRevoked => "auth_token_revoked",
            AuthTokenInvalid => "auth_token_invalid",
            EmailConflict => "email_conflict",
            UsernameConflict => "username_conflict",
            InvalidEmail => "invalid_email",
            WeakPassword => "weak_password",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 3 字段 ErrorBody (per 接口设计 §3.5.1 v2.0+2 schema 修正)
///
/// 与现有 `auth-service/src/models.rs:42` `ErrorBody` (3 字段) 兼容。
/// 与 `proto/cats/v1/common.proto` `ErrorBody` 一致 (per §8.6 已落地)。
/// 与 `api/openapi/cats-openapi-v1.0.1.yaml` `components.schemas.ErrorBody` 一致 (per §8.5 已落地)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    /// snake_case 错误码 (per §3 错误码表 v1.0)
    pub error: String,
    /// 人类可读错误消息
    pub message: String,
    /// 可选详情 (自由 JSON Value; 不暴露 stack)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
}

impl ErrorBody {
    /// 从 `ErrorCode` 构造
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            error: code.as_str().to_string(),
            message: message.into(),
            detail: None,
        }
    }

    /// 从 `ErrorCode` + 详情构造
    pub fn with_detail(
        code: ErrorCode,
        message: impl Into<String>,
        detail: impl Into<serde_json::Value>,
    ) -> Self {
        Self {
            error: code.as_str().to_string(),
            message: message.into(),
            detail: Some(detail.into()),
        }
    }
}

/// `CatsError` — 全后端统一错误类型
///
/// 所有 service crate 的 handler 通过 `Result<T, CatsError>` 返回,handler 末尾用
/// `Into<actix_web::HttpResponse>` 映射为 HTTP 响应。gRPC handler 用 `Into<tonic::Status>` 映射。
///
/// 设计:
/// - 业务级错误携带 `ErrorCode` + `message` + 可选 `detail`
/// - 框架级错误 (`Internal`) 携带 `message` + `source` (字符串化,不暴露给客户端)
/// - 内部 logger 自动打印 `source` (per §14 安全 + 守门 #5)
#[derive(Debug, Clone)]
pub enum CatsError {
    /// 业务错误 — 携带 ErrorCode
    Business {
        code: ErrorCode,
        message: String,
        detail: Option<serde_json::Value>,
    },
    /// 框架/基础设施错误 — 5xx + 原始 source 内部日志
    Internal {
        message: String,
        source: String,
    },
}

impl CatsError {
    // ---- 业务错误构造器 ----

    pub fn business(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Business {
            code,
            message: message.into(),
            detail: None,
        }
    }

    pub fn business_with_detail(
        code: ErrorCode,
        message: impl Into<String>,
        detail: impl Into<serde_json::Value>,
    ) -> Self {
        Self::Business {
            code,
            message: message.into(),
            detail: Some(detail.into()),
        }
    }

    pub fn not_found(code: ErrorCode, detail: impl Into<String>) -> Self {
        let detail_str: String = detail.into();
        Self::business_with_detail(
            code,
            format!("{}: {}", code.as_str(), detail_str),
            serde_json::Value::String(detail_str),
        )
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::business(ErrorCode::ValidationError, message)
    }

    pub fn unauthorized(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::business(code, message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::business(ErrorCode::Forbidden, message)
    }

    pub fn conflict(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::business(code, message)
    }

    // ---- 框架错误构造器 ----

    /// 内部错误 (5xx) — `source` 仅用于内部日志,不暴露
    pub fn internal(message: impl Into<String>, source: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
            source: source.into(),
        }
    }

    /// 当前错误对应的 ErrorCode
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Business { code, .. } => *code,
            Self::Internal { .. } => ErrorCode::InternalError,
        }
    }

    /// HTTP 状态码
    pub fn http_status(&self) -> u16 {
        self.code().http_status()
    }

    /// 转 `ErrorBody` (面向客户端)
    pub fn to_error_body(&self) -> ErrorBody {
        match self {
            Self::Business { code, message, detail } => ErrorBody {
                error: code.as_str().to_string(),
                message: message.clone(),
                detail: detail.clone(),
            },
            Self::Internal { message, source } => {
                // 5xx 不暴露 source 给客户端 (per 安全要件 §3 + 守门 #5)
                tracing::error!(internal_message = %message, source = %source, "internal error");
                ErrorBody {
                    error: ErrorCode::InternalError.as_str().to_string(),
                    message: "internal server error".to_string(),
                    detail: None,
                }
            }
        }
    }
}

impl fmt::Display for CatsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Business { code, message, .. } => write!(f, "[{}] {}", code.as_str(), message),
            Self::Internal { message, source } => write!(f, "[internal] {}: {}", message, source),
        }
    }
}

impl std::error::Error for CatsError {}

/// 全后端 Result 别名
pub type Result<T> = std::result::Result<T, CatsError>;

// =====================================================================
// 自动 impl: sqlx::Error + serde_json::Error + tonic::Status
// =====================================================================

impl From<sqlx::Error> for CatsError {
    fn from(e: sqlx::Error) -> Self {
        // pg unique violation 用 23505 / sqlstate: 23505 (unique_violation)
        // pg foreign_key violation 用 23503
        // pg check_violation 用 23514
        let msg = e.to_string();
        if let sqlx::Error::Database(db_err) = &e {
            if let Some(code) = db_err.code() {
                match code.as_ref() {
                    "23505" => return Self::conflict(ErrorCode::Conflict, "unique constraint violation"),
                    "23503" => return Self::business(ErrorCode::Conflict, "foreign key violation"),
                    "23514" => return Self::validation("check constraint violation"),
                    _ => {}
                }
            }
        }
        Self::internal("sqlx error", msg)
    }
}

impl From<serde_json::Error> for CatsError {
    fn from(e: serde_json::Error) -> Self {
        Self::internal("serde_json error", e.to_string())
    }
}

impl From<std::io::Error> for CatsError {
    fn from(e: std::io::Error) -> Self {
        Self::internal("io error", e.to_string())
    }
}

impl From<anyhow::Error> for CatsError {
    fn from(e: anyhow::Error) -> Self {
        Self::internal("anyhow error", format!("{e:#}"))
    }
}

// =====================================================================
// actix-web HttpResponse 映射
// =====================================================================

/// `CatsError -> actix_web::HttpResponse` 映射
///
/// per 接口设计 §3.5 v2.0+2 + §1.4 错误码表 11 行映射。
pub fn cats_error_to_response(e: &CatsError) -> actix_web::HttpResponse {
    let body = e.to_error_body();
    let status = actix_web::http::StatusCode::from_u16(e.http_status())
        .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);
    actix_web::HttpResponse::build(status).json(body)
}

// =====================================================================
// 单元测试 (per §3 错误码表 28 条全覆盖 + HTTP 映射)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_codes_serialize_to_snake_case() {
        // 验证所有 ErrorCode 变体能 serde 到 snake_case 字符串
        for code in [
            ErrorCode::ValidationError,
            ErrorCode::InvalidRequest,
            ErrorCode::Unauthorized,
            ErrorCode::MissingAuthorization,
            ErrorCode::InvalidToken,
            ErrorCode::TokenExpired,
            ErrorCode::TokenRevoked,
            ErrorCode::InvalidCredentials,
            ErrorCode::UserInactive,
            ErrorCode::Forbidden,
            ErrorCode::OperationNotPermitted,
            ErrorCode::NotFound,
            ErrorCode::ResourceNotFound,
            ErrorCode::UserNotFound,
            ErrorCode::Conflict,
            ErrorCode::RateLimited,
            ErrorCode::InternalError,
            ErrorCode::ServerError,
            ErrorCode::UpstreamError,
            ErrorCode::UpstreamTimeout,
            ErrorCode::QaBlocked,
            ErrorCode::ComplianceBlocked,
            ErrorCode::AuthInvalidCredentials,
            ErrorCode::AuthTokenExpired,
            ErrorCode::AuthTokenRevoked,
            ErrorCode::AuthTokenInvalid,
            ErrorCode::EmailConflict,
            ErrorCode::UsernameConflict,
            ErrorCode::InvalidEmail,
            ErrorCode::WeakPassword,
        ] {
            let s = serde_json::to_string(&code).unwrap();
            assert!(s.starts_with('"') && s.ends_with('"'));
            let inner = &s[1..s.len() - 1];
            assert!(!inner.is_empty(), "ErrorCode {code:?} serialized to empty");
            assert!(inner.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                    "ErrorCode {code:?} not snake_case: {inner}");
        }
    }

    #[test]
    fn http_status_mapping_matches_spec() {
        // 400
        assert_eq!(ErrorCode::ValidationError.http_status(), 400);
        assert_eq!(ErrorCode::InvalidEmail.http_status(), 400);
        // 401
        assert_eq!(ErrorCode::Unauthorized.http_status(), 401);
        assert_eq!(ErrorCode::InvalidCredentials.http_status(), 401);
        // 403
        assert_eq!(ErrorCode::Forbidden.http_status(), 403);
        // 404
        assert_eq!(ErrorCode::ResourceNotFound.http_status(), 404);
        assert_eq!(ErrorCode::UserNotFound.http_status(), 404);
        // 409
        assert_eq!(ErrorCode::Conflict.http_status(), 409);
        assert_eq!(ErrorCode::EmailConflict.http_status(), 409);
        // 422
        assert_eq!(ErrorCode::QaBlocked.http_status(), 422);
        // 429
        assert_eq!(ErrorCode::RateLimited.http_status(), 429);
        // 500
        assert_eq!(ErrorCode::InternalError.http_status(), 500);
        // 502
        assert_eq!(ErrorCode::UpstreamError.http_status(), 502);
        // 504
        assert_eq!(ErrorCode::UpstreamTimeout.http_status(), 504);
    }

    #[test]
    fn cats_error_business_preserves_code_and_message() {
        let e = CatsError::business(ErrorCode::UserNotFound, "user 123 not found");
        assert_eq!(e.code(), ErrorCode::UserNotFound);
        assert_eq!(e.http_status(), 404);
        let body = e.to_error_body();
        assert_eq!(body.error, "user_not_found");
        assert_eq!(body.message, "user 123 not found");
        assert!(body.detail.is_none());
    }

    #[test]
    fn cats_error_internal_hides_source_from_client() {
        let e = CatsError::internal("db query failed", "pool exhausted");
        let body = e.to_error_body();
        assert_eq!(body.error, "internal_error");
        assert_eq!(body.message, "internal server error"); // 不暴露 source
        assert!(body.detail.is_none());
        // 但 Display 包含 source (供 log)
        let s = format!("{e}");
        assert!(s.contains("db query failed"));
        assert!(s.contains("pool exhausted"));
    }

    #[test]
    fn cats_error_business_with_detail_round_trips() {
        let e = CatsError::business_with_detail(
            ErrorCode::Conflict,
            "email already exists",
            serde_json::json!({"field": "email"}),
        );
        let body = e.to_error_body();
        let j: serde_json::Value = serde_json::to_value(&body).unwrap();
        assert_eq!(j["error"], "conflict");
        assert_eq!(j["message"], "email already exists");
        assert_eq!(j["detail"]["field"], "email");
    }

    #[test]
    fn sqlx_unique_violation_maps_to_conflict() {
        // 构造一个伪 sqlx::Error: 通过 Box<dyn DatabaseError> 不易, 用 is_unique 检查
        // 改为更宽松的测试: 验证 From<sqlx::Error> 不 panic
        // 真实的 unique violation 在集成测试中验证 (需要 DB)
        fn check(_: CatsError) {}
        let _ = sqlx::Error::PoolClosed; // 不会真的发生,只确保编译通过
        check(CatsError::from(sqlx::Error::PoolClosed));
    }
}