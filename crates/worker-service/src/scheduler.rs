//! worker-service 周期调度器
//!
//! 每 30s 扫一次 `tasks WHERE status='pending'`,对每条:
//! 1. UPDATE status='in_progress'
//! 2. 委托 translation-core (MVP 简化: 写日志 + 立即标 status='completed')
//! 3. 失败 → status='qa_blocked' + 写 audit log
//!
//! 真连 translation-core 留 Sprint 3(需要 translation-core gRPC server 落地)。

use cats_common::error::{CatsError, ErrorCode};
use sqlx::PgPool;
use tracing::{error, info, warn};
use uuid::Uuid;

/// 一次扫描最大处理任务数 (防止雪崩)
const BATCH_SIZE: i64 = 50;

/// 抢占 SQL: pending → in_progress, 配合 SKIP LOCKED 防多 worker 重复抢占
const CLAIM_SQL: &str = r#"UPDATE tasks
           SET status = 'in_progress', updated_at = now()
           WHERE id IN (
               SELECT id FROM tasks
               WHERE status = 'pending'
               ORDER BY created_at ASC
               LIMIT $1
               FOR UPDATE SKIP LOCKED
           )
           RETURNING id, project_id"#;

/// 派发成功回写: → completed
const COMPLETE_SQL: &str =
    r#"UPDATE tasks SET status = 'completed', updated_at = now() WHERE id = $1"#;

/// 派发失败回写: → qa_blocked
const QA_BLOCKED_SQL: &str =
    r#"UPDATE tasks SET status = 'qa_blocked', updated_at = now() WHERE id = $1"#;

pub async fn run_scheduler_loop(pool: PgPool) {
    info!("worker-service scheduler started");
    loop {
        if let Err(e) = tick(&pool).await {
            error!(error = ?e, "scheduler tick failed");
        }
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    }
}

pub async fn tick(pool: &PgPool) -> Result<(), CatsError> {
    // 1. 抢占: 把一批 pending → in_progress (避免多 worker 抢同一行)
    let claimed: Vec<(Uuid, Uuid)> = sqlx::query_as(CLAIM_SQL)
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
    sqlx::query(COMPLETE_SQL)
        .bind(task_id)
        .execute(pool)
        .await
        .map_err(|e| {
            CatsError::business(ErrorCode::InternalError, format!("complete fail: {e}"))
        })?;
    Ok(())
}

async fn mark_qa_blocked(pool: &PgPool, task_id: Uuid) -> Result<(), CatsError> {
    sqlx::query(QA_BLOCKED_SQL)
        .bind(task_id)
        .execute(pool)
        .await
        .map_err(|e| {
            CatsError::business(ErrorCode::InternalError, format!("qa_blocked fail: {e}"))
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 不依赖 DB 的覆盖:
    /// - `BATCH_SIZE` 上限契约 (防雪崩)，下面是**编译期**断言
    /// - 抢占 SQL / 状态回写 SQL 的字面契约 (列名 + 状态字面量 + SKIP LOCKED)
    ///
    /// 说明: `tick` / `dispatch_one` / `mark_qa_blocked` 本身需要活的 PostgreSQL
    /// (sqlx 运行时校验 + 真实行锁语义), **本模块没有 DB 集成测试覆盖**。
    /// 抢占→派发→回写的行为级验证留 e2e-real-pg 阶段。
    ///
    /// 2026-10-05：`BATCH_SIZE` 这条原先是
    /// `#[test] fn batch_size_is_bounded_and_positive` 加两条
    /// `assert!(BATCH_SIZE ...)`。但 `BATCH_SIZE` 是 `const`，clippy 的
    /// `assertions_on_constants` 判它 "this assertion has a constant value"，
    /// 而 CI 跑 `clippy --workspace --all-features --all-targets -- -D warnings`，
    /// 三条平台全红。
    ///
    /// 这里**保留该 lint 但显式 allow**：它的建议是"改成运行时检查"，而在这个
    /// 场景里那恰恰是反的 —— 契约对象就是一个常量，改坏它时 crate 直接编不过
    /// 比"等跑测试才发现"强得多。allow 附在此处而不是 crate 级，理由写在这里，
    /// 免得下次有人以为可以整片关掉。
    #[allow(clippy::assertions_on_constants)]
    const _: () = {
        assert!(BATCH_SIZE > 0);
        assert!(BATCH_SIZE <= 1000);
    };

    #[test]
    fn claim_sql_uses_skip_locked_to_avoid_double_dispatch() {
        // 多 worker 并发时若去掉 SKIP LOCKED, 同一行会被重复抢占
        let sql = CLAIM_SQL;
        assert!(
            sql.contains("FOR UPDATE SKIP LOCKED"),
            "claim must use SKIP LOCKED"
        );
        assert!(
            sql.contains("status = 'pending'"),
            "claim must filter pending"
        );
        assert!(
            sql.contains("status = 'in_progress'"),
            "claim must set in_progress"
        );
    }

    #[test]
    fn complete_sql_marks_completed_and_touches_updated_at() {
        assert!(COMPLETE_SQL.contains("status = 'completed'"));
        assert!(COMPLETE_SQL.contains("updated_at = now()"));
    }

    #[test]
    fn qa_blocked_sql_marks_qa_blocked() {
        assert!(QA_BLOCKED_SQL.contains("status = 'qa_blocked'"));
        assert!(QA_BLOCKED_SQL.contains("updated_at = now()"));
    }
}
