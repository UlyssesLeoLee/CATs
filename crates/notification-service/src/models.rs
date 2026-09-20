//! 业务模型 + API DTO
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1 (notification-service)
//! 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.1 (notification_db 接口契约 v1.0.0)
//! 引用: doc/05-其他/管理/CATs_错误码表_v1.0.md §3 (error enum)
//!
//! 设计选择 (per 缺标比错标安全, 守门 #11):
//! - 接口设计书 v2.0 整份不存在 (per Sprint 1 §6.1 缺口)
//!   → DTO 基于微服务架构书 §4.1 + 本切片 B-4 任务描述
//! - ErrorBody 与 user-service / file-service 错误码表 §3 枚举值一致

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// DB 实体: notification_db.notifications
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NotificationRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    #[sqlx(rename = "type")]
    pub notif_type: String,
    pub title: String,
    pub body: String,
    pub payload: serde_json::Value,
    pub read_at: Option<chrono::DateTime<chrono::Utc>>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// =====================================================================
// API DTO
// =====================================================================

/// GET /v1/notifications/{id} 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationResponse {
    pub id: String,
    pub user_id: String,
    #[serde(rename = "type")]
    pub notif_type: String,
    pub title: String,
    pub body: String,
    pub payload: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_at: Option<chrono::DateTime<chrono::Utc>>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<NotificationRecord> for NotificationResponse {
    fn from(r: NotificationRecord) -> Self {
        Self {
            id: r.id.to_string(),
            user_id: r.user_id.to_string(),
            notif_type: r.notif_type,
            title: r.title,
            body: r.body,
            payload: r.payload,
            read_at: r.read_at,
            status: r.status,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// GET /v1/notifications 列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationListResponse {
    pub items: Vec<NotificationResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// POST /v1/notifications (内部创建, 测试用) 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNotificationRequest {
    pub user_id: Uuid,
    #[serde(rename = "type")]
    pub notif_type: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

/// PATCH /v1/notifications/{id}/read 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkReadResponse {
    pub id: String,
    pub marked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// GET /v1/notifications 列表查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListNotificationsQuery {
    pub user_id: Uuid,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

/// 错误响应 (与 user-service / file-service 错误码表 §3 枚举值保持一致)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// SSE 事件 (per /v1/notifications/ws 端点推送给客户端的 payload)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEvent {
    pub event: String, // "notification.created" / "notification.read" / "ping"
    pub data: serde_json::Value,
}
