//! PostgreSQL 数据访问层 (notification-service)

use crate::models::{KafkaEvent, NotificationRow};
use cats_common::CatsError;
use sqlx::PgPool;

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

pub async fn run_migrations(pool: &PgPool) -> Result<(), CatsError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| CatsError::internal("migration failed", e.to_string()))?;
    Ok(())
}

/// 从 Kafka event 落库 (MVP: 写库 + tracing log)
pub async fn insert_from_event(pool: &PgPool, ev: &KafkaEvent) -> Result<NotificationRow, CatsError> {
    let row = sqlx::query_as::<_, NotificationRow>(
        r#"
        INSERT INTO notifications (org_id, user_id, channel, event_type, payload, status, sent_at)
        VALUES ($1, $2, 'in_app', $3, $4, 'sent', now())
        RETURNING id, org_id, user_id, channel, event_type, payload, status, created_at, sent_at
        "#,
    )
    .bind(ev.org_id)
    .bind(ev.user_id)
    .bind(&ev.event_type)
    .bind(&ev.payload)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn list_by_user(
    pool: &PgPool,
    user_id: uuid::Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<NotificationRow>, CatsError> {
    let rows = sqlx::query_as::<_, NotificationRow>(
        r#"
        SELECT id, org_id, user_id, channel, event_type, payload, status, created_at, sent_at
        FROM notifications
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn count_by_user(pool: &PgPool, user_id: uuid::Uuid) -> Result<i64, CatsError> {
    let row: (i64,) = sqlx::query_as(r#"SELECT COUNT(*) FROM notifications WHERE user_id = $1"#)
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}