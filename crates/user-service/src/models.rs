//! 业务模型 + API DTO
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1 (user-service)
//! 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.1 (user_db 接口契约 v1.0.0)
//! 引用: doc/05-其他/管理/CATs_错误码表_v1.0.md §3-§4 (error enum 复用)
//!
//! 设计选择 (per 缺标比错标安全):
//! - 接口设计书 v2.0 整份不存在 (per Sprint 1 §6.1 缺口)
//!   → DTO 设计基于微服务架构书 §4.1 + Baseline §5.1 端点清单
//!   → 详细 request/response schema 留 T-07 启动时升接口设计书 v2.0
//! - ErrorBody 直接复用 auth-service 错误码表 §3 枚举值
//!   → 不重复定义, 错误码一致性通过引用错误码表 v1.0 保证

use serde::{Deserialize, Serialize};

/// DB 实体: user_db.user_profile
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserProfile {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub display_name: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub locale: String,
    pub timezone: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// =====================================================================
// API DTO
// =====================================================================

/// GET /v1/users/{id} 响应 (per Baseline §5.1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetUserResponse {
    pub id: String,
    pub user_id: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub locale: String,
    pub timezone: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<UserProfile> for GetUserResponse {
    fn from(p: UserProfile) -> Self {
        Self {
            id: p.id.to_string(),
            user_id: p.user_id.to_string(),
            display_name: p.display_name,
            email: p.email,
            avatar_url: p.avatar_url,
            locale: p.locale,
            timezone: p.timezone,
            is_active: p.is_active,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

/// PUT /v1/users/{id} 请求 (per Baseline §5.1 端点清单, 详细 schema 留接口设计书 v2.0)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

/// POST /v1/users (创建, per Baseline §5.1 端点清单)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub user_id: uuid::Uuid,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

pub fn default_locale() -> String {
    "ja-JP".to_string()
}

pub fn default_timezone() -> String {
    "Asia/Tokyo".to_string()
}

/// 错误响应 (与 auth-service 错误码表 §3 枚举值保持一致, per 错误码表 v1.0 §6.2 跨服务一致性)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// 通用错误响应包装 (HTTP status + body)
pub struct ApiError {
    pub status: u16,
    pub body: ErrorBody,
}
