//! notification_db 访问层 (sqlx)
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1 (notification-service)
//! 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.1 (notification_db 接口契约 v1.0.0)
//!
//! 设计选择 (per 缺标比错标安全, 守门 #11):
//! - 8 逻辑库各自独立 schema, 不直连 user_db / auth_db
//! - 软删除 (status='deleted') 而非物理删除
//! - 列出按 created_at DESC (per 业务: 最新通知优先)

use crate::models::NotificationRecord;
use anyhow::{Context, Result};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::env;
use std::time::Duration;

/// 构造 notification_db 连接池 (lazy, 不实际连 DB)
pub async fn build_pool() -> Result<PgPool> {
    let url = env::var("DATABASE_URL")
        .context("DATABASE_URL env var not set (per §5.2 注入规范)")?;
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(3))
        .connect_lazy(&url)
        .context("PgPool lazy build failed")?;
    Ok(pool)
}

/// 插入通知
pub async fn insert(
    pool: &PgPool,
    user_id: uuid::Uuid,
    notif_type: &str,
    title: &str,
    body: &str,
    payload: &serde_json::Value,
) -> Result<NotificationRecord> {
    let row: NotificationRecord = sqlx::query_as::<_, NotificationRecord>(
        r#"
        INSERT INTO notifications (user_id, type, title, body, payload)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, user_id, type, title, body, payload, read_at, status, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(notif_type)
    .bind(title)
    .bind(body)
    .bind(payload)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("insert notification failed: db_err={}", e))?;
    Ok(row)
}

/// 按 user_id 列出 active 通知 (分页, 按 created_at DESC)
pub async fn list_by_user(
    pool: &PgPool,
    user_id: uuid::Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<NotificationRecord>> {
    let rows: Vec<NotificationRecord> = sqlx::query_as::<_, NotificationRecord>(
        r#"
        SELECT id, user_id, type, title, body, payload, read_at, status, created_at, updated_at
        FROM notifications
        WHERE user_id = $1 AND status = 'active'
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("list_by_user failed: db_err={}", e))?;
    Ok(rows)
}

/// 按 user_id 统计 active 通知数
pub async fn count_by_user(pool: &PgPool, user_id: uuid::Uuid) -> Result<i64> {
    let row: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)::BIGINT
        FROM notifications
        WHERE user_id = $1 AND status = 'active'
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("count_by_user failed: db_err={}", e))?;
    Ok(row.0)
}

/// 标记已读 (返回 Ok(true) 标记成功, Ok(false) 通知不存在/已读/已删)
pub async fn mark_read(pool: &PgPool, id: uuid::Uuid, user_id: uuid::Uuid) -> Result<bool> {
    let row: Option<(uuid::Uuid,)> = sqlx::query_as(
        r#"
        UPDATE notifications
        SET read_at = now()
        WHERE id = $1 AND user_id = $2 AND read_at IS NULL AND status = 'active'
        RETURNING id
        "#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("mark_read failed: db_err={}", e))?;
    Ok(row.is_some())
}

/// 按 id 查 (用于 SSE 推送后的二次读取 / 测试)
pub async fn find_by_id(pool: &PgPool, id: uuid::Uuid) -> Result<Option<NotificationRecord>> {
    let row: Option<NotificationRecord> = sqlx::query_as::<_, NotificationRecord>(
        r#"
        SELECT id, user_id, type, title, body, payload, read_at, status, created_at, updated_at
        FROM notifications
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("find_by_id failed: db_err={}", e))?;
    Ok(row)
}