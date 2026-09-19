//! 业务模型 + API DTO (task-service)
//!
//! 引用: doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md §6.4
//! 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §6.6
//! 引用: doc/05-其他/管理/CATs_错误码表_v1.0.md §6 (QA_BLOCKED)
//!
//! 状态机 (per §6.4.5): pending → in_progress → completed (or → qa_blocked)
//! - qa_blocked: QA 阻断, 禁止提交/导出 (per 错误码表 §6 + §1.4 FAILED_PRECONDITION)
//! - failed: 致命错误 (人工介入)
//! 转换图: pending → in_progress | failed
//!         in_progress → completed | qa_blocked | failed
//!         qa_blocked → in_progress (返工) | failed

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    QaBlocked,
    Failed,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::QaBlocked => "qa_blocked",
            Self::Failed => "failed",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "in_progress" => Some(Self::InProgress),
            "completed" => Some(Self::Completed),
            "qa_blocked" => Some(Self::QaBlocked),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
    /// 状态机迁移是否合法 (per §6.4.5)
    pub fn can_transition_to(&self, target: &TaskStatus) -> bool {
        use TaskStatus::*;
        matches!(
            (self, target),
            (Pending, InProgress)
                | (Pending, Failed)
                | (InProgress, Completed)
                | (InProgress, QaBlocked)
                | (InProgress, Failed)
                | (QaBlocked, InProgress)
                | (QaBlocked, Failed)
        )
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct TaskRow {
    pub id: uuid::Uuid,
    pub project_id: uuid::Uuid,
    pub org_id: uuid::Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub media_type: String,
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
    pub assigned_to: Option<uuid::Uuid>,
    pub qa_violations: Option<serde_json::Value>,
    pub created_by: uuid::Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub project_id: uuid::Uuid,
    pub org_id: uuid::Uuid,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_media_type")]
    pub media_type: String,
    #[serde(default)]
    pub source_lang: Option<String>,
    #[serde(default)]
    pub target_lang: Option<String>,
    #[serde(default)]
    pub assigned_to: Option<uuid::Uuid>,
}

fn default_media_type() -> String {
    "document".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTaskStatusRequest {
    pub status: String,
    #[serde(default)]
    pub qa_violations: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponse {
    pub id: uuid::Uuid,
    pub project_id: uuid::Uuid,
    pub org_id: uuid::Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub media_type: String,
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
    pub assigned_to: Option<uuid::Uuid>,
    pub qa_violations: serde_json::Value,
    pub created_by: uuid::Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<TaskRow> for TaskResponse {
    fn from(t: TaskRow) -> Self {
        Self {
            id: t.id,
            project_id: t.project_id,
            org_id: t.org_id,
            title: t.title,
            description: t.description,
            status: t.status,
            media_type: t.media_type,
            source_lang: t.source_lang,
            target_lang: t.target_lang,
            assigned_to: t.assigned_to,
            qa_violations: t.qa_violations.unwrap_or(serde_json::Value::Array(vec![])),
            created_by: t.created_by,
            created_at: t.created_at,
            updated_at: t.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskListResponse {
    pub items: Vec<TaskResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}