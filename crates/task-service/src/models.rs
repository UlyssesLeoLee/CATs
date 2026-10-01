//! 业务模型 + API DTO
//!
//! 引用: doc/05-其他/MVP商业版/_slice_b2_task.md (slice B-2 切片)
//! 引用: doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md §3.4 (task-service)
//! 引用: doc/05-其他/管理/CATs_错误码表_v1.0.1.md §3-§4 (error enum 复用)
//! 引用: ULYS-45 子任务 A (af8abb6) SSE 既有实现参考
//!
//! 设计选择 (per 缺标比错标安全):
//! - DB 实体与 API DTO 解耦: 内部 Row 用 `sqlx::FromRow` 派生, API 响应通过 `From` impl 投影
//!   → 字段重命名 / 隐藏 / 类型转换不破坏 DB 兼容
//! - ErrorBody 与 auth/user-service 错误码表 §3 枚举值保持一致
//!   → 跨服务错误码一致性 per 错误码表 v1.0 §6.2
//! - TaskStatus 4 态对齐本切片 DB schema (pending/running/completed/failed/cancelled);
//!   复用 ULYS-45 (af8abb6) 中 8 态 TaskStatus 仅用于 SSE `TaskEvent::TaskTerminated`
//!   (per 切片 B-2: SSE 推送任务进度的端到端场景)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use uuid::Uuid;

// =====================================================================
// 1. 业务枚举
// =====================================================================

/// 任务类型 (per DB schema CHECK 约束)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    Translate,
    Review,
    Export,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Translate => "translate",
            Self::Review => "review",
            Self::Export => "export",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "translate" => Some(Self::Translate),
            "review" => Some(Self::Review),
            "export" => Some(Self::Export),
            _ => None,
        }
    }
}

/// 任务状态 (per DB schema CHECK 约束, 切片 B-2 范围: 5 态)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    /// 是否为终态 (per 接口设计书 §3.4 状态机: 终态后 SSE 流正常关闭)
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

impl Default for TaskStatus {
    fn default() -> Self {
        Self::Pending
    }
}

// =====================================================================
// 2. DB 实体
// =====================================================================

/// DB row: tasks 表 (per migrations/20260920_0001_init.sql)
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Task {
    pub id: Uuid,
    pub project_id: Uuid,
    pub task_type: String,
    pub status: String,
    pub input_payload: Json<serde_json::Value>,
    pub output_payload: Json<serde_json::Value>,
    pub progress: i32,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl Task {
    /// 解析 status 字段为 enum (DB 存 TEXT, 业务层用 enum)
    pub fn status_enum(&self) -> TaskStatus {
        TaskStatus::parse(&self.status).unwrap_or(TaskStatus::Pending)
    }

    /// 解析 task_type 字段为 enum
    pub fn task_type_enum(&self) -> TaskType {
        TaskType::parse(&self.task_type).unwrap_or(TaskType::Translate)
    }
}

// =====================================================================
// 3. API DTO
// =====================================================================

/// `POST /v1/tasks` 请求 (per 接口设计书 §3.4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub project_id: Uuid,
    pub task_type: TaskType,
    #[serde(default)]
    pub input_payload: serde_json::Value,
}

/// `POST /v1/tasks` 响应 — 直接返回完整 task 视图
pub type CreateTaskResponse = TaskView;

/// `GET /v1/tasks/{id}` 响应
pub type GetTaskResponse = TaskView;

/// `GET /v1/tasks` 响应 — 分页列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTasksResponse {
    pub items: Vec<TaskView>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// `GET /v1/tasks` query 参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTasksQuery {
    #[serde(default)]
    pub project_id: Option<Uuid>,
    #[serde(default)]
    pub status: Option<String>, // 接受字符串, 解析为 TaskStatus
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

/// `PATCH /v1/tasks/{id}/status` 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: TaskStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_payload: Option<serde_json::Value>,
}

/// `PATCH /v1/tasks/{id}/status` 响应
pub type UpdateStatusResponse = TaskView;

/// `GET /v1/tasks/{id}/events` (SSE) — 见 `events.rs` / SSE 帧编码
///
/// 单个媒体处理阶段 (per 接口设计书 §3.4 内部上报 schema; 复用 ULYS-45)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageKind {
    Asr,
    Ocr,
    Translation,
    Subtitle,
    Rendering,
    Ingestion,
}

/// 阶段上报状态 (per 接口设计书 §3.4)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    Started,
    InProgress,
    Completed,
    Failed,
}

/// 阶段产出引用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResultRef {
    pub file_id: String,
    pub kind: String,
}

/// 阶段度量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageMetrics {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_seconds: Option<u64>,
}

/// 内部上报: `/internal/v1/tasks/{id}/stage-progress` 请求体
/// (per 接口设计书 §3.4 + ULYS-45 af8abb6 既有实现参考)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageProgressRequest {
    /// 事件幂等键
    pub event_id: String,
    pub stage: StageKind,
    pub status: StageStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_ref: Option<StageResultRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics: Option<StageMetrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// SSE 事件 (per 接口设计书 §3.4 `respond` 节点出口状态)
///
/// 复用 ULYS-45 (af8abb6) 8 态 TaskStatus 在这里 (终态事件用 8 态子集兼容 SSE 协议),
/// 与切片 B-2 DB schema 5 态不冲突 — 切片 B-2 仅在 DB PATCH /v1/tasks/{id}/status
/// 处使用 5 态, SSE 内部表示沿用 ULYS-45 既有 8 态 TaskEvent 协议.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum TaskEvent {
    /// 阶段进度更新
    StageProgress {
        event_id: String,
        stage: StageKind,
        status: StageStatus,
        #[serde(skip_serializing_if = "Option::is_none")]
        progress: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        result_ref: Option<StageResultRef>,
        #[serde(skip_serializing_if = "Option::is_none")]
        metrics: Option<StageMetrics>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
        occurred_at: DateTime<Utc>,
    },
    /// 任务终态 (per 接口设计书 §3.4 状态机: completed/failed/cancelled/partially_failed)
    TaskTerminated {
        status: SseTaskStatus,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        occurred_at: DateTime<Utc>,
    },
    /// 心跳 (per SSE 协议最佳实践 + 接口设计书 §3.4 连接保持)
    Heartbeat { occurred_at: DateTime<Utc> },
}

/// SSE 协议用的 TaskStatus (per ULYS-45 af8abb6, 8 态; 切片 B-2 5 态的父集)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SseTaskStatus {
    Queued,
    Ingesting,
    Processing,
    Rendering,
    Completed,
    Failed,
    Cancelled,
    PartiallyFailed,
}

impl SseTaskStatus {
    pub fn from_db(s: TaskStatus) -> Self {
        match s {
            TaskStatus::Pending => Self::Queued,
            TaskStatus::Running => Self::Processing,
            TaskStatus::Completed => Self::Completed,
            TaskStatus::Failed => Self::Failed,
            TaskStatus::Cancelled => Self::Cancelled,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::PartiallyFailed
        )
    }
}

// =====================================================================
// 4. 公共视图 (per REST API 响应)
// =====================================================================

/// TaskView: 跨所有 REST 响应的对外视图 (与 DB row 解耦, 字段稳定)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskView {
    pub id: Uuid,
    pub project_id: Uuid,
    pub task_type: String,
    pub status: String,
    pub input_payload: serde_json::Value,
    pub output_payload: serde_json::Value,
    pub progress: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<Task> for TaskView {
    fn from(t: Task) -> Self {
        Self {
            id: t.id,
            project_id: t.project_id,
            task_type: t.task_type,
            status: t.status,
            input_payload: t.input_payload.0,
            output_payload: t.output_payload.0,
            progress: t.progress,
            error_message: t.error_message,
            created_at: t.created_at,
            updated_at: t.updated_at,
            started_at: t.started_at,
            completed_at: t.completed_at,
        }
    }
}

// =====================================================================
// 5. 错误响应 (per 错误码表 v1.0.1 §3 枚举值)
// =====================================================================

/// 错误响应 (与 auth/user-service 错误码表 §3 枚举值保持一致, per 错误码表 v1.0.1 §6.2)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_status_round_trip() {
        for s in [
            TaskStatus::Pending,
            TaskStatus::Running,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Cancelled,
        ] {
            let parsed = TaskStatus::parse(s.as_str()).unwrap();
            assert_eq!(parsed, s);
        }
        assert!(TaskStatus::parse("unknown").is_none());
    }

    #[test]
    fn task_type_round_trip() {
        for t in [TaskType::Translate, TaskType::Review, TaskType::Export] {
            let parsed = TaskType::parse(t.as_str()).unwrap();
            assert_eq!(parsed, t);
        }
        assert!(TaskType::parse("unknown").is_none());
    }

    #[test]
    fn task_status_is_terminal() {
        assert!(TaskStatus::Completed.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
        assert!(!TaskStatus::Pending.is_terminal());
        assert!(!TaskStatus::Running.is_terminal());
    }

    #[test]
    fn sse_task_status_is_terminal_matches() {
        for s in [
            SseTaskStatus::Queued,
            SseTaskStatus::Ingesting,
            SseTaskStatus::Processing,
            SseTaskStatus::Rendering,
        ] {
            assert!(!s.is_terminal(), "{s:?} should not be terminal");
        }
        for s in [
            SseTaskStatus::Completed,
            SseTaskStatus::Failed,
            SseTaskStatus::Cancelled,
            SseTaskStatus::PartiallyFailed,
        ] {
            assert!(s.is_terminal(), "{s:?} should be terminal");
        }
    }

    #[test]
    fn sse_task_status_from_db_maps() {
        assert_eq!(
            SseTaskStatus::from_db(TaskStatus::Pending),
            SseTaskStatus::Queued
        );
        assert_eq!(
            SseTaskStatus::from_db(TaskStatus::Running),
            SseTaskStatus::Processing
        );
        assert_eq!(
            SseTaskStatus::from_db(TaskStatus::Completed),
            SseTaskStatus::Completed
        );
    }
}