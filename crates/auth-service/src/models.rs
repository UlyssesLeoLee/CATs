//! 业务模型 + API DTO
//!
//! 引用: proto/cats/v1/auth.proto + api/openapi/cats-openapi-v1.yaml

use serde::{Deserialize, Serialize};

/// 登录请求 (REST, per OpenAPI v1)
#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 登录响应 (REST)
#[derive(Debug, Clone, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub token_type: String,
    pub user_id: String,
    pub username: String,
}

/// 刷新请求
#[derive(Debug, Clone, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// 刷新响应
#[derive(Debug, Clone, Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub token_type: String,
}

/// 错误响应 (统一格式 per 实施前QA §3.4)
#[derive(Debug, Clone, Serialize)]
pub struct ErrorBody {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
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
    pub password_hash: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
