//! file_db 访问层 (sqlx)
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1 (file-service)
//! 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.1 (file_db 接口契约 v1.0.0)
//!
//! 设计选择 (per 缺标比错标安全, 守门 #11):
//! - 8 逻辑库各自独立 schema, 不直连 user_db / auth_db
//! - 软删除 (status='deleted') 而非物理删除
//! - storage_path 是抽象: M1 本地磁盘 / Sprint 3 S3 key

use crate::models::{FileRecord, NewFileRecord};
use anyhow::{Context, Result};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::env;
use std::time::Duration;

/// 构造 file_db 连接池 (lazy, 不实际连 DB)
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

/// 插入文件元数据 (返回完整记录)
pub async fn insert(pool: &PgPool, rec: &NewFileRecord) -> Result<FileRecord> {
    let row: FileRecord = sqlx::query_as::<_, FileRecord>(
        r#"
        INSERT INTO files (
            workspace_id, owner_user_id, filename, content_type,
            size_bytes, storage_path, sha256
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, workspace_id, owner_user_id, filename, content_type,
                  size_bytes, storage_path, sha256, status, created_at, updated_at
        "#,
    )
    .bind(rec.workspace_id)
    .bind(rec.owner_user_id)
    .bind(&rec.filename)
    .bind(&rec.content_type)
    .bind(rec.size_bytes)
    .bind(&rec.storage_path)
    .bind(&rec.sha256)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("insert file failed: db_err={}", e))?;
    Ok(row)
}

/// 按 id 查 FileRecord (任意状态)
pub async fn find_by_id(pool: &PgPool, id: uuid::Uuid) -> Result<Option<FileRecord>> {
    let row: Option<FileRecord> = sqlx::query_as::<_, FileRecord>(
        r#"
        SELECT id, workspace_id, owner_user_id, filename, content_type,
               size_bytes, storage_path, sha256, status, created_at, updated_at
        FROM files
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("find_by_id failed: db_err={}", e))?;
    Ok(row)
}

/// 按 workspace_id 列出 active 文件 (分页)
pub async fn list_by_workspace(
    pool: &PgPool,
    workspace_id: uuid::Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<FileRecord>> {
    let rows: Vec<FileRecord> = sqlx::query_as::<_, FileRecord>(
        r#"
        SELECT id, workspace_id, owner_user_id, filename, content_type,
               size_bytes, storage_path, sha256, status, created_at, updated_at
        FROM files
        WHERE workspace_id = $1 AND status = 'active'
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(workspace_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("list_by_workspace failed: db_err={}", e))?;
    Ok(rows)
}

/// 按 workspace_id 统计 active 文件数
pub async fn count_by_workspace(pool: &PgPool, workspace_id: uuid::Uuid) -> Result<i64> {
    let row: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)::BIGINT
        FROM files
        WHERE workspace_id = $1 AND status = 'active'
        "#,
    )
    .bind(workspace_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("count_by_workspace failed: db_err={}", e))?;
    Ok(row.0)
}

/// 软删除 (status='deleted')
///
/// 返回: Ok(true) 删除成功, Ok(false) 文件不存在或已删除, Err 数据库错误
pub async fn soft_delete(pool: &PgPool, id: uuid::Uuid) -> Result<bool> {
    let row: Option<(uuid::Uuid,)> = sqlx::query_as(
        r#"
        UPDATE files
        SET status = 'deleted'
        WHERE id = $1 AND status = 'active'
        RETURNING id
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("soft_delete failed: db_err={}", e))?;
    Ok(row.is_some())
}