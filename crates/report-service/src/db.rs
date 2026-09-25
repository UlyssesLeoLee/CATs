//! report_db SQL 访问层 (per ULYS-153 切片 C-1)
//!
//! 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md (report_db 视图)
//! 引用: api/openapi/cats-openapi-v1.0.1.yaml §/v1/reports
//!
//! 数据源说明 (per 切片 C-1 §数据源 + 微服务架构书 §1.2 原则 4):
//! - 本切片 MVP 默认从本 crate 视角聚合 (SQL 函数级别跨表 JOIN 需要 ops 侧
//!   配置 postgres_fdw / dblink 指向 audit_db, task_db, translation_db)
//! - SQL 函数使用 schema 限定 (`audit.audit_logs`) 让 ops 侧可在 report_db
//!   创建对应 foreign schema 后零代码改动启用跨库聚合
//!
//! 所有查询都用 sqlx 直查, 不拉数据到内存做后聚合 (per 切片 C-1 §数据源
//! "直接 SQL 聚合, 不要拉数据到内存")

use crate::models::{ActionCountRow, AuditSummary, TranslationVolumeRow};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

// =====================================================================
// 1. /v1/reports/usage — usage stats grouped by action × resource_type
// =====================================================================

/// 按 org 在 [from, to] 区间, 按 action × resource_type 聚合 audit_logs
///
/// SQL: 直接 GROUP BY, 不拉数据到内存. 即使 audit_logs 百万行也能秒回
pub async fn usage_by_org(
    pool: &PgPool,
    org_id: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<ActionCountRow>, String> {
    // 注意: 这里用 schema-qualified `audit.audit_logs` 让 ops 侧配置
    // postgres_fdw 指向 audit_db 后无需改代码; 若 ops 未配置, 这条 SQL
    // 会失败, 此时 report-service 退化只服务本地聚合表 (见 README)
    let rows: Vec<ActionCountRow> = sqlx::query_as::<_, ActionCountRow>(
        r#"
        SELECT
            action,
            resource_type,
            COUNT(*)::BIGINT                          AS event_count,
            COUNT(DISTINCT actor_user_id)::BIGINT      AS distinct_actors
        FROM audit.audit_logs
        WHERE org_id = $1
          AND occurred_at >= $2
          AND occurred_at <  $3
        GROUP BY action, resource_type
        ORDER BY event_count DESC
        "#,
    )
    .bind(org_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("usage_by_org query failed: {e}"))?;
    Ok(rows)
}

/// usage 总条数 (per org 在 [from, to])
pub async fn usage_total(
    pool: &PgPool,
    org_id: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<i64, String> {
    let row: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)::BIGINT
        FROM audit.audit_logs
        WHERE org_id = $1
          AND occurred_at >= $2
          AND occurred_at <  $3
        "#,
    )
    .bind(org_id)
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("usage_total query failed: {e}"))?;
    Ok(row.0)
}

// =====================================================================
// 2. /v1/reports/translation-volume — translation events by project
// =====================================================================

/// 翻译量按 project_id + [from, to] 区间, 按 day 聚合
///
/// SQL: 用 date_trunc('day', occurred_at) 分组, action 限定 translate.completed
pub async fn translation_volume_by_project(
    pool: &PgPool,
    project_id: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<TranslationVolumeRow>, String> {
    // resource_id 在 audit_logs 里是 TEXT, 假定 project_id 直接存为 resource_id
    // (per task-service / translation-core 集成约定, action='translate.completed')
    let rows: Vec<(chrono::NaiveDate, i64, Option<i64>)> = sqlx::query_as(
        r#"
        SELECT
            (occurred_at AT TIME ZONE 'UTC')::date  AS day,
            COUNT(*)::BIGINT                          AS completed_tasks,
            SUM( COALESCE( (after_state->>'char_count')::BIGINT, 0) )::BIGINT AS total_chars
        FROM audit.audit_logs
        WHERE action         = 'translate.completed'
          AND resource_type  = 'project'
          AND resource_id    = $1
          AND occurred_at   >= $2
          AND occurred_at   <  $3
        GROUP BY day
        ORDER BY day ASC
        "#,
    )
    .bind(project_id.to_string())
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("translation_volume query failed: {e}"))?;

    Ok(rows
        .into_iter()
        .map(|(day, completed_tasks, total_chars)| TranslationVolumeRow {
            day,
            completed_tasks,
            total_chars,
        })
        .collect())
}

/// 翻译总量 (per project 在 [from, to])
pub async fn translation_volume_total(
    pool: &PgPool,
    project_id: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<(i64, i64), String> {
    let row: (i64, Option<i64>) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*)::BIGINT,
            SUM( COALESCE( (after_state->>'char_count')::BIGINT, 0) )
        FROM audit.audit_logs
        WHERE action         = 'translate.completed'
          AND resource_type  = 'project'
          AND resource_id    = $1
          AND occurred_at   >= $2
          AND occurred_at   <  $3
        "#,
    )
    .bind(project_id.to_string())
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("translation_volume_total query failed: {e}"))?;
    Ok((row.0, row.1.unwrap_or(0)))
}

// =====================================================================
// 3. /v1/reports/audit-summary — audit summary for a workspace
// =====================================================================

/// 审计摘要 — topN action + 总数 + distinct actors + last event time
///
/// per workspace = per org (微服务架构中 workspace ≡ org)
pub async fn audit_summary_by_workspace(
    pool: &PgPool,
    workspace_id: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<
    (
        i64,                // total_events
        i64,                // distinct_actors
        Option<DateTime<Utc>>, // last_event_at
        Vec<AuditSummary>,  // top_actions (top 10 by count)
    ),
    String,
> {
    // 一次查询拿所有聚合, 避免多次 round trip
    let rows: Vec<(String, i64)> = sqlx::query_as(
        r#"
        SELECT action, COUNT(*)::BIGINT AS cnt
        FROM audit.audit_logs
        WHERE org_id = $1
          AND occurred_at >= $2
          AND occurred_at <  $3
        GROUP BY action
        ORDER BY cnt DESC
        LIMIT 10
        "#,
    )
    .bind(workspace_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("audit_summary top_actions query failed: {e}"))?;

    let total_row: (i64, Option<i64>, Option<DateTime<Utc>>) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*)::BIGINT,
            COUNT(DISTINCT actor_user_id)::BIGINT,
            MAX(occurred_at)
        FROM audit.audit_logs
        WHERE org_id = $1
          AND occurred_at >= $2
          AND occurred_at <  $3
        "#,
    )
    .bind(workspace_id)
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("audit_summary totals query failed: {e}"))?;

    let top_actions = rows
        .into_iter()
        .map(|(action, cnt)| AuditSummary {
            action,
            event_count: cnt,
        })
        .collect();

    Ok((
        total_row.0,
        total_row.1.unwrap_or(0),
        total_row.2,
        top_actions,
    ))
}
