//! project-service B-1 切片集成测试 (e2e)
//!
//! 引用: doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0+1.md §2 (T-02 完成判据模式)
//! 引用: ULYS-150 切片 B-1 §"集成测试 ≥3 case"
//!
//! 覆盖 (>=3 case per 任务书):
//! - healthz → 200
//! - POST /v1/projects 创建 → 201 + 项目
//! - GET  /v1/projects/{id} 命中 → 200 + 字段一致
//! - GET  /v1/projects/{id} 不存在 → 404 project_not_found
//! - GET  /v1/projects?workspace_id=&page=&page_size= 列表
//! - PATCH /v1/projects/{id} 部分更新 → 200
//! - DELETE /v1/projects/{id} 软删除 → 200 + status='archived'
//!
//! 完成判据 (per ULYS-150 切片 B-1):
//! ① cargo check -p project-service exit 0
//! ② 5 endpoint 全部实现 + 错误信封统一
//! ③ ≥3 集成测试 case 通过

use actix_web::{test as actix_test, web, App};
use serde_json::json;
use sqlx::PgPool;
use std::env;
use std::sync::Once;
use uuid::Uuid;

use project_service::handlers;
use project_service::models::{
    CreateProjectRequest, GetProjectResponse, ListProjectsResponse, UpdateProjectRequest,
};

/// 集成测试环境 (env var 一次性检查)
fn setup_env() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        if env::var("DATABASE_URL").is_err() {
            panic!("DATABASE_URL must be set for e2e tests (e.g. postgres://svc_project:***@localhost:5432/project_test_db)");
        }
    });
}

async fn make_pool() -> PgPool {
    project_service::db::build_pool()
        .await
        .expect("build_pool failed (check DATABASE_URL)")
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
        .route("/v1/projects", web::post().to(handlers::create_project))
        .route("/v1/projects", web::get().to(handlers::list_projects))
        .route("/v1/projects/{id}", web::get().to(handlers::get_project))
        .route("/v1/projects/{id}", web::patch().to(handlers::patch_project))
        .route(
            "/v1/projects/{id}",
            web::delete().to(handlers::delete_project),
        )
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
    assert_eq!(body["name"], json!("project-service"));
    assert!(body["version"].as_str().unwrap().starts_with("0.1."));
}

// =====================================================================
// 2. POST /v1/projects 创建 → 201
// =====================================================================
#[actix_web::test]
async fn e2e_create_project_returns_201() {
    setup_env();
    let pool = make_pool().await;
    let workspace_id = Uuid::new_v4();
    let owner_user_id = Uuid::new_v4();

    let app = actix_test::init_service(make_app(pool.clone())).await;
    let req = actix_test::TestRequest::post()
        .uri("/v1/projects")
        .set_json(CreateProjectRequest {
            workspace_id,
            name: "ULYS-150 Test Project".to_string(),
            source_lang: "ja".to_string(),
            target_lang: "en".to_string(),
            owner_user_id,
        })
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 201, "create should return 201");
    let body: GetProjectResponse = actix_test::read_body_json(resp).await;
    assert_eq!(body.workspace_id, workspace_id.to_string());
    assert_eq!(body.name, "ULYS-150 Test Project");
    assert_eq!(body.source_lang, "ja");
    assert_eq!(body.target_lang, "en");

    // cleanup
    sqlx::query("DELETE FROM projects WHERE id = $1")
        .bind(Uuid::parse_str(&body.id).unwrap())
        .execute(&pool)
        .await
        .ok();
}

// =====================================================================
// 3. GET /v1/projects/{id} 命中 + 字段一致
// =====================================================================
#[actix_web::test]
async fn e2e_get_project_by_id_returns_200() {
    setup_env();
    let pool = make_pool().await;
    let workspace_id = Uuid::new_v4();
    let owner_user_id = Uuid::new_v4();

    let app = actix_test::init_service(make_app(pool.clone())).await;

    // 先 create
    let create_req = actix_test::TestRequest::post()
        .uri("/v1/projects")
        .set_json(CreateProjectRequest {
            workspace_id,
            name: "B-1 Get Test".to_string(),
            source_lang: "en".to_string(),
            target_lang: "zh-CN".to_string(),
            owner_user_id,
        })
        .to_request();
    let create_resp = actix_test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status().as_u16(), 201);
    let created: GetProjectResponse = actix_test::read_body_json(create_resp).await;
    let id = created.id.clone();

    // 再 get by id
    let get_req = actix_test::TestRequest::get()
        .uri(&format!("/v1/projects/{id}"))
        .to_request();
    let get_resp = actix_test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status().as_u16(), 200);
    let fetched: GetProjectResponse = actix_test::read_body_json(get_resp).await;
    assert_eq!(fetched.id, id);
    assert_eq!(fetched.workspace_id, workspace_id.to_string());
    assert_eq!(fetched.name, "B-1 Get Test");
    assert_eq!(fetched.source_lang, "en");
    assert_eq!(fetched.target_lang, "zh-CN");

    // cleanup
    sqlx::query("DELETE FROM projects WHERE id = $1")
        .bind(Uuid::parse_str(&id).unwrap())
        .execute(&pool)
        .await
        .ok();
}

// =====================================================================
// 4. GET /v1/projects/{id} 不存在 → 404 project_not_found
// =====================================================================
#[actix_web::test]
async fn e2e_get_project_not_found_returns_404() {
    setup_env();
    let pool = make_pool().await;
    let app = actix_test::init_service(make_app(pool)).await;

    let non_existing = Uuid::new_v4();
    let req = actix_test::TestRequest::get()
        .uri(&format!("/v1/projects/{non_existing}"))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
    let body: serde_json::Value = actix_test::read_body_json(resp).await;
    assert_eq!(body["error"], json!("project_not_found"));
}

// =====================================================================
// 5. GET /v1/projects?workspace_id=... 列表
// =====================================================================
#[actix_web::test]
async fn e2e_list_projects_with_workspace_filter() {
    setup_env();
    let pool = make_pool().await;
    let workspace_id = Uuid::new_v4();
    let other_workspace_id = Uuid::new_v4();
    let owner_user_id = Uuid::new_v4();

    let app = actix_test::init_service(make_app(pool.clone())).await;

    // 创建 2 个属于 workspace_id 的项目
    let mut created_ids: Vec<String> = Vec::new();
    for i in 0..2 {
        let req = actix_test::TestRequest::post()
            .uri("/v1/projects")
            .set_json(CreateProjectRequest {
                workspace_id,
                name: format!("B-1 List Test {i}"),
                source_lang: "ja".to_string(),
                target_lang: "en".to_string(),
                owner_user_id,
            })
            .to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 201);
        let body: GetProjectResponse = actix_test::read_body_json(resp).await;
        created_ids.push(body.id);
    }

    // 列表查询 — workspace_id 过滤
    let list_req = actix_test::TestRequest::get()
        .uri(&format!(
            "/v1/projects?workspace_id={workspace_id}&page=1&page_size=10"
        ))
        .to_request();
    let list_resp = actix_test::call_service(&app, list_req).await;
    assert_eq!(list_resp.status().as_u16(), 200);
    let list_body: ListProjectsResponse = actix_test::read_body_json(list_resp).await;
    // total >= 2 (因为可能其他 test 已插入)
    assert!(
        list_body.total >= 2,
        "expected total >= 2, got {}",
        list_body.total
    );
    let count_in_workspace = list_body
        .items
        .iter()
        .filter(|i| i.workspace_id == workspace_id.to_string())
        .count();
    assert!(count_in_workspace >= 2);
    // 全部 page_size=10, page=1
    assert_eq!(list_body.page, 1);
    assert_eq!(list_body.page_size, 10);

    // cleanup
    for id in &created_ids {
        sqlx::query("DELETE FROM projects WHERE id = $1")
            .bind(Uuid::parse_str(id).unwrap())
            .execute(&pool)
            .await
            .ok();
    }

    // 防未使用变量
    let _ = other_workspace_id;
}

// =====================================================================
// 6. PATCH /v1/projects/{id} 部分更新
// =====================================================================
#[actix_web::test]
async fn e2e_patch_project_partial_returns_200() {
    setup_env();
    let pool = make_pool().await;
    let workspace_id = Uuid::new_v4();
    let owner_user_id = Uuid::new_v4();

    let app = actix_test::init_service(make_app(pool.clone())).await;

    // create
    let create_req = actix_test::TestRequest::post()
        .uri("/v1/projects")
        .set_json(CreateProjectRequest {
            workspace_id,
            name: "B-1 Patch Original".to_string(),
            source_lang: "ja".to_string(),
            target_lang: "en".to_string(),
            owner_user_id,
        })
        .to_request();
    let create_resp = actix_test::call_service(&app, create_req).await;
    let created: GetProjectResponse = actix_test::read_body_json(create_resp).await;
    let id = created.id.clone();

    // patch 部分字段 (name + target_lang)
    let patch_req = actix_test::TestRequest::patch()
        .uri(&format!("/v1/projects/{id}"))
        .set_json(UpdateProjectRequest {
            name: Some("B-1 Patch Updated".to_string()),
            source_lang: None,
            target_lang: Some("zh-CN".to_string()),
            status: None,
        })
        .to_request();
    let patch_resp = actix_test::call_service(&app, patch_req).await;
    assert_eq!(patch_resp.status().as_u16(), 200);
    let updated: GetProjectResponse = actix_test::read_body_json(patch_resp).await;
    assert_eq!(updated.name, "B-1 Patch Updated");
    assert_eq!(updated.target_lang, "zh-CN");
    // source_lang 保持不变
    assert_eq!(updated.source_lang, "ja");

    // cleanup
    sqlx::query("DELETE FROM projects WHERE id = $1")
        .bind(Uuid::parse_str(&id).unwrap())
        .execute(&pool)
        .await
        .ok();
}

// =====================================================================
// 7. DELETE /v1/projects/{id} 软删除
// =====================================================================
#[actix_web::test]
async fn e2e_delete_project_soft_returns_200() {
    setup_env();
    let pool = make_pool().await;
    let workspace_id = Uuid::new_v4();
    let owner_user_id = Uuid::new_v4();

    let app = actix_test::init_service(make_app(pool.clone())).await;

    // create
    let create_req = actix_test::TestRequest::post()
        .uri("/v1/projects")
        .set_json(CreateProjectRequest {
            workspace_id,
            name: "B-1 Delete Test".to_string(),
            source_lang: "ja".to_string(),
            target_lang: "en".to_string(),
            owner_user_id,
        })
        .to_request();
    let create_resp = actix_test::call_service(&app, create_req).await;
    let created: GetProjectResponse = actix_test::read_body_json(create_resp).await;
    let id = created.id.clone();

    // delete (软删除 → status='archived')
    let del_req = actix_test::TestRequest::delete()
        .uri(&format!("/v1/projects/{id}"))
        .to_request();
    let del_resp = actix_test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status().as_u16(), 200);
    let deleted: GetProjectResponse = actix_test::read_body_json(del_resp).await;
    assert_eq!(deleted.id, id);
    assert!(matches!(
        deleted.status,
        project_service::models::ProjectStatus::Archived
    ));

    // cleanup (强制删除)
    sqlx::query("DELETE FROM projects WHERE id = $1")
        .bind(Uuid::parse_str(&id).unwrap())
        .execute(&pool)
        .await
        .ok();
}