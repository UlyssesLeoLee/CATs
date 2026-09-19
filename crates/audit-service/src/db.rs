//! audit_db SQL 访问层
//!
//! 角色: `svc_audit` (CRUD), migration 用 `migrator_audit`

use crate::models::{AuditLogRow, KafkaAuditEvent};
use cats_common::{CatsError, ErrorCode};
use sqlx::PgPool;
use uuid::Uuid;

/// 建池 (DATABASE_URL 由调用方保证已设)
pub async fn build_pool() -> Result<PgPool, CatsError> {
    let url = std::env::var("DATABASE_URL")
        .map_err(|_| CatsError::business(ErrorCode::InternalError, "DATABASE_URL not set"))?;
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(&url)
        .await
        .map_err(|e| CatsError::business(ErrorCode::InternalError, format!("connect fail: {e}")))
}

/// sqlx-migrate 自动跑 (从 crates/audit-service/migrations/*.sql)
pub async fn run_migrations(pool: &PgPool) -> Result<(), CatsError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| CatsError::business(ErrorCode::InternalError, format!("migrate fail: {e}")))
}

/// 插入一条 audit log (从 Kafka event 落档)
pub async fn insert_event(pool: &PgPool, ev: &KafkaAuditEvent) -> Result<i64, CatsError> {
    let ip: Option<sqlx::types::ipnetwork::IpNetwork> = ev
        .ip
        .as_ref()
        .and_then(|s| s.parse().ok());
    let row: (i64,) = sqlx::query_as(
        r#"INSERT INTO audit_logs (event_id, org_id, actor_user_id, action, resource_type, resource_id,
                                  before_state, after_state, ip, occurred_at)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
           ON CONFLICT (event_id) DO UPDATE SET ingested_at = now()
           RETURNING id"#,
    )
    .bind(ev.event_id)
    .bind(ev.org_id)
    .bind(ev.actor_user_id)
    .bind(&ev.action)
    .bind(&ev.resource_type)
    .bind(&ev.resource_id)
    .bind(&ev.before_state)
    .bind(&ev.after_state)
    .bind(ip)
    .bind(ev.occurred_at)
    .fetch_one(pool)
    .await
    .map_err(|e| CatsError::business(ErrorCode::InternalError, format!("insert fail: {e}")))?;
    Ok(row.0)
}

/// 按 org 分页查询 (供 GET /v1/audit-logs 用)
pub async fn list_by_org(
    pool: &PgPool,
    org_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<AuditLogRow>, CatsError> {
    sqlx::query_as::<_, AuditLogRow>(
        r#"SELECT id, event_id, org_id, actor_user_id, action, resource_type, resource_id,
                  before_state, after_state, ip, occurred_at, ingested_at
           FROM audit_logs
           WHERE org_id = $1
           ORDER BY occurred_at DESC
           LIMIT $2 OFFSET $3"#,
    )
    .bind(org_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| CatsError::business(ErrorCode::InternalError, format!("query fail: {e}")))
}

/// 总数
pub async fn count_by_org(pool: &PgPool, org_id: Uuid) -> Result<i64, CatsError> {
    let row: (i64,) = sqlx::query_as(r#"SELECT COUNT(*) FROM audit_logs WHERE org_id = $1"#)
        .bind(org_id)
        .fetch_one(pool)
        .await
        .map_err(|e| CatsError::business(ErrorCode::InternalError, format!("count fail: {e}")))?;
    Ok(row.0)
}