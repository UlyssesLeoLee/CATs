//! report-service 数据模型 (per ULYS-153 切片 C-1)
//!
//! 引用: api/openapi/cats-openapi-v1.0.1.yaml §/v1/reports

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// =====================================================================
// 1. /v1/reports/usage 响应
// =====================================================================

/// usage 单行 (按 action × resource_type 分组)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageReportItem {
    pub action: String,
    pub resource_type: String,
    pub event_count: i64,
    pub distinct_actors: i64,
}

/// usage 列表响应 (per org 在 [from, to] 区间的统计)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageReportResponse {
    pub org_id: Uuid,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub total_events: i64,
    pub items: Vec<UsageReportItem>,
}

/// SQL 行 (与 UsageReportItem 字段顺序一致, FromRow derive)
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ActionCountRow {
    pub action: String,
    pub resource_type: String,
    pub event_count: i64,
    pub distinct_actors: Option<i64>,
}

// =====================================================================
// 2. /v1/reports/translation-volume 响应
// =====================================================================

/// 翻译量单行 (按 day 聚合)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationVolumeRow {
    pub day: chrono::NaiveDate,
    pub completed_tasks: i64,
    pub total_chars: Option<i64>,
}

/// 翻译量响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationVolumeResponse {
    pub project_id: Uuid,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub total_completed: i64,
    pub total_chars: i64,
    pub daily: Vec<TranslationVolumeRow>,
}

// =====================================================================
// 3. /v1/reports/audit-summary 响应
// =====================================================================

/// 审计摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSummary {
    pub action: String,
    pub event_count: i64,
}

/// audit-summary 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSummaryResponse {
    pub workspace_id: Uuid,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub total_events: i64,
    pub distinct_actors: i64,
    pub top_actions: Vec<AuditSummary>,
    pub last_event_at: Option<DateTime<Utc>>,
}

// =====================================================================
// 4. 错误信封 (统一格式, 与接口设计书 §1.3 对齐)
// =====================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl ErrorBody {
    pub fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            detail: None,
        }
    }
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}
