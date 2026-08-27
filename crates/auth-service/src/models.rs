//! 业务模型 + API DTO
//!
//! 引用: proto/cats/v1/auth.proto + api/openapi/cats-openapi-v1.yaml

use serde::{Deserialize, Serialize};

/// 登录请求 (REST, per OpenAPI v1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 登录响应 (REST)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub token_type: String,
    pub user_id: String,
    pub username: String,
}

/// 刷新请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// 刷新响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub token_type: String,
}

/// 错误响应 (统一格式 per 实施前QA §3.4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// GET /v1/auth/me 响应 (per OpenAPI v1 §3.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeResponse {
    pub user_id: String,
    pub username: String,
    pub email: String,
}

/// JWT Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id (UUID)
    pub username: String,
    pub exp: i64,           // unix timestamp
    pub iat: i64,           // unix timestamp
    pub jti: String,        // JWT ID (UUID v4)
    pub token_type: String, // "access" or "refresh"
}

/// DB 实体: auth_db.users_credential
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserCredential {
    pub id: uuid::Uuid,
    pub username: String,
    pub email: Option<String>,
    pub password_hash: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// =====================================================================
// T-01: 审计事件 + 登出 + 刷新撤销 增量
// 引用: doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md §2 T-01
// =====================================================================

/// 审计事件结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    Success,
    Failure,
}

impl AuditOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
        }
    }
}

/// 审计事件 (per T-01 + 安全要件 §6)
///
/// 事件类型枚举（参考错误码表 v1.0 §3.1, 后续在表里详列）:
/// - `login` / `login_failed` — 登录成功 / 失败
/// - `logout` — 登出
/// - `refresh` / `refresh_failed` / `refresh_revoked` — 刷新成功 / 失败 / 撤销
/// - `me_access` — 当前用户查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_id: uuid::Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<uuid::Uuid>,
    pub event_type: String,
    pub outcome: AuditOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    pub occurred_at: chrono::DateTime<chrono::Utc>,
}

/// POST /v1/auth/logout 请求 (per 接口设计书 v2.0 §3.4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutRequest {
    /// 撤销的 refresh_token (jti 从中解析)
    pub refresh_token: String,
}

/// POST /v1/auth/logout 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutResponse {
    pub revoked: bool,
    pub revoked_at: chrono::DateTime<chrono::Utc>,
}

/// audit_log 表 raw row (查询返回)
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuditEventRow {
    pub event_id: uuid::Uuid,
    pub user_id: Option<uuid::Uuid>,
    pub event_type: String,
    pub outcome: String,
    pub detail: Option<serde_json::Value>,
    pub source_ip: Option<String>,
    pub user_agent: Option<String>,
    pub occurred_at: chrono::DateTime<chrono::Utc>,
}
