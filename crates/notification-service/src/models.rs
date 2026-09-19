//! 业务模型 + API DTO (notification-service)
//!
//! 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §6.7

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct NotificationRow {
    pub id: uuid::Uuid,
    pub org_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub channel: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub sent_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationResponse {
    pub id: uuid::Uuid,
    pub org_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub channel: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub sent_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<NotificationRow> for NotificationResponse {
    fn from(n: NotificationRow) -> Self {
        Self {
            id: n.id,
            org_id: n.org_id,
            user_id: n.user_id,
            channel: n.channel,
            event_type: n.event_type,
            payload: n.payload,
            status: n.status,
            created_at: n.created_at,
            sent_at: n.sent_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationListResponse {
    pub items: Vec<NotificationResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Kafka topic 内事件结构 (per §2 内部事件流)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaEvent {
    pub event_id: uuid::Uuid,
    pub org_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub event_type: String,
    pub payload: serde_json::Value,
}