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
    ErrorBody, FileListResponse, FileMetadataResponse, NewFileRecord, UploadFileRequest,
    UploadFileResponse,
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
    app: cats_common::AppMeta,
}

/// `GET /healthz` — 存活/就绪探针
///
/// 响应形状 = 全仓统一后的唯一形状（per BACKEND_STATUS §4.1m）：
/// `{"status":"ok","app":{"name":...,"version":...}}`
///
/// 用 `env!("CARGO_PKG_NAME")` 而不是 `AppMeta::current()` —— 后者返回的是
/// **cats-common 自己**的包名，会让每个服务都自报 "cats-common"，监控分不出
/// 是谁应答的。这个坑见 asr-service/src/main.rs 里的同款注释。
pub async fn healthz() -> impl Responder {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok",
        app: cats_common::AppMeta {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
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
    let auth = match rbac::enforce(rbac_checker.get_ref(), &req, "/v1/files", "POST").await {
        Ok(a) => a,
        Err((status, body)) => return HttpResponse::build(status).json(body),
    };
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
    // owner_user_id: 真实从 JWT 解析 (per rbac::enforce AuthContext.user_id)
    // rbac::enforce 已强制 auth (401 missing_authorization), 所以 auth.user_id 兜底 nil 永远不会触发
    let owner_user_id = auth.user_id.unwrap_or_else(Uuid::nil);
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
    if let Err((status, body)) = rbac::enforce(rbac_checker.get_ref(), &req, &path_str, "GET").await
    {
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
    if let Err((status, body)) = rbac::enforce(rbac_checker.get_ref(), &req, &path_str, "GET").await
    {
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
    if let Err((status, body)) =
        rbac::enforce(rbac_checker.get_ref(), &req, &path_str, "DELETE").await
    {
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
    if let Err((status, body)) =
        rbac::enforce(rbac_checker.get_ref(), &req, "/v1/files", "GET").await
    {
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

// =====================================================================
// **唯一路由表** —— `main.rs` 与集成测试都从这里接进来
// =====================================================================
//
// 2026-10-07。这 6 条原本内联在 `main.rs`，而 `tests/integration.rs` 的
// `make_app` 逐条抄了一份（6 = 6，逐条核对完全一致，所以改之前不是假绿）。
// 与 auth / task / user / project 一样收敛到单一事实来源。
//
// 测试漏注册 RBAC 那个 app_data 时，actix extractor 取不到，表现为
// 500 "Requested application data is not configured correctly"，而 `/healthz`
// 不吃这个 extractor —— 于是表现成"healthz 过、其余全挂"，极易误判成
// RBAC 逻辑坏了。收进本函数后这类"忘了注册"只剩一种写法。
//
// `main.rs` 的 HttpServer 闭包是 `Fn`（每个 worker 线程各调一次），
// 要**先 clone 再 move**（`web::Data` 内封 Arc，clone 廉价）。

/// `GET /readyz` — 就绪探针，**必须**在 DB 不可用时返回 503
///
/// 2026-10-07 新增。此前 k3s 的 `deploy/k3s/cats-core/file-service.yaml` 把
/// readinessProbe 指向了 `/healthz`，而 `/healthz` 恒返 200 且不查任何依赖 ——
/// 数据库挂了 Pod 照样 Ready，流量继续被派发进来，而每个请求都 500。
///
/// 形状与状态码判定都收在 `cats_common::ReadyResponse`，本服务只负责真的探一下。
/// 注意这里探的是**数据库**而不是磁盘/PVC：文件元数据存在 DB 里，DB 不可用时
/// 本服务一个 HTTP 请求都处理不了。
pub async fn readyz(pool: web::Data<PgPool>) -> impl Responder {
    let db_ok = sqlx::query("SELECT 1")
        .execute(pool.get_ref())
        .await
        .is_ok();
    let body = cats_common::ReadyResponse::new(env!("CARGO_PKG_NAME"), db_ok);
    HttpResponse::build(
        actix_web::http::StatusCode::from_u16(body.status_code())
            .expect("status_code 只返回 200/503，都是合法状态码"),
    )
    .json(body)
}

pub fn configure_routes(
    cfg: &mut web::ServiceConfig,
    pool: web::Data<PgPool>,
    rbac: web::Data<Arc<RbacChecker>>,
) {
    cfg.app_data(pool)
        .app_data(rbac)
        .route("/healthz", web::get().to(healthz))
        .route("/readyz", web::get().to(readyz))
        .route("/v1/files", web::post().to(upload_file))
        .route("/v1/files", web::get().to(list_files))
        .route("/v1/files/{id}", web::get().to(download_file))
        .route("/v1/files/{id}", web::delete().to(delete_file))
        .route("/v1/files/{id}/metadata", web::get().to(get_file_metadata));
}
