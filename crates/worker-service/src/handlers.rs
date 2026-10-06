//! worker-service HTTP handlers

use crate::state::AppState;
use actix_web::{web, HttpResponse, Responder};
use cats_common::error::cats_error_to_response;
use cats_rbac::service_helpers::{extract_user_id_and_roles, require_roles};
use cats_rbac::{Action, Resource, Role};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    app: cats_common::AppMeta,
}

pub async fn healthz() -> impl Responder {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok",
        app: cats_common::AppMeta {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    })
}

pub async fn readyz(pool: web::Data<PgPool>) -> impl Responder {
    #[derive(Serialize)]
    struct ReadyResponse {
        status: &'static str,
        service: &'static str,
        db: &'static str,
    }
    let db_ok = sqlx::query(r#"SELECT 1"#)
        .execute(pool.get_ref())
        .await
        .is_ok();
    HttpResponse::Ok().json(ReadyResponse {
        status: if db_ok { "ready" } else { "not_ready" },
        service: "worker-service",
        db: if db_ok { "ok" } else { "fail" },
    })
}

/// POST /v1/worker/tick — 手动触发一次 tick (开发/测试用)
pub async fn manual_tick(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let (_user_id, _roles) = match extract_user_id_and_roles(&req) {
        Ok(v) => v,
        Err(e) => return cats_error_to_response(&e),
    };
    if let Err(e) = require_roles(
        &state.checker,
        &[Role::Sponsor, Role::RustLead, Role::SRELead],
        Resource::Task,
        Action::Update,
    )
    .await
    {
        return cats_error_to_response(&e);
    }
    match crate::scheduler::tick(&state.pool).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({"status": "ticked"})),
        Err(e) => cats_error_to_response(&e),
    }
}
