//! worker-service 周期调度器
//!
//! 每 30s 扫一次 `tasks WHERE status='pending'`,对每条:
//! 1. UPDATE status='in_progress'
//! 2. 委托 translation-core (MVP 简化: 写日志 + 立即标 status='completed')
//! 3. 失败 → status='qa_blocked' + 写 audit log
//!
//! 真连 translation-core 留 Sprint 3(需要 translation-core gRPC server 落地)。

use cats_common::{CatsError, ErrorCode};
use sqlx::PgPool;
use tracing::{error, info, warn};
use uuid::Uuid;

/// 一次扫描最大处理任务数 (防止雪崩)
const BATCH_SIZE: i64 = 50;

pub async fn run_scheduler_loop(pool: PgPool) {
    info!("worker-service scheduler started");
    loop {
        if let Err(e) = tick(&pool).await {
            error!(error = ?e, "scheduler tick failed");
        }
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    }
}

async fn tick(pool: &PgPool) -> Result<(), CatsError> {
    // 1. 抢占: 把一批 pending → in_progress (避免多 worker 抢同一行)
    let claimed: Vec<(Uuid, Uuid)> = sqlx::query_as(
        r#"UPDATE tasks
           SET status = 'in_progress', updated_at = now()
           WHERE id IN (
               SELECT id FROM tasks
               WHERE status = 'pending'
               ORDER BY created_at ASC
               LIMIT $1
               FOR UPDATE SKIP LOCKED
           )
           RETURNING id, project_id"#,
    )
    .bind(BATCH_SIZE)
    .fetch_all(pool)
    .await
    .map_err(|e| CatsError::business(ErrorCode::InternalError, format!("claim fail: {e}")))?;

    if claimed.is_empty() {
        return Ok(());
    }

    info!(count = claimed.len(), "claimed pending tasks");

    for (task_id, _project_id) in claimed {
        match dispatch_one(pool, task_id).await {
            Ok(()) => {}
            Err(e) => {
                warn!(task_id = %task_id, error = ?e, "dispatch failed");
                if let Err(e2) = mark_qa_blocked(pool, task_id).await {
                    error!(task_id = %task_id, error = ?e2, "mark qa_blocked failed");
                }
            }
        }
    }
    Ok(())
}

async fn dispatch_one(pool: &PgPool, task_id: Uuid) -> Result<(), CatsError> {
    // MVP 简化: 实际翻译逻辑留 translation-core,这里写日志 + 直接 completed
    info!(task_id = %task_id, "dispatch_one (MVP stub, real translation via translation-core deferred to Sprint 3)");
    sqlx::query(r#"UPDATE tasks SET status = 'completed', updated_at = now() WHERE id = $1"#)
        .bind(task_id)
        .execute(pool)
        .await
        .map_err(|e| CatsError::business(ErrorCode::InternalError, format!("complete fail: {e}")))?;
    Ok(())
}

async fn mark_qa_blocked(pool: &PgPool, task_id: Uuid) -> Result<(), CatsError> {
    sqlx::query(r#"UPDATE tasks SET status = 'qa_blocked', updated_at = now() WHERE id = $1"#)
        .bind(task_id)
        .execute(pool)
        .await
        .map_err(|e| CatsError::business(ErrorCode::InternalError, format!("qa_blocked fail: {e}")))
}