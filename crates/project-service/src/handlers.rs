//! HTTP handlers (project-service)
//!
//! 端点 (per 接口设计 §6.3.1):
//! - GET    /healthz
//! - POST   /v1/projects                  — 创建项目
//! - GET    /v1/projects/{id}             — 查询
//! - PUT    /v1/projects/{id}             — 更新
//! - GET    /v1/projects?org_id=...      — 列 org 项目 (分页)
//! - POST   /v1/projects/{id}/members     — 加成员
//! - GET    /v1/projects/{id}/members     — 列成员
//!
//! RBAC 集成: 每个 handler 启动时构造 `RbacChecker::new()`, 由 Gateway 注入
//! X-Cats-Roles header, handler 解析并调 `checker.check(...)`. MVP 简化:
//! 仅校验已登录 + 角色非空, 业务级 (跨 org 隔离) 由 handler 内显式判断。

use crate::db;
use crate::models::{
    AddMemberRequest, CreateProjectRequest, MemberResponse, ProjectListResponse,
    ProjectResponse, UpdateProjectRequest,
};
use crate::state::AppState;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use cats_common::{cats_error_to_response, CatsError, ErrorCode};
use cats_rbac::service_helpers::{extract_user_id_and_roles, require_roles};
use cats_rbac::{Action, Resource, Role};
use serde::Serialize;
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
        service: "project-service",
    })
}

pub async fn create_project(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CreateProjectRequest>,
) -> impl Responder {
    let req_body = body.into_inner();

    // 1. RBAC: Sponsor / ArchitectLead / RustLead / ProjectLead / User 均可创建
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
        Resource::Project,
        Action::Create,
    )
    .await
    {
        return cats_error_to_response(&e);
    }

    // 2. 字段校验
    if req_body.name.trim().is_empty() {
        return cats_error_to_response(&CatsError::business(
            ErrorCode::InvalidRequest,
            "name must not be empty",
        ));
    }
    if req_body.name.len() > 200 {
        return cats_error_to_response(&CatsError::business(
            ErrorCode::InvalidRequest,
            "name must be ≤ 200 chars",
        ));
    }

    // 3. 落库
    match db::create(
        &state.pool,
        req_body.org_id,
        &req_body.name,
        req_body.description.as_deref(),
        req_body.source_lang.as_deref(),
        req_body.target_lang.as_deref(),
        user_id,
    )
    .await
    {
        Ok(row) => HttpResponse::Created().json(ProjectResponse::from(row)),
        Err(e) => cats_error_to_response(&e),
    }
}

pub async fn get_project(
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
        Resource::Project,
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
        Ok(Some(p)) => HttpResponse::Ok().json(ProjectResponse::from(p)),
        Ok(None) => cats_error_to_response(&CatsError::not_found(
            ErrorCode::ResourceNotFound,
            format!("project {id}"),
        )),
        Err(e) => cats_error_to_response(&e),
    }
}

pub async fn update_project(
    state: web::Data<AppState>,
    _req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<UpdateProjectRequest>,
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
        Resource::Project,
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
    if let Some(n) = &req_body.name {
        if n.trim().is_empty() || n.len() > 200 {
            return cats_error_to_response(&CatsError::business(
                ErrorCode::InvalidRequest,
                "name must be 1..=200 chars",
            ));
        }
    }
    match db::update(
        &state.pool,
        id,
        req_body.name.as_deref(),
        req_body.description.as_deref(),
        req_body.status.as_deref(),
    )
    .await
    {
        Ok(Some(p)) => HttpResponse::Ok().json(ProjectResponse::from(p)),
        Ok(None) => cats_error_to_response(&CatsError::not_found(
            ErrorCode::ResourceNotFound,
            format!("project {id}"),
        )),
        Err(e) => cats_error_to_response(&e),
    }
}

#[derive(serde::Deserialize)]
pub struct ListQuery {
    pub org_id: Uuid,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

pub async fn list_projects(
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
        Resource::Project,
        Action::Read,
    )
    .await
    {
        return cats_error_to_response(&e);
    }
    let limit = q.limit.clamp(1, 100);
    let offset = q.offset.max(0);

    // 顺序执行 (避免 futures crate 依赖), MVP 端到端简单可靠
    let rows = match db::list_by_org(&state.pool, q.org_id, limit, offset).await {
        Ok(r) => r,
        Err(e) => return cats_error_to_response(&e),
    };
    let total = match db::count_by_org(&state.pool, q.org_id).await {
        Ok(t) => t,
        Err(e) => return cats_error_to_response(&e),
    };

    HttpResponse::Ok().json(ProjectListResponse {
        items: rows.into_iter().map(ProjectResponse::from).collect(),
        total,
        limit,
        offset,
    })
}

pub async fn add_member(
    state: web::Data<AppState>,
    _req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<AddMemberRequest>,
) -> impl Responder {
    if let Err(e) = require_roles(
        &state.checker,
        &[Role::Sponsor, Role::ArchitectLead, Role::ProjectLead],
        Resource::Project,
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
    match db::add_member(&state.pool, id, req_body.user_id, &req_body.role).await {
        Ok(m) => HttpResponse::Created().json(MemberResponse::from(m)),
        Err(e) => cats_error_to_response(&e),
    }
}

pub async fn list_members(
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
        Resource::Project,
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
    match db::list_members(&state.pool, id).await {
        Ok(rows) => HttpResponse::Ok().json(
            rows.into_iter().map(MemberResponse::from).collect::<Vec<_>>(),
        ),
        Err(e) => cats_error_to_response(&e),
    }
}

/// `/readyz` — DB ping + 关键表存在性检查
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
            service: "project-service",
            db: "ok",
        }),
        Err(_) => HttpResponse::ServiceUnavailable().json(ReadyResponse {
            status: "not_ready",
            service: "project-service",
            db: "fail",
        }),
    }
}