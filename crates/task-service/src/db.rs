//! PostgreSQL 数据访问层 (task-service)

use crate::models::TaskRow;
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

pub async fn create(
    pool: &PgPool,
    project_id: uuid::Uuid,
    org_id: uuid::Uuid,
    title: &str,
    description: Option<&str>,
    media_type: &str,
    source_lang: Option<&str>,
    target_lang: Option<&str>,
    assigned_to: Option<uuid::Uuid>,
    created_by: uuid::Uuid,
) -> Result<TaskRow, CatsError> {
    let row = sqlx::query_as::<_, TaskRow>(
        r#"
        INSERT INTO tasks (project_id, org_id, title, description, media_type,
                           source_lang, target_lang, assigned_to, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, project_id, org_id, title, description, status, media_type,
                  source_lang, target_lang, assigned_to, qa_violations,
                  created_by, created_at, updated_at
        "#,
    )
    .bind(project_id)
    .bind(org_id)
    .bind(title)
    .bind(description)
    .bind(media_type)
    .bind(source_lang)
    .bind(target_lang)
    .bind(assigned_to)
    .bind(created_by)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn find_by_id(pool: &PgPool, id: uuid::Uuid) -> Result<Option<TaskRow>, CatsError> {
    let row = sqlx::query_as::<_, TaskRow>(
        r#"
        SELECT id, project_id, org_id, title, description, status, media_type,
               source_lang, target_lang, assigned_to, qa_violations,
               created_by, created_at, updated_at
        FROM tasks WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn update_status(
    pool: &PgPool,
    id: uuid::Uuid,
    status: &str,
    qa_violations: Option<serde_json::Value>,
) -> Result<Option<TaskRow>, CatsError> {
    let row = sqlx::query_as::<_, TaskRow>(
        r#"
        UPDATE tasks
        SET status = $2,
            qa_violations = COALESCE($3, qa_violations)
        WHERE id = $1
        RETURNING id, project_id, org_id, title, description, status, media_type,
                  source_lang, target_lang, assigned_to, qa_violations,
                  created_by, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(status)
    .bind(qa_violations)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn list(
    pool: &PgPool,
    org_id: uuid::Uuid,
    project_id: Option<uuid::Uuid>,
    status: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<TaskRow>, CatsError> {
    let rows = sqlx::query_as::<_, TaskRow>(
        r#"
        SELECT id, project_id, org_id, title, description, status, media_type,
               source_lang, target_lang, assigned_to, qa_violations,
               created_by, created_at, updated_at
        FROM tasks
        WHERE org_id = $1
          AND ($2::uuid IS NULL OR project_id = $2)
          AND ($3::text IS NULL OR status = $3)
        ORDER BY created_at DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(org_id)
    .bind(project_id)
    .bind(status)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn count(
    pool: &PgPool,
    org_id: uuid::Uuid,
    project_id: Option<uuid::Uuid>,
    status: Option<&str>,
) -> Result<i64, CatsError> {
    let row: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM tasks
        WHERE org_id = $1
          AND ($2::uuid IS NULL OR project_id = $2)
          AND ($3::text IS NULL OR status = $3)
        "#,
    )
    .bind(org_id)
    .bind(project_id)
    .bind(status)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// 写入 outbox (per §2 Outbox 模式)
pub async fn enqueue_outbox(
    pool: &PgPool,
    task_id: uuid::Uuid,
    event_type: &str,
    payload: serde_json::Value,
) -> Result<(), CatsError> {
    sqlx::query(
        r#"
        INSERT INTO task_events_outbox (task_id, event_type, payload)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(task_id)
    .bind(event_type)
    .bind(payload)
    .execute(pool)
    .await?;
    Ok(())
}