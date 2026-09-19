//! audit-service 数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 数据库一行 audit_logs
#[derive(Debug, Clone, FromRow)]
pub struct AuditLogRow {
    pub id: i64,
    pub event_id: Uuid,
    pub org_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub before_state: Option<serde_json::Value>,
    pub after_state: Option<serde_json::Value>,
    pub ip: Option<sqlx::types::ipnetwork::IpNetwork>,
    pub occurred_at: DateTime<Utc>,
    pub ingested_at: DateTime<Utc>,
}

/// API 响应
#[derive(Debug, Clone, Serialize)]
pub struct AuditLogResponse {
    pub id: i64,
    pub event_id: Uuid,
    pub org_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub before_state: Option<serde_json::Value>,
    pub after_state: Option<serde_json::Value>,
    pub ip: Option<String>,
    pub occurred_at: DateTime<Utc>,
    pub ingested_at: DateTime<Utc>,
}

impl From<AuditLogRow> for AuditLogResponse {
    fn from(r: AuditLogRow) -> Self {
        Self {
            id: r.id,
            event_id: r.event_id,
            org_id: r.org_id,
            actor_user_id: r.actor_user_id,
            action: r.action,
            resource_type: r.resource_type,
            resource_id: r.resource_id,
            before_state: r.before_state,
            after_state: r.after_state,
            ip: r.ip.map(|ip| ip.ip().to_string()),
            occurred_at: r.occurred_at,
            ingested_at: r.ingested_at,
        }
    }
}

/// 列表响应
#[derive(Debug, Clone, Serialize)]
pub struct AuditLogListResponse {
    pub items: Vec<AuditLogResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Kafka 事件 schema (per 接口设计 §1.5 schema_version 字段 + 文档约定)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KafkaAuditEvent {
    pub event_id: Uuid,
    pub org_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    #[serde(default)]
    pub before_state: Option<serde_json::Value>,
    #[serde(default)]
    pub after_state: Option<serde_json::Value>,
    #[serde(default)]
    pub ip: Option<String>,
    pub occurred_at: DateTime<Utc>,
    #[serde(default)]
    pub schema_version: u32,
}