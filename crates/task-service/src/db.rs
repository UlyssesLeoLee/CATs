//! task_db 访问层 (sqlx)
//!
//! 引用: doc/05-其他/MVP商业版/_slice_b2_task.md
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §5.1 (task_db 逻辑库边界)
//!
//! 设计选择 (per 缺标比错标安全):
//! - 与 project_db / user_db / auth_db 同 pattern, 各自独立 schema; 跨服务不直连
//! - project_id 不做 FK 到 project.project (per 架构书 §1.2 原则 4); 调用方负责合法性
//! - input_payload / output_payload JSONB, 用 `sqlx::types::Json<T>` 直接读写
//! - tasks.id 默认 gen_random_uuid() (依赖 pgcrypto, 由本 service 自管)
//!   → 数据库层面要求 `CREATE EXTENSION IF NOT EXISTS pgcrypto`, 留 Sprint 2 启动器注入

use crate::models::{Task, TaskStatus, TaskType};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::types::Json;
use std::env;
use std::time::Duration;
use uuid::Uuid;

/// 构造 task_db 连接池 (lazy, 不实际连 DB, 与 user-service/auth-service 同模式)
pub async fn build_pool() -> Result<PgPool> {
    let url =
        env::var("DATABASE_URL").context("DATABASE_URL env var not set (per §5.2 注入规范)")?;
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(3))
        .connect_lazy(&url)
        .context("PgPool lazy build failed")?;
    Ok(pool)
}

/// 列出 tasks 的查询选项
#[derive(Debug, Clone, Default)]
pub struct ListFilter {
    /// 可选: 按 project_id 过滤
    pub project_id: Option<Uuid>,
    /// 可选: 按 status 过滤
    pub status: Option<TaskStatus>,
    /// 分页 limit (默认 50, 最大 200)
    pub limit: i64,
    /// 分页 offset
    pub offset: i64,
}

impl ListFilter {
    /// 校验 limit 范围, 防止恶意查询
    pub fn normalized(mut self) -> Self {
        if self.limit <= 0 {
            self.limit = 50;
        } else if self.limit > 200 {
            self.limit = 200;
        }
        if self.offset < 0 {
            self.offset = 0;
        }
        self
    }
}

/// 列表返回结果 (含分页元信息)
#[derive(Debug, Clone)]
pub struct ListResult {
    pub items: Vec<Task>,
    pub total: i64,
}

/// 创建 task (per POST /v1/tasks)
///
/// `input_payload` 与 `output_payload` 用 `Json<T>` 包裹, sqlx 自动序列化/反序列化
///
/// 返回新建的 task, 由 DB 默认填充 id / created_at / updated_at / status=pending
pub async fn create(
    pool: &PgPool,
    project_id: Uuid,
    task_type: TaskType,
    input_payload: Json<serde_json::Value>,
) -> Result<Task> {
    let row: Task = sqlx::query_as::<_, Task>(
        r#"
        INSERT INTO tasks (project_id, task_type, input_payload)
        VALUES ($1, $2, $3)
        RETURNING id, project_id, task_type, status, input_payload, output_payload,
                  progress, error_message, created_at, updated_at, started_at, completed_at
        "#,
    )
    .bind(project_id)
    .bind(task_type.as_str())
    .bind(input_payload)
    .fetch_one(pool)
    .await
    .context("create task insert failed")?;
    Ok(row)
}

/// 按 id 查 task (per GET /v1/tasks/{id})
pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Task>> {
    let row: Option<Task> = sqlx::query_as::<_, Task>(
        r#"
        SELECT id, project_id, task_type, status, input_payload, output_payload,
               progress, error_message, created_at, updated_at, started_at, completed_at
        FROM tasks
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("find_by_id query failed")?;
    Ok(row)
}

/// 按 filter 列 task (per GET /v1/tasks)
///
/// 返回 (items, total) — total = 满足条件的总行数 (无 limit/offset 截断)
pub async fn list(pool: &PgPool, filter: &ListFilter) -> Result<ListResult> {
    let filter = filter.clone().normalized();

    // 1) 查总数 (per 分页元信息)
    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint FROM tasks
        WHERE ($1::uuid IS NULL OR project_id = $1)
          AND ($2::text IS NULL OR status = $2)
        "#,
    )
    .bind(filter.project_id)
    .bind(filter.status.map(|s| s.as_str()))
    .fetch_one(pool)
    .await
    .context("list count query failed")?;

    // 2) 查分页结果 (按 created_at DESC, 与 idx_tasks_created_at 一致)
    let items: Vec<Task> = sqlx::query_as::<_, Task>(
        r#"
        SELECT id, project_id, task_type, status, input_payload, output_payload,
               progress, error_message, created_at, updated_at, started_at, completed_at
        FROM tasks
        WHERE ($1::uuid IS NULL OR project_id = $1)
          AND ($2::text IS NULL OR status = $2)
        ORDER BY created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(filter.project_id)
    .bind(filter.status.map(|s| s.as_str()))
    .bind(filter.limit)
    .bind(filter.offset)
    .fetch_all(pool)
    .await
    .context("list query failed")?;

    Ok(ListResult { items, total })
}

/// 更新 task 状态 (per PATCH /v1/tasks/{id}/status)
///
/// 返回更新后的 task; `None` = task 不存在 (404)
///
/// 状态机 (per 接口设计书 §3.4):
/// - pending → running → completed | failed | cancelled
/// - running → running (允许 progress 更新, 但状态不变)
/// - 终态 (completed/failed/cancelled) → 不可再更新 (返回 None 触发 409)
///
/// 自动维护 started_at / completed_at 时间戳 (per 状态机语义)
pub async fn update_status(
    pool: &PgPool,
    id: Uuid,
    new_status: TaskStatus,
    progress: Option<i32>,
    error_message: Option<&str>,
    output_payload: Option<Json<serde_json::Value>>,
) -> Result<Option<Task>> {
    // 校验 progress 范围 (per DB CHECK 约束)
    let progress = match progress {
        Some(p) if !(0..=100).contains(&p) => {
            anyhow::bail!("progress must be in 0..=100, got {p}")
        }
        other => other,
    };

    // 按新状态决定时间戳字段
    let now: DateTime<Utc> = chrono::Utc::now();
    let started_at = match new_status {
        TaskStatus::Running => Some(now),
        _ => None, // 仅 running 状态显式设置 started_at (避免重复覆盖)
    };
    let completed_at = if new_status.is_terminal() {
        Some(now)
    } else {
        None
    };

    let row: Option<Task> = sqlx::query_as::<_, Task>(
        r#"
        UPDATE tasks
        SET status          = $2,
            progress        = COALESCE($3, progress),
            error_message   = COALESCE($4, error_message),
            output_payload  = COALESCE($5, output_payload),
            started_at      = COALESCE($6, started_at),
            completed_at    = COALESCE($7, completed_at)
        WHERE id = $1
          AND status NOT IN ('completed', 'failed', 'cancelled')
        RETURNING id, project_id, task_type, status, input_payload, output_payload,
                  progress, error_message, created_at, updated_at, started_at, completed_at
        "#,
    )
    .bind(id)
    .bind(new_status.as_str())
    .bind(progress)
    .bind(error_message)
    .bind(output_payload)
    .bind(started_at)
    .bind(completed_at)
    .fetch_optional(pool)
    .await
    .context("update_status failed")?;
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_filter_normalized_clamps_limit() {
        let f = ListFilter {
            project_id: None,
            status: None,
            limit: 0,
            offset: 0,
        }
        .normalized();
        assert_eq!(f.limit, 50);

        let f = ListFilter {
            project_id: None,
            status: None,
            limit: 9999,
            offset: 0,
        }
        .normalized();
        assert_eq!(f.limit, 200);

        let f = ListFilter {
            project_id: None,
            status: None,
            limit: 50,
            offset: -5,
        }
        .normalized();
        assert_eq!(f.offset, 0);
    }
}