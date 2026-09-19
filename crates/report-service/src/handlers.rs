//! HTTP handlers (report-service)
//!
//! MVP 端点:
//! - GET /healthz
//! - GET /readyz
//! - GET /v1/reports/usage?org_id=...&from=...&to=...

use crate::db;
use crate::models::UsageReport;
use crate::state::AppState;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use cats_common::{cats_error_to_response, CatsError, ErrorCode};
use cats_rbac::service_helpers::require_roles;
use cats_rbac::{Action, Resource, Role};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

pub async fn healthz() -> impl Responder {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok",
        service: "report-service",
    })
}

pub async fn readyz(pool: web::Data<PgPool>) -> impl Responder {
    #[derive(Serialize)]
    struct ReadyResponse {
        status: &'static str,
        service: &'static str,
        db: &'static str,
        task_db: &'static str,
    }
    let db_ok = sqlx::query(r#"SELECT 1"#).execute(pool.get_ref()).await.is_ok();
    HttpResponse::Ok().json(ReadyResponse {
        status: if db_ok { "ready" } else { "not_ready" },
        service: "report-service",
        db: if db_ok { "ok" } else { "fail" },
        task_db: "unknown",
    })
}

#[derive(Deserialize)]
pub struct UsageQuery {
    pub org_id: Uuid,
    /// ISO-8601 时间, 默认 30 天前
    #[serde(default)]
    pub from: Option<DateTime<Utc>>,
    /// ISO-8601 时间, 默认 now()
    #[serde(default)]
    pub to: Option<DateTime<Utc>>,
}

pub async fn get_usage_report(
    state: web::Data<AppState>,
    _req: HttpRequest,
    q: web::Query<UsageQuery>,
) -> impl Responder {
    if let Err(e) = require_roles(
        &state.checker,
        &[Role::Sponsor, Role::DatabaseLead, Role::QualityLead, Role::ProjectLead],
        Resource::Report,
        Action::Read,
    )
    .await
    {
        return cats_error_to_response(&e);
    }

    let now = Utc::now();
    let to = q.to.unwrap_or(now);
    let from = q.from.unwrap_or_else(|| now - chrono::Duration::days(30));

    let task_pool = match &state.pools.task_ro {
        Some(p) => p,
        None => {
            return cats_error_to_response(&CatsError::internal(
                "task_db not configured (TASK_DATABASE_URL env missing)",
                "no task_ro pool",
            ))
        }
    };

    match db::usage_report(task_pool, q.org_id, from, to).await {
        Ok(r) => HttpResponse::Ok().json(r),
        Err(e) => cats_error_to_response(&e),
    }
}

/// MVP 占位: 用 mock 数据返回 quality report (无真实 task_media_items 数据)
pub async fn get_quality_report(
    state: web::Data<AppState>,
    _req: HttpRequest,
    q: web::Query<UsageQuery>,
) -> impl Responder {
    if let Err(e) = require_roles(
        &state.checker,
        &[Role::Sponsor, Role::DatabaseLead, Role::QualityLead],
        Resource::Report,
        Action::Read,
    )
    .await
    {
        return cats_error_to_response(&e);
    }
    // 返回占位结构
    let r: UsageReport = UsageReport {
        org_id: q.org_id,
        period_start: q.from.unwrap_or_else(|| Utc::now() - chrono::Duration::days(30)),
        period_end: q.to.unwrap_or_else(Utc::now),
        total_tasks: 0,
        completed_tasks: 0,
        qa_blocked_tasks: 0,
        in_progress_tasks: 0,
        pending_tasks: 0,
        failed_tasks: 0,
    };
    HttpResponse::Ok().json(r)
}