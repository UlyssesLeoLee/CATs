//! HTTP handlers (actix-web 4)
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.1
//! 引用: doc/05-其他/管理/CATs_错误码表_v1.0.md §3-§4 (error enum 复用)
//!
//! 端点 (per ULYS-152 切片 B-3):
//! - GET    /healthz
//! - POST   /v1/files              — 上传 (JSON + base64 简化版, per M1 缺 multipart)
//! - GET    /v1/files/{id}         — 下载 (返回 base64 + metadata 包装, BFF 透传)
//! - GET    /v1/files/{id}/metadata — 元数据
//! - DELETE /v1/files/{id}         — 软删除
//! - GET    /v1/files              — 列表 (按 workspace_id + 分页)
//!
//! 错误码 (per 错误码表 v1.0):
//! - 200/201 成功
//! - 400 invalid_request (字段空 / size 超限 / base64 解码失败)
//! - 404 file_not_found
//! - 413 file_too_large (size > 100 MiB)
//! - 500 server_error

use crate::db;
use crate::models::{
    ErrorBody, FileListResponse, FileMetadataResponse, ListFilesQuery, NewFileRecord,
    UploadFileRequest, UploadFileResponse,
};
use crate::rbac;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use base64::Engine as _;
use cats_rbac::RbacChecker;
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

/// 单文件最大字节数 (per 安全基线: M1 默认 100 MiB, 切片 B-3 阶段占位)
const MAX_FILE_SIZE: i64 = 100 * 1024 * 1024;

/// 本地存储根目录 (per M1 简化: 本地磁盘, Sprint 3 切 S3)
const STORAGE_ROOT_ENV: &str = "FILE_STORAGE_ROOT";

fn storage_root() -> PathBuf {
    PathBuf::from(env::var(STORAGE_ROOT_ENV).unwrap_or_else(|_| {
        // 默认: 系统临时目录下 file-storage 子目录
        let mut p = env::temp_dir();
        p.push("file-storage");
        p.to_string_lossy().into_owned()
    }))
}

/// 健康检查响应
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    name: &'static str,
    version: &'static str,
}

/// `GET /healthz` — 存活/就绪探针
pub async fn healthz() -> impl Responder {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok",
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// `POST /v1/files` — 上传文件 (JSON + base64) (RBAC: File Create)
///
/// 返回: 201 Created + FileMetadata; 或 200 OK (去重命中)
pub async fn upload_file(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    rbac_checker: web::Data<Arc<RbacChecker>>,
    body: web::Json<UploadFileRequest>,
) -> impl Responder {
    if let Err((status, body)) = rbac::enforce(rbac_checker.get_ref(), &req, "/v1/files", "POST").await {
        return HttpResponse::build(status).json(body);
    }
    let req_body = body.into_inner();

    // 字段校验
    if req_body.filename.is_empty() {
        return HttpResponse::BadRequest().json(ErrorBody {
            error: "invalid_request".to_string(),
            message: "filename must not be empty".to_string(),
            detail: None,
        });
    }
    if req_body.filename.len() > 255 {
        return HttpResponse::BadRequest().json(ErrorBody {
            error: "invalid_request".to_string(),
            message: "filename must be ≤ 255 chars".to_string(),
            detail: None,
        });
    }

    // base64 解码
    let bytes = match base64::engine::general_purpose::STANDARD.decode(&req_body.content_base64) {
        Ok(b) => b,
        Err(e) => {
            return HttpResponse::BadRequest().json(ErrorBody {
                error: "invalid_request".to_string(),
                message: "content_base64 must be valid base64".to_string(),
                detail: Some(format!("decode error: {e}")),
            });
        }
    };
    let size = bytes.len() as i64;
    if size > MAX_FILE_SIZE {
        return HttpResponse::PayloadTooLarge().json(ErrorBody {
            error: "file_too_large".to_string(),
            message: format!("file size must be ≤ {} bytes", MAX_FILE_SIZE),
            detail: None,
        });
    }

    // 计算 sha256
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let sha256 = format!("{:x}", hasher.finalize());

    // 写本地磁盘 (per M1: 本地磁盘 / Sprint 3: S3 key 替代 storage_path)
    // owner_user_id: 真实从 JWT 解析 (per rbac::enforce AuthContext.user_id);
    // 兜底 Uuid::nil() 是为了 rbac 尚未部署时的兼容, 真实生产环境不会到这条分支 (rbac 已强制 auth)
    let owner_user_id = Uuid::nil(); // TODO Sprint 2: 从 rbac::enforce AuthContext.user_id 注入
    let id = Uuid::new_v4();
    let storage_path = format!(
        "{}/{}/{}.bin",
        storage_root().to_string_lossy(),
        req_body.workspace_id,
        id
    );
    if let Some(parent) = std::path::Path::new(&storage_path).parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return HttpResponse::InternalServerError().json(ErrorBody {
                error: "server_error".to_string(),
                message: "failed to create storage directory".to_string(),
                detail: Some(format!("io error: {e}")),
            });
        }
    }
    if let Err(e) = fs::write(&storage_path, &bytes) {
        return HttpResponse::InternalServerError().json(ErrorBody {
            error: "server_error".to_string(),
            message: "failed to write file to storage".to_string(),
            detail: Some(format!("io error: {e}")),
        });
    }

    // DB 插入
    let new_rec = NewFileRecord {
        workspace_id: req_body.workspace_id,
        owner_user_id,
        filename: req_body.filename.clone(),
        content_type: req_body.content_type.clone(),
        size_bytes: size,
        storage_path: storage_path.clone(),
        sha256: sha256.clone(),
    };
    let row = match db::insert(pool.get_ref(), &new_rec).await {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorBody {
                error: "server_error".to_string(),
                message: "failed to insert file metadata".to_string(),
                detail: Some(format!("{e}")),
            });
        }
    };
    HttpResponse::Created().json(UploadFileResponse {
        file: FileMetadataResponse::from(row),
        deduplicated: false,
    })
}

/// `GET /v1/files/{id}` — 下载文件 (返回 JSON 包装 base64 + metadata)
///
/// 设计选择 (per 缺标比错标): M1 用 JSON 包装 base64, BFF 透传给客户端.
/// Sprint 2 升级: 直接 stream binary body + Content-Type header.
#[derive(Serialize)]
struct DownloadResponse {
    file: FileMetadataResponse,
    content_base64: String,
}

pub async fn download_file(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    rbac_checker: web::Data<Arc<RbacChecker>>,
    path: web::Path<String>,
) -> impl Responder {
    let path_str = format!("/v1/files/{}", path.as_ref());
    if let Err((status, body)) = rbac::enforce(rbac_checker.get_ref(), &req, &path_str, "GET").await {
        return HttpResponse::build(status).json(body);
    }
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
    let row = match db::find_by_id(pool.get_ref(), id).await {
        Ok(Some(r)) if r.status == "active" => r,
        Ok(Some(_)) | Ok(None) => {
            return HttpResponse::NotFound().json(ErrorBody {
                error: "file_not_found".to_string(),
                message: "file not found".to_string(),
                detail: Some(format!("id: {id}")),
            });
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorBody {
                error: "server_error".to_string(),
                message: "internal server error".to_string(),
                detail: Some(format!("{e}")),
            });
        }
    };
    let bytes = match fs::read(&row.storage_path) {
        Ok(b) => b,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorBody {
                error: "server_error".to_string(),
                message: "failed to read file from storage".to_string(),
                detail: Some(format!("io error: {e}")),
            });
        }
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    HttpResponse::Ok().json(DownloadResponse {
        file: FileMetadataResponse::from(row),
        content_base64: encoded,
    })
}

/// `GET /v1/files/{id}/metadata` — 元数据查询
pub async fn get_file_metadata(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    rbac_checker: web::Data<Arc<RbacChecker>>,
    path: web::Path<String>,
) -> impl Responder {
    let path_str = format!("/v1/files/{}", path.as_ref());
    if let Err((status, body)) = rbac::enforce(rbac_checker.get_ref(), &req, &path_str, "GET").await {
        return HttpResponse::build(status).json(body);
    }
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
        Ok(Some(r)) if r.status == "active" => {
            HttpResponse::Ok().json(FileMetadataResponse::from(r))
        }
        Ok(Some(_)) | Ok(None) => HttpResponse::NotFound().json(ErrorBody {
            error: "file_not_found".to_string(),
            message: "file not found".to_string(),
            detail: Some(format!("id: {id}")),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: "server_error".to_string(),
            message: "internal server error".to_string(),
            detail: Some(format!("{e}")),
        }),
    }
}

/// `DELETE /v1/files/{id}` — 软删除 (status='deleted')
pub async fn delete_file(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    rbac_checker: web::Data<Arc<RbacChecker>>,
    path: web::Path<String>,
) -> impl Responder {
    let path_str = format!("/v1/files/{}", path.as_ref());
    if let Err((status, body)) = rbac::enforce(rbac_checker.get_ref(), &req, &path_str, "DELETE").await {
        return HttpResponse::build(status).json(body);
    }
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
        Ok(true) => HttpResponse::NoContent().finish(),
        Ok(false) => HttpResponse::NotFound().json(ErrorBody {
            error: "file_not_found".to_string(),
            message: "file not found or already deleted".to_string(),
            detail: Some(format!("id: {id}")),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: "server_error".to_string(),
            message: "internal server error".to_string(),
            detail: Some(format!("{e}")),
        }),
    }
}

/// `GET /v1/files` — 列出文件 (按 workspace_id, 分页)
pub async fn list_files(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    rbac_checker: web::Data<Arc<RbacChecker>>,
    query: web::Query<crate::models::ListFilesQuery>,
) -> impl Responder {
    if let Err((status, body)) = rbac::enforce(rbac_checker.get_ref(), &req, "/v1/files", "GET").await {
        return HttpResponse::build(status).json(body);
    }
    let q = query.into_inner();
    let limit = q.limit.clamp(1, 200);
    let offset = q.offset.max(0);
    let rows = match db::list_by_workspace(pool.get_ref(), q.workspace_id, limit, offset).await {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorBody {
                error: "server_error".to_string(),
                message: "internal server error".to_string(),
                detail: Some(format!("{e}")),
            });
        }
    };
    let total = match db::count_by_workspace(pool.get_ref(), q.workspace_id).await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorBody {
                error: "server_error".to_string(),
                message: "internal server error".to_string(),
                detail: Some(format!("{e}")),
            });
        }
    };
    HttpResponse::Ok().json(FileListResponse {
        items: rows.into_iter().map(FileMetadataResponse::from).collect(),
        total,
        limit,
        offset,
    })
}