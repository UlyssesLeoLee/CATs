//! 业务模型 + API DTO (report-service)
//!
//! 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §6.8

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageReport {
    pub org_id: uuid::Uuid,
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,
    pub total_tasks: i64,
    pub completed_tasks: i64,
    pub qa_blocked_tasks: i64,
    pub in_progress_tasks: i64,
    pub pending_tasks: i64,
    pub failed_tasks: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityReport {
    pub org_id: uuid::Uuid,
    pub total_violations: i64,
    pub top_rule_ids: Vec<String>,
    pub qa_blocked_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportListResponse {
    pub items: Vec<UsageReport>,
}