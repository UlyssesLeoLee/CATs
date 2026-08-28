//! user_db 访问层 (sqlx)
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §5.1 (8 逻辑库)
//! 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.1 (user_db 接口契约 v1.0.0)
//!
//! 设计选择 (per 缺标比错标安全):
//! - 与 auth_db 8 逻辑库边界一致, 各自独立 schema
//! - 不直连 auth_db, 跨服务 user_id 一致性由调用方保证 (per T-02 schema 注释)

use crate::models::UserProfile;
use anyhow::{Context, Result};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::env;
use std::time::Duration;

/// 构造 user_db 连接池 (lazy, 不实际连 DB)
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

/// 按 id 查 UserProfile
pub async fn find_by_id(pool: &PgPool, id: uuid::Uuid) -> Result<Option<UserProfile>> {
    let row: Option<UserProfile> = sqlx::query_as::<_, UserProfile>(
        r#"
        SELECT id, user_id, display_name, email, avatar_url, locale, timezone, is_active, created_at, updated_at
        FROM user_profile
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("find_by_id query failed: db_err={}", e))?;
    Ok(row)
}

/// 按 user_id 查 UserProfile (per 业务: 跨服务 user_id 一致)
pub async fn find_by_user_id(pool: &PgPool, user_id: uuid::Uuid) -> Result<Option<UserProfile>> {
    let row: Option<UserProfile> = sqlx::query_as::<_, UserProfile>(
        r#"
        SELECT id, user_id, display_name, email, avatar_url, locale, timezone, is_active, created_at, updated_at
        FROM user_profile
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("find_by_user_id query failed: db_err={}", e))?;
    Ok(row)
}

/// 创建 UserProfile
///
/// 返回 (新创建的 UserProfile, 是否新建 true / 已存在 false)
pub async fn create(
    pool: &PgPool,
    user_id: uuid::Uuid,
    display_name: &str,
    email: Option<&str>,
    avatar_url: Option<&str>,
    locale: &str,
    timezone: &str,
) -> Result<(UserProfile, bool)> {
    // 先查 (幂等: 已存在则返回, 不重复创建)
    if let Some(existing) = find_by_user_id(pool, user_id).await? {
        return Ok((existing, false));
    }
    let row: UserProfile = sqlx::query_as::<_, UserProfile>(
        r#"
        INSERT INTO user_profile (user_id, display_name, email, avatar_url, locale, timezone)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, user_id, display_name, email, avatar_url, locale, timezone, is_active, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(display_name)
    .bind(email)
    .bind(avatar_url)
    .bind(locale)
    .bind(timezone)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("create insert failed: db_err={}", e))?;
    Ok((row, true))
}

/// 更新 UserProfile (按 id, 部分字段)
pub async fn update(
    pool: &PgPool,
    id: uuid::Uuid,
    display_name: Option<&str>,
    email: Option<&str>,
    avatar_url: Option<&str>,
    locale: Option<&str>,
    timezone: Option<&str>,
) -> Result<Option<UserProfile>> {
    // 用 COALESCE 实现部分更新 (None 不变)
    let row: Option<UserProfile> = sqlx::query_as::<_, UserProfile>(
        r#"
        UPDATE user_profile
        SET
            display_name = COALESCE($2, display_name),
            email        = COALESCE($3, email),
            avatar_url   = COALESCE($4, avatar_url),
            locale       = COALESCE($5, locale),
            timezone     = COALESCE($6, timezone)
        WHERE id = $1
        RETURNING id, user_id, display_name, email, avatar_url, locale, timezone, is_active, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(display_name)
    .bind(email)
    .bind(avatar_url)
    .bind(locale)
    .bind(timezone)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("update failed: db_err={}", e))?;
    Ok(row)
}
