//! 数据访问层 (report-service)
//!
//! 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §6.8
//!
//! MVP: report_db 自身轻量 (cache), 同时连 task_db (跨库只读 via svc_report_ro)

use crate::models::UsageReport;
use cats_common::CatsError;
use sqlx::PgPool;

#[derive(Clone)]
pub struct Pools {
    pub report: PgPool,
    /// task_db 只读 pool (per brief §1 report-service)
    pub task_ro: Option<PgPool>,
}

pub async fn build_pool() -> Result<PgPool, CatsError> {
    let url = std::env::var("DATABASE_URL")
        .map_err(|_| CatsError::internal("DATABASE_URL not set", "env missing".to_string()))?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(8)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&url)
        .await
        .map_err(|e| CatsError::internal("db connect failed", e.to_string()))?;
    Ok(pool)
}

pub async fn build_task_ro_pool() -> Result<Option<PgPool>, CatsError> {
    match std::env::var("TASK_DATABASE_URL") {
        Ok(url) => {
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(4)
                .acquire_timeout(std::time::Duration::from_secs(5))
                .connect(&url)
                .await
                .map_err(|e| CatsError::internal("task_db connect failed", e.to_string()))?;
            Ok(Some(pool))
        }
        Err(_) => Ok(None), // MVP 可选; 没配就降级
    }
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), CatsError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| CatsError::internal("migration failed", e.to_string()))?;
    Ok(())
}

/// 跨库只读: 从 task_db 聚合 usage (per §15.3 迁移策略)
pub async fn usage_report(
    task_pool: &PgPool,
    org_id: uuid::Uuid,
    period_start: chrono::DateTime<chrono::Utc>,
    period_end: chrono::DateTime<chrono::Utc>,
) -> Result<UsageReport, CatsError> {
    let rows = sqlx::query_as::<_, (String, i64)>(
        r#"
        SELECT status, COUNT(*)
        FROM tasks
        WHERE org_id = $1 AND created_at >= $2 AND created_at < $3
        GROUP BY status
        "#,
    )
    .bind(org_id)
    .bind(period_start)
    .bind(period_end)
    .fetch_all(task_pool)
    .await?;

    let mut total = 0i64;
    let mut completed = 0i64;
    let mut qa_blocked = 0i64;
    let mut in_progress = 0i64;
    let mut pending = 0i64;
    let mut failed = 0i64;
    for (status, count) in rows {
        total += count;
        match status.as_str() {
            "completed" => completed = count,
            "qa_blocked" => qa_blocked = count,
            "in_progress" => in_progress = count,
            "pending" => pending = count,
            "failed" => failed = count,
            _ => {}
        }
    }
    Ok(UsageReport {
        org_id,
        period_start,
        period_end,
        total_tasks: total,
        completed_tasks: completed,
        qa_blocked_tasks: qa_blocked,
        in_progress_tasks: in_progress,
        pending_tasks: pending,
        failed_tasks: failed,
    })
}