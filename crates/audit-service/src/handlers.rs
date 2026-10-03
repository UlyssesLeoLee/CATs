//! HTTP handlers (audit-service)

use crate::db;
use crate::models::{AuditLogListResponse, AuditLogResponse, KafkaAuditEvent};
use crate::state::AppState;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use cats_common::{cats_error_to_response, CatsError, ErrorCode};
use cats_rbac::service_helpers::{extract_user_id_and_roles, require_roles};
use cats_rbac::{Action, Resource, Role};
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
        service: "audit-service",
    })
}

pub async fn readyz(pool: web::Data<PgPool>) -> impl Responder {
    #[derive(Serialize)]
    struct ReadyResponse {
        status: &'static str,
        service: &'static str,
        db: &'static str,
    }
    let db_ok = sqlx::query(r#"SELECT 1"#).execute(pool.get_ref()).await.is_ok();
    HttpResponse::Ok().json(ReadyResponse {
        status: if db_ok { "ready" } else { "not_ready" },
        service: "audit-service",
        db: if db_ok { "ok" } else { "fail" },
    })
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub org_id: Uuid,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

pub async fn list_audit_logs(
    state: web::Data<AppState>,
    req: HttpRequest,
    q: web::Query<ListQuery>,
) -> impl Responder {
    let (user_id, _roles) = match extract_user_id_and_roles(&req) {
        Ok(v) => v,
        Err(e) => return cats_error_to_response(&e),
    };
    if let Err(e) = require_roles(
        &state.checker,
        &[
            Role::Sponsor,
            Role::ArchitectLead,
            Role::DatabaseLead,
            Role::QualityLead,
            Role::ReviewLead,
            Role::PlatformLead,
        ],
        Resource::Audit,
        Action::Read,
    )
    .await
    {
        return cats_error_to_response(&e);
    }
    tracing::info!(user_id = %user_id, org_id = %q.org_id, "list audit logs");

    let limit = q.limit.clamp(1, 200);
    let offset = q.offset.max(0);
    let rows = match db::list_by_org(&state.pool, q.org_id, limit, offset).await {
        Ok(r) => r,
        Err(e) => return cats_error_to_response(&e),
    };
    let total = match db::count_by_org(&state.pool, q.org_id).await {
        Ok(t) => t,
        Err(e) => return cats_error_to_response(&e),
    };
    HttpResponse::Ok().json(AuditLogListResponse {
        items: rows.into_iter().map(AuditLogResponse::from).collect(),
        total,
        limit,
        offset,
    })
}

/// POST /v1/audit-logs/test — 手动 ingest 一条 (开发/测试用)
pub async fn test_ingest(
    state: web::Data<AppState>,
    _req: HttpRequest,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    let ev = KafkaAuditEvent {
        event_id: Uuid::new_v4(),
        org_id: Uuid::new_v4(),
        actor_user_id: None,
        action: "test.event".to_string(),
        resource_type: "test".to_string(),
        resource_id: "test-0".to_string(),
        before_state: None,
        after_state: Some(body.into_inner()),
        ip: None,
        occurred_at: chrono::Utc::now(),
        schema_version: 1,
    };
    match db::insert_event(&state.pool, &ev).await {
        Ok(id) => HttpResponse::Ok().json(serde_json::json!({"audit_log_id": id})),
        Err(e) => cats_error_to_response(&e),
    }
}