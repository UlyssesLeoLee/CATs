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
    /// 列类型是 `INET`，但读出来时用 `ip::text AS ip` 转成文本。
    ///
    /// 2026-10-05：这里原本写的是 `Option<sqlx::types::ipnetwork::IpNetwork>`，
    /// 而 workspace 的 sqlx **没有开 `ipnetwork` feature**，整个 crate 编译不过
    /// （E0433 `cannot find 'ipnetwork' in 'types'`，models.rs + db.rs 各一处）。
    /// 开着这个 feature 要给 Cargo.lock 加一个新包，而 CI 跑的是 `--locked`。
    ///
    /// 改用 TEXT 与仓库既有约定一致：
    ///   - `consumer.rs` 早就写着「不依赖 sqlx::types::ipnetwork (feature 未开),
    ///     让 PG 自己做类型转换」，写侧用 `$9::inet` + `&str`；
    ///   - `auth-service/migrations/0004_audit_log.sql` 的 `source_ip` 同样
    ///     显式改成 `TEXT`，注释写「改 TEXT 简化 bind」。
    pub ip: Option<String>,
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
            ip: r.ip,
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
