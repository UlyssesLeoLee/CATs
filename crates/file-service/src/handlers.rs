//! HTTP handlers (file-service)
//!
//! MVP 端点 (per brief §1):
//! - GET    /healthz
//! - GET    /readyz
//! - POST   /v1/files                — 接收上传 (multipart/form-data)
//!   MVP 简化: 用 `application/octet-stream` 接收 bytes + header 携带 metadata (org_id, project_id, task_id, name, content_type)
//!   避免 actix-multipart 复杂度 (per 守门 #11 缺标比错标, 文档实装阶段再细化)
//! - GET    /v1/files/{id}/content  — 返回 metadata + storage_path (实际下载字节 MVP 不实装, 留 Sprint 2)

use crate::db;
use crate::models::{FileContentResponse, FileResponse};
use crate::state::AppState;
use actix_web::http::header::{HeaderMap, HeaderValue};
use actix_web::web::Bytes;
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
        service: "file-service",
    })
}

pub async fn readyz(pool: web::Data<PgPool>) -> impl Responder {
    #[derive(Serialize)]
    struct ReadyResponse {
        status: &'static str,
        service: &'static str,
        db: &'static str,
        storage: &'static str,
    }
    let db_ok = sqlx::query(r#"SELECT 1"#).execute(pool.get_ref()).await.is_ok();
    let storage_ok = crate::storage::ensure_base_dir().await.is_ok();
    HttpResponse::Ok().json(ReadyResponse {
        status: if db_ok && storage_ok { "ready" } else { "not_ready" },
        service: "file-service",
        db: if db_ok { "ok" } else { "fail" },
        storage: if storage_ok { "ok" } else { "fail" },
    })
}

/// POST /v1/files — multipart 简化版: bytes in body, metadata in headers
///
/// Headers:
/// - X-Cats-Org-Id: UUID (required)
/// - X-Cats-Project-Id: UUID (optional)
/// - X-Cats-Task-Id: UUID (optional)
/// - X-Cats-File-Name: string (required)
/// - X-Cats-Content-Type: string (optional)
/// - X-Cats-Roles + X-Cats-User-Id: per §1.2
pub async fn upload_file(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: Bytes,
) -> impl Responder {
    let (user_id, _roles) = match extract_user_id_and_roles(&req) {
        Ok(v) => v,
        Err(e) => return cats_error_to_response(&e),
    };
    if let Err(e) = require_roles(
        &state.checker,
        &[Role::Sponsor, Role::User, Role::ArchitectLead, Role::RustLead],
        Resource::File,
        Action::Create,
    )
    .await
    {
        return cats_error_to_response(&e);
    }

    let headers = req.headers();
    let org_id = match parse_uuid_header(headers, "X-Cats-Org-Id") {
        Ok(u) => u,
        Err(e) => return cats_error_to_response(&e),
    };
    let project_id = parse_optional_uuid_header(headers, "X-Cats-Project-Id");
    let task_id = parse_optional_uuid_header(headers, "X-Cats-Task-Id");
    let file_name = match headers.get("X-Cats-File-Name").and_then(|h| h.to_str().ok()) {
        Some(s) if !s.is_empty() && s.len() <= 256 => s.to_string(),
        _ => {
            return cats_error_to_response(&CatsError::business(
                ErrorCode::InvalidRequest,
                "X-Cats-File-Name header required (1..=256 chars)",
            ))
        }
    };
    let content_type = headers
        .get("X-Cats-Content-Type")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    if body.is_empty() {
        return cats_error_to_response(&CatsError::business(
            ErrorCode::InvalidRequest,
            "empty body",
        ));
    }
    let size_bytes = body.len() as i64;

    // sha256
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&body);
    let sha256 = format!("{:x}", hasher.finalize());

    // 本地落盘
    let file_id = Uuid::new_v4();
    let storage_path = match crate::storage::write_file(org_id, file_id, &file_name, &body).await {
        Ok(p) => p,
        Err(e) => return cats_error_to_response(&e),
    };

    match db::create(
        &state.pool,
        org_id,
        project_id,
        task_id,
        &file_name,
        content_type.as_deref(),
        size_bytes,
        Some(&sha256),
        &storage_path,
        user_id,
    )
    .await
    {
        Ok(row) => HttpResponse::Created().json(FileResponse::from(row)),
        Err(e) => cats_error_to_response(&e),
    }
}

pub async fn get_file_content(
    state: web::Data<AppState>,
    _req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    if let Err(e) = require_roles(
        &state.checker,
        &[Role::Sponsor, Role::User, Role::DatabaseLead, Role::QualityLead],
        Resource::File,
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
        Ok(Some(f)) => HttpResponse::Ok().json(FileContentResponse {
            id: f.id,
            name: f.name,
            content_type: f.content_type,
            size_bytes: f.size_bytes,
            sha256: f.sha256,
            storage_path: f.storage_path,
            status: f.status,
            note: "MVP 不实装字节下载; Sprint 2 加 actix-files::NamedFile 或 MinIO 集成",
        }),
        Ok(None) => cats_error_to_response(&CatsError::not_found(
            ErrorCode::ResourceNotFound,
            format!("file {id}"),
        )),
        Err(e) => cats_error_to_response(&e),
    }
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub org_id: Uuid,
    #[serde(default)]
    pub project_id: Option<Uuid>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

pub async fn list_files(
    state: web::Data<AppState>,
    _req: HttpRequest,
    q: web::Query<ListQuery>,
) -> impl Responder {
    if let Err(e) = require_roles(
        &state.checker,
        &[Role::Sponsor, Role::User, Role::DatabaseLead, Role::QualityLead],
        Resource::File,
        Action::Read,
    )
    .await
    {
        return cats_error_to_response(&e);
    }
    let limit = q.limit.clamp(1, 100);
    let offset = q.offset.max(0);
    let rows = match db::list(&state.pool, q.org_id, q.project_id, limit, offset).await {
        Ok(r) => r,
        Err(e) => return cats_error_to_response(&e),
    };
    HttpResponse::Ok().json(
        rows.into_iter()
            .map(FileResponse::from)
            .collect::<Vec<_>>(),
    )
}

fn parse_uuid_header(headers: &HeaderMap, name: &str) -> Result<Uuid, CatsError> {
    let raw = headers
        .get(name)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            CatsError::business(
                ErrorCode::InvalidRequest,
                format!("missing {name} header"),
            )
        })?;
    Uuid::parse_str(raw).map_err(|_| {
        CatsError::business(
            ErrorCode::InvalidRequest,
            format!("{name} is not a valid UUID"),
        )
    })
}

fn parse_optional_uuid_header(headers: &HeaderMap, name: &str) -> Option<Uuid> {
    headers
        .get(name)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
}

// 抑制 unused warning (HeaderValue 保留 for future use)
#[allow(dead_code)]
fn _check_header_value(v: &HeaderValue) -> bool {
    !v.is_empty()
}