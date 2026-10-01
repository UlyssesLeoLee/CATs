//! project_db 访问层 (sqlx)
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §5.1 (8 逻辑库)
//! 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md (projects 表)
//! 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.2 (project_db 接口契约 v1.0.0)
//! 引用: ULYS-150 切片 B-1 §"db.rs"
//!
//! 设计选择 (per 缺标比错标安全):
//! - 与 user_db 8 逻辑库边界一致, 各自独立 schema
//! - 不直连 user_db / workspace_db, 跨服务 user_id/workspace_id 一致性由调用方保证 (per T-02 schema 注释)
//! - migrate 由 main.rs 调用 sqlx::migrate! 走 migrations/ 目录

use crate::models::{Project, ProjectStatus};
use anyhow::{Context, Result};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::env;
use std::time::Duration;

/// 构造 project_db 连接池 (lazy, 不实际连 DB)
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

/// 创建项目
///
/// 返回 (新创建的 Project, true=新建 / false=某种原因返回 None 时 None)
pub async fn create(
    pool: &PgPool,
    workspace_id: uuid::Uuid,
    name: &str,
    source_lang: &str,
    target_lang: &str,
    owner_user_id: uuid::Uuid,
) -> Result<Project> {
    let row: Project = sqlx::query_as::<_, Project>(
        r#"
        INSERT INTO projects (workspace_id, name, source_lang, target_lang, owner_user_id)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, workspace_id, name, source_lang, target_lang, status, owner_user_id, created_at, updated_at
        "#,
    )
    .bind(workspace_id)
    .bind(name)
    .bind(source_lang)
    .bind(target_lang)
    .bind(owner_user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("create insert failed: db_err={}", e))?;
    Ok(row)
}

/// 按 id 查 Project
pub async fn find_by_id(pool: &PgPool, id: uuid::Uuid) -> Result<Option<Project>> {
    let row: Option<Project> = sqlx::query_as::<_, Project>(
        r#"
        SELECT id, workspace_id, name, source_lang, target_lang, status, owner_user_id, created_at, updated_at
        FROM projects
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("find_by_id query failed: db_err={}", e))?;
    Ok(row)
}

/// 列出项目 (按 workspace_id 过滤, 分页)
///
/// 返回 (items, total_count)
pub async fn list(
    pool: &PgPool,
    workspace_id: Option<uuid::Uuid>,
    page: i64,
    page_size: i64,
) -> Result<(Vec<Project>, i64)> {
    let offset = (page - 1).max(0) * page_size;

    // 总数 (per workspace_id 过滤)
    let total: i64 = if let Some(ws) = workspace_id {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)::BIGINT FROM projects WHERE workspace_id = $1
            "#,
        )
        .bind(ws)
        .fetch_one(pool)
        .await
        .map_err(|e| anyhow::anyhow!("list count failed: db_err={}", e))?
    } else {
        sqlx::query_scalar::<_, i64>(r#"SELECT COUNT(*)::BIGINT FROM projects"#)
            .fetch_one(pool)
            .await
            .map_err(|e| anyhow::anyhow!("list count failed: db_err={}", e))?
    };

    // 列表 (per workspace_id 过滤 + 分页)
    let rows: Vec<Project> = if let Some(ws) = workspace_id {
        sqlx::query_as::<_, Project>(
            r#"
            SELECT id, workspace_id, name, source_lang, target_lang, status, owner_user_id, created_at, updated_at
            FROM projects
            WHERE workspace_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(ws)
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| anyhow::anyhow!("list query failed: db_err={}", e))?
    } else {
        sqlx::query_as::<_, Project>(
            r#"
            SELECT id, workspace_id, name, source_lang, target_lang, status, owner_user_id, created_at, updated_at
            FROM projects
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| anyhow::anyhow!("list query failed: db_err={}", e))?
    };

    Ok((rows, total))
}

/// 部分更新 Project (按 id, name/source_lang/target_lang/status 任选)
///
/// None = 不变 (用 COALESCE 实现); status=Some(_) 时写入, 用 enum 字符串。
pub async fn update(
    pool: &PgPool,
    id: uuid::Uuid,
    name: Option<&str>,
    source_lang: Option<&str>,
    target_lang: Option<&str>,
    status: Option<ProjectStatus>,
) -> Result<Option<Project>> {
    let status_str: Option<&str> = status.map(|s| s.as_str());

    let row: Option<Project> = sqlx::query_as::<_, Project>(
        r#"
        UPDATE projects
        SET
            name        = COALESCE($2, name),
            source_lang = COALESCE($3, source_lang),
            target_lang = COALESCE($4, target_lang),
            status      = COALESCE($5, status)
        WHERE id = $1
        RETURNING id, workspace_id, name, source_lang, target_lang, status, owner_user_id, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(source_lang)
    .bind(target_lang)
    .bind(status_str)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("update failed: db_err={}", e))?;
    Ok(row)
}

/// 软删除 (per 切片 B-1 §"DELETE /v1/projects/{id}"): status='archived'
///
/// 返回 Some(Project) 当 archive 成功 (含 updated Project), None 当不存在。
pub async fn soft_delete(pool: &PgPool, id: uuid::Uuid) -> Result<Option<Project>> {
    let row: Option<Project> = sqlx::query_as::<_, Project>(
        r#"
        UPDATE projects
        SET status = 'archived'
        WHERE id = $1
        RETURNING id, workspace_id, name, source_lang, target_lang, status, owner_user_id, created_at, updated_at
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("soft_delete failed: db_err={}", e))?;
    Ok(row)
}