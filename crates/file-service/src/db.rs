//! PostgreSQL 数据访问层 (file-service)

use crate::models::FileRow;
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
    org_id: uuid::Uuid,
    project_id: Option<uuid::Uuid>,
    task_id: Option<uuid::Uuid>,
    name: &str,
    content_type: Option<&str>,
    size_bytes: i64,
    sha256: Option<&str>,
    storage_path: &str,
    uploaded_by: uuid::Uuid,
) -> Result<FileRow, CatsError> {
    let row = sqlx::query_as::<_, FileRow>(
        r#"
        INSERT INTO files (org_id, project_id, task_id, name, content_type, size_bytes,
                           sha256, storage_path, uploaded_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, org_id, project_id, task_id, name, content_type, size_bytes, sha256,
                  storage_path, status, uploaded_by, created_at, updated_at
        "#,
    )
    .bind(org_id)
    .bind(project_id)
    .bind(task_id)
    .bind(name)
    .bind(content_type)
    .bind(size_bytes)
    .bind(sha256)
    .bind(storage_path)
    .bind(uploaded_by)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn find_by_id(pool: &PgPool, id: uuid::Uuid) -> Result<Option<FileRow>, CatsError> {
    let row = sqlx::query_as::<_, FileRow>(
        r#"
        SELECT id, org_id, project_id, task_id, name, content_type, size_bytes, sha256,
               storage_path, status, uploaded_by, created_at, updated_at
        FROM files WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn list(
    pool: &PgPool,
    org_id: uuid::Uuid,
    project_id: Option<uuid::Uuid>,
    limit: i64,
    offset: i64,
) -> Result<Vec<FileRow>, CatsError> {
    let rows = sqlx::query_as::<_, FileRow>(
        r#"
        SELECT id, org_id, project_id, task_id, name, content_type, size_bytes, sha256,
               storage_path, status, uploaded_by, created_at, updated_at
        FROM files
        WHERE org_id = $1
          AND ($2::uuid IS NULL OR project_id = $2)
        ORDER BY created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(org_id)
    .bind(project_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}