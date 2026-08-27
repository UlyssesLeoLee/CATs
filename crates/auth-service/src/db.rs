//! auth_db 访问层 (sqlx)
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §5.1
//! 引用: doc/05-其他/CATs_实施前QA登记册_v1.3.md §2.2 (D-Day 验证清单)

use crate::models::UserCredential;
use anyhow::{Context, Result};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::env;
use std::time::Duration;

/// 构造 auth_db 连接池 (lazy, 不实际连 DB)
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

/// 按 username 查 UserCredential
pub async fn find_by_username(pool: &PgPool, username: &str) -> Result<Option<UserCredential>> {
    let row: Option<UserCredential> = sqlx::query_as::<_, UserCredential>(
        r#"
        SELECT id, username, password_hash, is_active, created_at, updated_at
        FROM users_credential
        WHERE username = $1
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .context("find_by_username query failed")?;
    Ok(row)
}

/// 按 user_id 查 UserCredential (供 refresh token 用)
pub async fn find_by_id(pool: &PgPool, id: uuid::Uuid) -> Result<Option<UserCredential>> {
    let row: Option<UserCredential> = sqlx::query_as::<_, UserCredential>(
        r#"
        SELECT id, username, password_hash, is_active, created_at, updated_at
        FROM users_credential
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("find_by_id query failed")?;
    Ok(row)
}

/// 启动时检测种子用户 (per 任务规范: 不存明文 hash, 启动时检测并自动创建)
pub async fn ensure_seed_user(pool: &PgPool, username: &str, plain_password: &str) -> Result<bool> {
    use crate::auth::hash_password;
    let existing = find_by_username(pool, username).await?;
    if existing.is_some() {
        return Ok(false);
    }
    let hash = hash_password(plain_password)?;
    sqlx::query(
        r#"
        INSERT INTO users_credential (username, password_hash)
        VALUES ($1, $2)
        ON CONFLICT (username) DO NOTHING
        "#,
    )
    .bind(username)
    .bind(&hash)
    .execute(pool)
    .await
    .context("seed user insert failed")?;
    Ok(true)
}
