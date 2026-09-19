//! HTTP handlers (task-service)
//!
//! 端点 (per 接口设计 §6.4.1):
//! - GET    /healthz
//! - POST   /v1/tasks                       — 创建任务
//! - GET    /v1/tasks/{id}                  — 查询
//! - PATCH  /v1/tasks/{id}/status           — 状态机迁移 (per §6.4.5)
//! - GET    /v1/tasks?org_id=...           — 列任务 (分页)
//! - GET    /readyz

use crate::db;
use crate::models::{
    CreateTaskRequest, TaskListResponse, TaskResponse, TaskStatus, UpdateTaskStatusRequest,
};
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
        service: "task-service",
    })
}

pub async fn readyz(pool: web::Data<PgPool>) -> impl Responder {
    #[derive(Serialize)]
    struct ReadyResponse {
        status: &'static str,
        service: &'static str,
        db: &'static str,
    }
    match sqlx::query(r#"SELECT 1"#).execute(pool.get_ref()).await {
        Ok(_) => HttpResponse::Ok().json(ReadyResponse {
            status: "ready",
            service: "task-service",
            db: "ok",
        }),
        Err(_) => HttpResponse::ServiceUnavailable().json(ReadyResponse {
            status: "not_ready",
            service: "task-service",
            db: "fail",
        }),
    }
}

pub async fn create_task(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CreateTaskRequest>,
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
            Role::RustLead,
            Role::ProjectLead,
            Role::User,
        ],
        Resource::Task,
        Action::Create,
    )
    .await
    {
        return cats_error_to_response(&e);
    }
    let req_body = body.into_inner();
    if req_body.title.trim().is_empty() || req_body.title.len() > 200 {
        return cats_error_to_response(&CatsError::business(
            ErrorCode::InvalidRequest,
            "title must be 1..=200 chars",
        ));
    }
    match db::create(
        &state.pool,
        req_body.project_id,
        req_body.org_id,
        &req_body.title,
        req_body.description.as_deref(),
        &req_body.media_type,
        req_body.source_lang.as_deref(),
        req_body.target_lang.as_deref(),
        req_body.assigned_to,
        user_id,
    )
    .await
    {
        Ok(row) => {
            // outbox event (per §2 Outbox + 接口设计 §6.4.5)
            let _ = db::enqueue_outbox(
                &state.pool,
                row.id,
                "task.created",
                serde_json::json!({"task_id": row.id, "project_id": row.project_id, "status": row.status}),
            )
            .await;
            HttpResponse::Created().json(TaskResponse::from(row))
        }
        Err(e) => cats_error_to_response(&e),
    }
}

pub async fn get_task(
    state: web::Data<AppState>,
    _req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    if let Err(e) = require_roles(
        &state.checker,
        &[
            Role::Sponsor,
            Role::ArchitectLead,
            Role::RustLead,
            Role::ProjectLead,
            Role::DatabaseLead,
            Role::QualityLead,
            Role::User,
        ],
        Resource::Task,
        Action::Read,
    )
    .await
    {
        return cats_error_to_response(&e);
    }
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => {
            return cats_error_to_response(&CatsError::business(
                ErrorCode::InvalidRequest,
                "id must be a valid UUID",
            ))
        }
    };
    match db::find_by_id(&state.pool, id).await {
        Ok(Some(t)) => HttpResponse::Ok().json(TaskResponse::from(t)),
        Ok(None) => cats_error_to_response(&CatsError::not_found(
            ErrorCode::ResourceNotFound,
            format!("task {id}"),
        )),
        Err(e) => cats_error_to_response(&e),
    }
}

pub async fn update_task_status(
    state: web::Data<AppState>,
    _req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<UpdateTaskStatusRequest>,
) -> impl Responder {
    if let Err(e) = require_roles(
        &state.checker,
        &[
            Role::Sponsor,
            Role::ArchitectLead,
            Role::RustLead,
            Role::ProjectLead,
            Role::User,
        ],
        Resource::Task,
        Action::Update,
    )
    .await
    {
        return cats_error_to_response(&e);
    }
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(u) => u,
        Err(_) => {
            return cats_error_to_response(&CatsError::business(
                ErrorCode::InvalidRequest,
                "id must be a valid UUID",
            ))
        }
    };
    let req_body = body.into_inner();
    let new_status = match TaskStatus::parse(&req_body.status) {
        Some(s) => s,
        None => {
            return cats_error_to_response(&CatsError::business(
                ErrorCode::InvalidRequest,
                format!("unknown status: {}", req_body.status),
            ))
        }
    };

    // 状态机校验 (per §6.4.5)
    let current = match db::find_by_id(&state.pool, id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return cats_error_to_response(&CatsError::not_found(
                ErrorCode::ResourceNotFound,
                format!("task {id}"),
            ))
        }
        Err(e) => return cats_error_to_response(&e),
    };
    let cur_status = match TaskStatus::parse(&current.status) {
        Some(s) => s,
        None => {
            return cats_error_to_response(&CatsError::internal(
                "task status in db is corrupted",
                format!("unknown status {}", current.status),
            ))
        }
    };
    if !cur_status.can_transition_to(&new_status) {
        return cats_error_to_response(&CatsError::business(
            ErrorCode::Conflict,
            format!(
                "invalid status transition: {} → {}",
                cur_status.as_str(),
                new_status.as_str()
            ),
        ));
    }

    match db::update_status(&state.pool, id, new_status.as_str(), req_body.qa_violations).await {
        Ok(Some(t)) => {
            let _ = db::enqueue_outbox(
                &state.pool,
                id,
                "task.status_changed",
                serde_json::json!({"task_id": id, "from": cur_status.as_str(), "to": new_status.as_str()}),
            )
            .await;
            HttpResponse::Ok().json(TaskResponse::from(t))
        }
        Ok(None) => cats_error_to_response(&CatsError::not_found(
            ErrorCode::ResourceNotFound,
            format!("task {id}"),
        )),
        Err(e) => cats_error_to_response(&e),
    }
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub org_id: Uuid,
    #[serde(default)]
    pub project_id: Option<Uuid>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

pub async fn list_tasks(
    state: web::Data<AppState>,
    _req: HttpRequest,
    q: web::Query<ListQuery>,
) -> impl Responder {
    if let Err(e) = require_roles(
        &state.checker,
        &[
            Role::Sponsor,
            Role::ArchitectLead,
            Role::RustLead,
            Role::ProjectLead,
            Role::DatabaseLead,
            Role::QualityLead,
            Role::User,
        ],
        Resource::Task,
        Action::Read,
    )
    .await
    {
        return cats_error_to_response(&e);
    }
    let limit = q.limit.clamp(1, 100);
    let offset = q.offset.max(0);
    let rows = match db::list(&state.pool, q.org_id, q.project_id, q.status.as_deref(), limit, offset).await {
        Ok(r) => r,
        Err(e) => return cats_error_to_response(&e),
    };
    let total = match db::count(&state.pool, q.org_id, q.project_id, q.status.as_deref()).await {
        Ok(t) => t,
        Err(e) => return cats_error_to_response(&e),
    };
    HttpResponse::Ok().json(TaskListResponse {
        items: rows.into_iter().map(TaskResponse::from).collect(),
        total,
        limit,
        offset,
    })
}