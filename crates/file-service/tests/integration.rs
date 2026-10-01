//! file-service ULYS-152 切片 B-3 实战落地 e2e 测试
//!
//! 引用: ULYS-152 切片 B-3 (file-service 业务 4 endpoint)
//! 引用: doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0+1.md §2 (测试模式参考)
//!
//! 覆盖:
//! - healthz → 200
//! - POST /v1/files 上传 → 201 + FileMetadata
//! - GET  /v1/files/{id}/metadata 命中 → 200
//! - GET  /v1/files/{id}/metadata 不存在 → 404
//! - GET  /v1/files/{id} 下载 → 200 + base64 content
//! - GET  /v1/files 列表 → 200 + 包含已上传文件
//! - DELETE /v1/files/{id} 软删除 → 204
//! - DELETE /v1/files/{id} 二次删除 → 404
//!
//! 完成判据 (per ULYS-152 切片 B-3):
//! ① cargo check -p file-service exit 0
//! ② cargo test -p file-service 集成测试通过 (本文件)
//! ③ healthz e2e 1/1 通过
//! ④ 4 业务 endpoint (POST/GET metadata/GET download/DELETE) 落地

use actix_web::{test as actix_test, web, App};
use base64::Engine as _;
use file_service::handlers;
use file_service::models::{
    FileListResponse, FileMetadataResponse, UploadFileRequest, UploadFileResponse,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::env;
use std::sync::Once;
use uuid::Uuid;

/// 集成测试环境 (env var 一次性检查)
fn setup_env() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        if env::var("DATABASE_URL").is_err() {
            panic!("DATABASE_URL must be set for e2e tests (e.g. postgres://svc_file:***@localhost:5432/file_test_db)");
        }
    });
}

async fn make_pool() -> PgPool {
    file_service::db::build_pool()
        .await
        .expect("build_pool failed (check DATABASE_URL)")
}

fn unique_test_workspace() -> Uuid {
    Uuid::new_v4()
}

async fn cleanup_test_files(pool: &PgPool, workspace_id: Uuid) {
    sqlx::query("DELETE FROM files WHERE workspace_id = $1")
        .bind(workspace_id)
        .execute(pool)
        .await
        .ok();
}

/// 构造 actix App 用于测试 (与 main.rs 一致)
fn make_app(
    pool: PgPool,
) -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let pool_data = web::Data::new(pool);
    App::new()
        .app_data(pool_data)
        .route("/healthz", web::get().to(handlers::healthz))
        .route("/v1/files", web::post().to(handlers::upload_file))
        .route("/v1/files", web::get().to(handlers::list_files))
        .route("/v1/files/{id}", web::get().to(handlers::download_file))
        .route("/v1/files/{id}", web::delete().to(handlers::delete_file))
        .route(
            "/v1/files/{id}/metadata",
            web::get().to(handlers::get_file_metadata),
        )
}

/// 测试用下载响应视图 (与 handlers::DownloadResponse 字段对齐)
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DownloadResponseView {
    pub file: FileMetadataResponse,
    pub content_base64: String,
}

// =====================================================================
// 1. healthz
// =====================================================================
#[actix_web::test]
async fn e2e_healthz_returns_200() {
    setup_env();
    let pool = make_pool().await;
    let app = actix_test::init_service(make_app(pool)).await;
    let req = actix_test::TestRequest::get().uri("/healthz").to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = actix_test::read_body_json(resp).await;
    assert_eq!(body["status"], json!("ok"));
    assert_eq!(body["name"], json!("file-service"));
}

// =====================================================================
// 2. POST /v1/files 上传 → 201
// =====================================================================
#[actix_web::test]
async fn e2e_upload_file_returns_201() {
    setup_env();
    let pool = make_pool().await;
    let workspace_id = unique_test_workspace();
    let content = b"hello world, file-service e2e test";
    let content_b64 = base64::engine::general_purpose::STANDARD.encode(content);

    let app = actix_test::init_service(make_app(pool.clone())).await;
    let req = actix_test::TestRequest::post()
        .uri("/v1/files")
        .set_json(UploadFileRequest {
            workspace_id,
            filename: "test-e2e.txt".to_string(),
            content_type: "text/plain".to_string(),
            content_base64: content_b64.clone(),
        })
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 201, "upload should return 201");
    let body: UploadFileResponse = actix_test::read_body_json(resp).await;
    assert_eq!(body.file.filename, "test-e2e.txt");
    assert_eq!(body.file.size_bytes, content.len() as i64);
    assert_eq!(body.file.workspace_id, workspace_id.to_string());
    assert!(!body.deduplicated);
    // sha256 验证
    let mut hasher = Sha256::new();
    hasher.update(content);
    let expected_sha = format!("{:x}", hasher.finalize());
    assert_eq!(body.file.sha256, expected_sha);

    cleanup_test_files(&pool, workspace_id).await;
}

// =====================================================================
// 3. GET /v1/files/{id}/metadata 命中 → 200
// =====================================================================
#[actix_web::test]
async fn e2e_get_file_metadata_returns_200() {
    setup_env();
    let pool = make_pool().await;
    let workspace_id = unique_test_workspace();
    let content = b"metadata fetch test";
    let content_b64 = base64::engine::general_purpose::STANDARD.encode(content);

    let app = actix_test::init_service(make_app(pool.clone())).await;

    // 先 upload
    let create_req = actix_test::TestRequest::post()
        .uri("/v1/files")
        .set_json(UploadFileRequest {
            workspace_id,
            filename: "meta.txt".to_string(),
            content_type: "text/plain".to_string(),
            content_base64: content_b64,
        })
        .to_request();
    let create_resp = actix_test::call_service(&app, create_req).await;
    let created: UploadFileResponse = actix_test::read_body_json(create_resp).await;
    let id = created.file.id.clone();

    // 再 get metadata
    let get_req = actix_test::TestRequest::get()
        .uri(&format!("/v1/files/{id}/metadata"))
        .to_request();
    let get_resp = actix_test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status().as_u16(), 200);
    let meta: FileMetadataResponse = actix_test::read_body_json(get_resp).await;
    assert_eq!(meta.id, id);
    assert_eq!(meta.filename, "meta.txt");
    assert_eq!(meta.status, "active");

    cleanup_test_files(&pool, workspace_id).await;
}

// =====================================================================
// 4. GET /v1/files/{id}/metadata 不存在 → 404
// =====================================================================
#[actix_web::test]
async fn e2e_get_file_metadata_not_found_returns_404() {
    setup_env();
    let pool = make_pool().await;
    let app = actix_test::init_service(make_app(pool)).await;

    let non_existing = Uuid::new_v4();
    let req = actix_test::TestRequest::get()
        .uri(&format!("/v1/files/{non_existing}/metadata"))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
    let body: serde_json::Value = actix_test::read_body_json(resp).await;
    assert_eq!(body["error"], json!("file_not_found"));
}

// =====================================================================
// 5. GET /v1/files/{id} 下载 → 200 + base64 content 一致
// =====================================================================
#[actix_web::test]
async fn e2e_download_file_returns_200() {
    setup_env();
    let pool = make_pool().await;
    let workspace_id = unique_test_workspace();
    let content = b"download test payload, ascii safe base64.";
    let content_b64 = base64::engine::general_purpose::STANDARD.encode(content);

    let app = actix_test::init_service(make_app(pool.clone())).await;

    // upload
    let create_req = actix_test::TestRequest::post()
        .uri("/v1/files")
        .set_json(UploadFileRequest {
            workspace_id,
            filename: "download.txt".to_string(),
            content_type: "text/plain".to_string(),
            content_base64: content_b64,
        })
        .to_request();
    let create_resp = actix_test::call_service(&app, create_req).await;
    let created: UploadFileResponse = actix_test::read_body_json(create_resp).await;
    let id = created.file.id.clone();

    // download
    let dl_req = actix_test::TestRequest::get()
        .uri(&format!("/v1/files/{id}"))
        .to_request();
    let dl_resp = actix_test::call_service(&app, dl_req).await;
    assert_eq!(dl_resp.status().as_u16(), 200);
    let body: DownloadResponseView = actix_test::read_body_json(dl_resp).await;
    assert_eq!(body.file.id, id);
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&body.content_base64)
        .expect("base64 decode");
    assert_eq!(decoded, content);

    cleanup_test_files(&pool, workspace_id).await;
}

// =====================================================================
// 6. GET /v1/files 列表 → 200 + 包含已上传文件
// =====================================================================
#[actix_web::test]
async fn e2e_list_files_returns_200() {
    setup_env();
    let pool = make_pool().await;
    let workspace_id = unique_test_workspace();
    let content_b64 = base64::engine::general_purpose::STANDARD.encode(b"list test");

    let app = actix_test::init_service(make_app(pool.clone())).await;

    // upload 2 files
    for i in 0..2 {
        let req = actix_test::TestRequest::post()
            .uri("/v1/files")
            .set_json(UploadFileRequest {
                workspace_id,
                filename: format!("list-{i}.txt"),
                content_type: "text/plain".to_string(),
                content_base64: content_b64.clone(),
            })
            .to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 201);
    }

    // list
    let list_req = actix_test::TestRequest::get()
        .uri(&format!("/v1/files?workspace_id={workspace_id}&limit=10"))
        .to_request();
    let list_resp = actix_test::call_service(&app, list_req).await;
    assert_eq!(list_resp.status().as_u16(), 200);
    let list: FileListResponse = actix_test::read_body_json(list_resp).await;
    assert!(list.total >= 2, "total should be >= 2, got {}", list.total);
    assert!(list.items.iter().any(|i| i.filename == "list-0.txt"));
    assert!(list.items.iter().any(|i| i.filename == "list-1.txt"));

    cleanup_test_files(&pool, workspace_id).await;
}

// =====================================================================
// 7. DELETE /v1/files/{id} 软删除 → 204
// =====================================================================
#[actix_web::test]
async fn e2e_delete_file_returns_204() {
    setup_env();
    let pool = make_pool().await;
    let workspace_id = unique_test_workspace();
    let content_b64 = base64::engine::general_purpose::STANDARD.encode(b"delete me");

    let app = actix_test::init_service(make_app(pool.clone())).await;

    // upload
    let create_req = actix_test::TestRequest::post()
        .uri("/v1/files")
        .set_json(UploadFileRequest {
            workspace_id,
            filename: "to-delete.txt".to_string(),
            content_type: "text/plain".to_string(),
            content_base64: content_b64,
        })
        .to_request();
    let create_resp = actix_test::call_service(&app, create_req).await;
    let created: UploadFileResponse = actix_test::read_body_json(create_resp).await;
    let id = created.file.id.clone();

    // delete
    let del_req = actix_test::TestRequest::delete()
        .uri(&format!("/v1/files/{id}"))
        .to_request();
    let del_resp = actix_test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status().as_u16(), 204);

    // 二次 delete → 404
    let del2_req = actix_test::TestRequest::delete()
        .uri(&format!("/v1/files/{id}"))
        .to_request();
    let del2_resp = actix_test::call_service(&app, del2_req).await;
    assert_eq!(del2_resp.status().as_u16(), 404);

    // get metadata → 404
    let get_req = actix_test::TestRequest::get()
        .uri(&format!("/v1/files/{id}/metadata"))
        .to_request();
    let get_resp = actix_test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status().as_u16(), 404);

    cleanup_test_files(&pool, workspace_id).await;
}

// =====================================================================
// 8. POST /v1/files 字段校验: 空 filename → 400
// =====================================================================
#[actix_web::test]
async fn e2e_upload_empty_filename_returns_400() {
    setup_env();
    let pool = make_pool().await;
    let app = actix_test::init_service(make_app(pool)).await;
    let req = actix_test::TestRequest::post()
        .uri("/v1/files")
        .set_json(UploadFileRequest {
            workspace_id: Uuid::new_v4(),
            filename: "".to_string(),
            content_type: "text/plain".to_string(),
            content_base64: base64::engine::general_purpose::STANDARD.encode(b"x"),
        })
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 400);
    let body: serde_json::Value = actix_test::read_body_json(resp).await;
    assert_eq!(body["error"], json!("invalid_request"));
}
