//! HTTP handlers (notification-service)

use crate::db;
use crate::models::{NotificationListResponse, NotificationResponse};
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
        service: "notification-service",
    })
}

pub async fn readyz(pool: web::Data<PgPool>) -> impl Responder {
    #[derive(Serialize)]
    struct ReadyResponse {
        status: &'static str,
        service: &'static str,
        db: &'static str,
        consumer: &'static str,
    }
    let db_ok = sqlx::query(r#"SELECT 1"#).execute(pool.get_ref()).await.is_ok();
    HttpResponse::Ok().json(ReadyResponse {
        status: if db_ok { "ready" } else { "not_ready" },
        service: "notification-service",
        db: if db_ok { "ok" } else { "fail" },
        consumer: "stub",
    })
}

#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

pub async fn list_my_notifications(
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
        &[Role::Sponsor, Role::User, Role::DatabaseLead, Role::QualityLead],
        Resource::User, // 用 User 资源因为是查"我自己的通知"
        Action::Read,
    )
    .await
    {
        return cats_error_to_response(&e);
    }
    let limit = q.limit.clamp(1, 100);
    let offset = q.offset.max(0);
    let rows = match db::list_by_user(&state.pool, user_id, limit, offset).await {
        Ok(r) => r,
        Err(e) => return cats_error_to_response(&e),
    };
    let total = match db::count_by_user(&state.pool, user_id).await {
        Ok(t) => t,
        Err(e) => return cats_error_to_response(&e),
    };
    HttpResponse::Ok().json(NotificationListResponse {
        items: rows.into_iter().map(NotificationResponse::from).collect(),
        total,
        limit,
        offset,
    })
}

/// POST /v1/notifications/test — 测试用, 手动触发 process_event
pub async fn test_publish(
    state: web::Data<AppState>,
    _req: HttpRequest,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    let ev = crate::models::KafkaEvent {
        event_id: Uuid::new_v4(),
        org_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        event_type: "test.event".to_string(),
        payload: body.into_inner(),
    };
    let bytes = match serde_json::to_vec(&ev) {
        Ok(b) => b,
        Err(e) => return cats_error_to_response(&CatsError::from(e)),
    };
    match crate::consumer::process_event(&state.pool, &bytes).await {
        Ok(id) => HttpResponse::Ok().json(serde_json::json!({"notification_id": id})),
        Err(e) => cats_error_to_response(&e),
    }
}