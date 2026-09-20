//! HTTP handlers (actix-web 4)
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1 (project-service)
//! 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.2 (project_db 接口契约 v1.0.0)
//! 引用: doc/05-其他/管理/CATs_错误码表_v1.0.md §3-§4 (error enum 复用)
//! 引用: ULYS-150 切片 B-1 §"5 endpoint"
//!
//! 端点 (per 切片 B-1 §5 endpoint):
//! - GET    /healthz
//! - POST   /v1/projects                  — 创建
//! - GET    /v1/projects                  — 列出 (分页 + 过滤 by workspace_id)
//! - GET    /v1/projects/{id}             — 查询
//! - PATCH  /v1/projects/{id}             — 部分更新 (name/source_lang/target_lang/status)
//! - DELETE /v1/projects/{id}             — 软删除 (status='archived')
//!
//! 错误码 (per 错误码表 v1.0):
//! - 200 成功
//! - 201 创建
//! - 400 invalid_request (字段空 / UUID 解析失败 / 长度超限 / 不支持的状态值)
//! - 404 project_not_found / resource_not_found
//! - 500 server_error

use crate::db;
use crate::models::{
    CreateProjectRequest, ErrorBody, GetProjectResponse, ListProjectsItem, ListProjectsQuery,
    ListProjectsResponse, UpdateProjectRequest,
};
use actix_web::{web, HttpResponse, Responder};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

/// 健康检查响应
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    name: &'static str,
    version: &'static str,
}

/// `GET /healthz` — 存活探针 + 启动探针复用
pub async fn healthz() -> impl Responder {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok",
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
    })
}

// =====================================================================
// POST /v1/projects
// =====================================================================

/// `POST /v1/projects` — 创建项目
pub async fn create_project(
    pool: web::Data<PgPool>,
    body: web::Json<CreateProjectRequest>,
) -> impl Responder {
    let req = body.into_inner();

    // 字段校验 (per 错误码表 §3.2 invalid_request)
    let trimmed_name = req.name.trim();
    if trimmed_name.is_empty() {
        return HttpResponse::BadRequest().json(ErrorBody {
            error: "invalid_request".to_string(),
            message: "name must not be empty".to_string(),
            detail: None,
        });
    }
    if req.name.len() > 128 {
        return HttpResponse::BadRequest().json(ErrorBody {
            error: "invalid_request".to_string(),
            message: "name must be ≤ 128 chars".to_string(),
            detail: None,
        });
    }
    if req.source_lang.is_empty() || req.target_lang.is_empty() {
        return HttpResponse::BadRequest().json(ErrorBody {
            error: "invalid_request".to_string(),
            message: "source_lang and target_lang must not be empty".to_string(),
            detail: None,
        });
    }
    if req.source_lang.len() > 16 || req.target_lang.len() > 16 {
        return HttpResponse::BadRequest().json(ErrorBody {
            error: "invalid_request".to_string(),
            message: "lang codes must be ≤ 16 chars".to_string(),
            detail: None,
        });
    }

    match db::create(
        pool.get_ref(),
        req.workspace_id,
        &req.name,
        &req.source_lang,
        &req.target_lang,
        req.owner_user_id,
    )
    .await
    {
        Ok(p) => HttpResponse::Created().json(GetProjectResponse::from(p)),
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: "server_error".to_string(),
            message: "internal server error".to_string(),
            detail: Some(format!("{}", e)),
        }),
    }
}

// =====================================================================
// GET /v1/projects/{id}
// =====================================================================

/// `GET /v1/projects/{id}` — 查询项目 (按主键 id)
pub async fn get_project(pool: web::Data<PgPool>, path: web::Path<String>) -> impl Responder {
    let id_str = path.into_inner();
    let id = match Uuid::parse_str(&id_str) {
        Ok(u) => u,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorBody {
                error: "invalid_request".to_string(),
                message: "id must be a valid UUID".to_string(),
                detail: None,
            });
        }
    };
    match db::find_by_id(pool.get_ref(), id).await {
        Ok(Some(p)) => HttpResponse::Ok().json(GetProjectResponse::from(p)),
        Ok(None) => HttpResponse::NotFound().json(ErrorBody {
            error: "project_not_found".to_string(),
            message: "project not found".to_string(),
            detail: Some(format!("id: {id}")),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: "server_error".to_string(),
            message: "internal server error".to_string(),
            detail: Some(format!("{}", e)),
        }),
    }
}

// =====================================================================
// GET /v1/projects (列表)
// =====================================================================

/// `GET /v1/projects?workspace_id=&page=&page_size=` — 列出项目 (分页 + 过滤)
pub async fn list_projects(
    pool: web::Data<PgPool>,
    query: web::Query<ListProjectsQuery>,
) -> impl Responder {
    let q = query.into_inner();

    // 参数校验
    if q.page < 1 {
        return HttpResponse::BadRequest().json(ErrorBody {
            error: "invalid_request".to_string(),
            message: "page must be >= 1".to_string(),
            detail: None,
        });
    }
    if !(1..=100).contains(&q.page_size) {
        return HttpResponse::BadRequest().json(ErrorBody {
            error: "invalid_request".to_string(),
            message: "page_size must be in 1..=100".to_string(),
            detail: None,
        });
    }

    match db::list(pool.get_ref(), q.workspace_id, q.page, q.page_size).await {
        Ok((rows, total)) => {
            let items: Vec<ListProjectsItem> = rows.into_iter().map(Into::into).collect();
            HttpResponse::Ok().json(ListProjectsResponse {
                total,
                page: q.page,
                page_size: q.page_size,
                items,
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: "server_error".to_string(),
            message: "internal server error".to_string(),
            detail: Some(format!("{}", e)),
        }),
    }
}

// =====================================================================
// PATCH /v1/projects/{id}
// =====================================================================

/// `PATCH /v1/projects/{id}` — 部分更新
pub async fn patch_project(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
    body: web::Json<UpdateProjectRequest>,
) -> impl Responder {
    let id_str = path.into_inner();
    let id = match Uuid::parse_str(&id_str) {
        Ok(u) => u,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorBody {
                error: "invalid_request".to_string(),
                message: "id must be a valid UUID".to_string(),
                detail: None,
            });
        }
    };
    let req = body.into_inner();

    // 字段校验 (per 错误码表 §3.2 invalid_request)
    if let Some(n) = &req.name {
        let trimmed = n.trim();
        if trimmed.is_empty() {
            return HttpResponse::BadRequest().json(ErrorBody {
                error: "invalid_request".to_string(),
                message: "name must not be empty if provided".to_string(),
                detail: None,
            });
        }
        if n.len() > 128 {
            return HttpResponse::BadRequest().json(ErrorBody {
                error: "invalid_request".to_string(),
                message: "name must be ≤ 128 chars".to_string(),
                detail: None,
            });
        }
    }
    if let Some(s) = &req.source_lang {
        if s.is_empty() || s.len() > 16 {
            return HttpResponse::BadRequest().json(ErrorBody {
                error: "invalid_request".to_string(),
                message: "source_lang must be 1..=16 chars".to_string(),
                detail: None,
            });
        }
    }
    if let Some(t) = &req.target_lang {
        if t.is_empty() || t.len() > 16 {
            return HttpResponse::BadRequest().json(ErrorBody {
                error: "invalid_request".to_string(),
                message: "target_lang must be 1..=16 chars".to_string(),
                detail: None,
            });
        }
    }

    match db::update(
        pool.get_ref(),
        id,
        req.name.as_deref(),
        req.source_lang.as_deref(),
        req.target_lang.as_deref(),
        req.status,
    )
    .await
    {
        Ok(Some(p)) => HttpResponse::Ok().json(GetProjectResponse::from(p)),
        Ok(None) => HttpResponse::NotFound().json(ErrorBody {
            error: "project_not_found".to_string(),
            message: "project not found".to_string(),
            detail: Some(format!("id: {id}")),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: "server_error".to_string(),
            message: "internal server error".to_string(),
            detail: Some(format!("{}", e)),
        }),
    }
}

// =====================================================================
// DELETE /v1/projects/{id}
// =====================================================================

/// `DELETE /v1/projects/{id}` — 软删除 (status='archived')
///
/// 返回 200 + 新状态 Project (per 接口设计书 §/projects DELETE 惯例)
pub async fn delete_project(pool: web::Data<PgPool>, path: web::Path<String>) -> impl Responder {
    let id_str = path.into_inner();
    let id = match Uuid::parse_str(&id_str) {
        Ok(u) => u,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorBody {
                error: "invalid_request".to_string(),
                message: "id must be a valid UUID".to_string(),
                detail: None,
            });
        }
    };
    match db::soft_delete(pool.get_ref(), id).await {
        Ok(Some(p)) => HttpResponse::Ok().json(GetProjectResponse::from(p)),
        Ok(None) => HttpResponse::NotFound().json(ErrorBody {
            error: "project_not_found".to_string(),
            message: "project not found".to_string(),
            detail: Some(format!("id: {id}")),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: "server_error".to_string(),
            message: "internal server error".to_string(),
            detail: Some(format!("{}", e)),
        }),
    }
}

// =====================================================================
// 测试辅助
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_create_project(name: &str) -> CreateProjectRequest {
        CreateProjectRequest {
            workspace_id: Uuid::new_v4(),
            name: name.to_string(),
            source_lang: "ja".to_string(),
            target_lang: "en".to_string(),
            owner_user_id: Uuid::new_v4(),
        }
    }

    #[test]
    fn create_request_name_validation() {
        let empty = make_create_project("");
        assert!(empty.name.trim().is_empty());
        let long = make_create_project(&"a".repeat(129));
        assert!(long.name.len() > 128);
    }
}