//! PostgreSQL 数据访问层 (project-service)
//!
//! 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §6.4
//!
//! MVP 简化: 使用 `sqlx::query_as` (动态 bind) 而非 `query_as!` (compile-time check),
//! 避免对 DATABASE_URL 在线连接的依赖 (per Sprint 1 实施前 QA 调研 + MVP 阶段不连 DB).

use crate::models::{ProjectMemberRow, ProjectRow};
use cats_common::CatsError;
use sqlx::PgPool;

/// 启动时建 pool (DATABASE_URL env)
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

/// 启动时跑迁移 (idempotent)
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
    name: &str,
    description: Option<&str>,
    source_lang: Option<&str>,
    target_lang: Option<&str>,
    created_by: uuid::Uuid,
) -> Result<ProjectRow, CatsError> {
    let row = sqlx::query_as::<_, ProjectRow>(
        r#"
        INSERT INTO projects (org_id, name, description, source_lang, target_lang, created_by)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, org_id, name, description, status, source_lang, target_lang,
                  created_by, created_at, updated_at
        "#,
    )
    .bind(org_id)
    .bind(name)
    .bind(description)
    .bind(source_lang)
    .bind(target_lang)
    .bind(created_by)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn find_by_id(
    pool: &PgPool,
    id: uuid::Uuid,
) -> Result<Option<ProjectRow>, CatsError> {
    let row = sqlx::query_as::<_, ProjectRow>(
        r#"
        SELECT id, org_id, name, description, status, source_lang, target_lang,
               created_by, created_at, updated_at
        FROM projects WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn update(
    pool: &PgPool,
    id: uuid::Uuid,
    name: Option<&str>,
    description: Option<&str>,
    status: Option<&str>,
) -> Result<Option<ProjectRow>, CatsError> {
    let row = sqlx::query_as::<_, ProjectRow>(
        r#"
        UPDATE projects
        SET name = COALESCE($2, name),
            description = COALESCE($3, description),
            status = COALESCE($4, status)
        WHERE id = $1
        RETURNING id, org_id, name, description, status, source_lang, target_lang,
                  created_by, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(status)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn list_by_org(
    pool: &PgPool,
    org_id: uuid::Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<ProjectRow>, CatsError> {
    let rows = sqlx::query_as::<_, ProjectRow>(
        r#"
        SELECT id, org_id, name, description, status, source_lang, target_lang,
               created_by, created_at, updated_at
        FROM projects
        WHERE org_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(org_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn count_by_org(pool: &PgPool, org_id: uuid::Uuid) -> Result<i64, CatsError> {
    let row: (i64,) = sqlx::query_as(r#"SELECT COUNT(*) FROM projects WHERE org_id = $1"#)
        .bind(org_id)
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

pub async fn add_member(
    pool: &PgPool,
    project_id: uuid::Uuid,
    user_id: uuid::Uuid,
    role: &str,
) -> Result<ProjectMemberRow, CatsError> {
    let row = sqlx::query_as::<_, ProjectMemberRow>(
        r#"
        INSERT INTO project_members (project_id, user_id, role)
        VALUES ($1, $2, $3)
        ON CONFLICT (project_id, user_id) DO UPDATE SET role = EXCLUDED.role
        RETURNING project_id, user_id, role, added_at
        "#,
    )
    .bind(project_id)
    .bind(user_id)
    .bind(role)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn list_members(
    pool: &PgPool,
    project_id: uuid::Uuid,
) -> Result<Vec<ProjectMemberRow>, CatsError> {
    let rows = sqlx::query_as::<_, ProjectMemberRow>(
        r#"
        SELECT project_id, user_id, role, added_at
        FROM project_members WHERE project_id = $1 ORDER BY added_at
        "#,
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}