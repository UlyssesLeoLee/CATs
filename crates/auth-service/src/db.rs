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
        SELECT id, username, email, password_hash, is_active, created_at, updated_at
        FROM users_credential
        WHERE username = $1
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("find_by_username query failed: db_err={}", e))?;
    Ok(row)
}

/// 按 user_id 查 UserCredential (供 refresh token / me 端点用)
pub async fn find_by_id(pool: &PgPool, id: uuid::Uuid) -> Result<Option<UserCredential>> {
    let row: Option<UserCredential> = sqlx::query_as::<_, UserCredential>(
        r#"
        SELECT id, username, email, password_hash, is_active, created_at, updated_at
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
///
/// `email` 可选: None 表示不设置 (适合不关心 email 的本地开发场景)
pub async fn ensure_seed_user(
    pool: &PgPool,
    username: &str,
    plain_password: &str,
    email: Option<&str>,
) -> Result<bool> {
    use crate::auth::hash_password;
    let existing = find_by_username(pool, username).await?;
    if existing.is_some() {
        // 已存在则补 email (允许 None 不覆盖)
        if let Some(em) = email {
            sqlx::query(
                r#"
                UPDATE users_credential
                SET email = COALESCE(email, $1)
                WHERE username = $2
                "#,
            )
            .bind(em)
            .bind(username)
            .execute(pool)
            .await
            .context("seed user email update failed")?;
        }
        return Ok(false);
    }
    let hash = hash_password(plain_password)?;
    sqlx::query(
        r#"
        INSERT INTO users_credential (username, email, password_hash)
        VALUES ($1, $2, $3)
        ON CONFLICT (username) DO NOTHING
        "#,
    )
    .bind(username)
    .bind(email)
    .bind(&hash)
    .execute(pool)
    .await
    .context("seed user insert failed")?;
    Ok(true)
}

// =====================================================================
// T-01: refresh_token 撤销 + 审计事件落库
// 引用: doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md §2 T-01
// =====================================================================

/// 检查 jti 是否已被撤销 (refresh 路由必须先查)
pub async fn jti_is_revoked(pool: &PgPool, jti: uuid::Uuid) -> Result<bool> {
    let row: Option<(uuid::Uuid,)> = sqlx::query_as(
        r#"
        SELECT jti FROM refresh_token_revoke WHERE jti = $1
        "#,
    )
    .bind(jti)
    .fetch_optional(pool)
    .await
    .context("jti_is_revoked query failed")?;
    Ok(row.is_some())
}

/// 撤销一个 jti (refresh 轮换旧 token / logout 撤销)
///
/// reason: 'rotated' | 'logout' | 'admin_revoke' | 'expired'
pub async fn revoke_jti(
    pool: &PgPool,
    jti: uuid::Uuid,
    user_id: uuid::Uuid,
    reason: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO refresh_token_revoke (jti, user_id, reason)
        VALUES ($1, $2, $3)
        ON CONFLICT (jti) DO NOTHING
        "#,
    )
    .bind(jti)
    .bind(user_id)
    .bind(reason)
    .execute(pool)
    .await
    .context("revoke_jti insert failed")?;
    Ok(())
}

/// 查某事件类型最近 N 条 (测试断言用) — 查 audit_log raw row
///
/// 返回 8-tuple 列, 业务上由调用方按需组装 AuditEvent
pub async fn recent_audit_events(
    pool: &PgPool,
    event_type_filter: &str,
    limit_n: i64,
) -> Result<Vec<crate::models::AuditEventRow>> {
    let rows: Vec<crate::models::AuditEventRow> = sqlx::query_as(
        r#"
        SELECT
            event_id,
            user_id,
            event_type,
            outcome,
            detail,
            source_ip::text AS source_ip,
            user_agent,
            occurred_at
        FROM audit_log
        WHERE event_type = $1
        ORDER BY occurred_at DESC
        LIMIT $2
        "#,
    )
    .bind(event_type_filter)
    .bind(limit_n)
    .fetch_all(pool)
    .await
    .context("recent_audit_events query failed")?;
    Ok(rows)
}
