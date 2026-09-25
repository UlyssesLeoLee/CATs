//! notification-service ULYS-152 切片 B-4 实战落地 e2e 测试
//!
//! 引用: ULYS-152 切片 B-4 (notification-service 业务 3 endpoint)
//! 引用: doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0+1.md §2 (测试模式参考)
//!
//! 覆盖:
//! - healthz → 200
//! - POST /v1/notifications 创建 → 201 + NotificationResponse
//! - GET  /v1/notifications 列表 → 200 + 包含已创建
//! - PATCH /v1/notifications/{id}/read 标记已读 → 200
//! - PATCH /v1/notifications/{id}/read 二次 → 404
//! - GET  /v1/notifications/ws 实时推送 (SSE) → 200 + text/event-stream
//! - POST 空 title → 400
//!
//! 完成判据 (per ULYS-152 切片 B-4):
//! ① cargo check -p notification-service exit 0
//! ② cargo test -p notification-service 集成测试通过 (本文件)
//! ③ healthz e2e 1/1 通过
//! ④ 3 业务 endpoint (GET list / PATCH mark read / WS stream) 落地

use actix_web::{test as actix_test, web, App};
use notification_service::{CreateNotificationRequest, EventBus, NotificationListResponse, NotificationResponse};
use serde_json::json;
use sqlx::PgPool;
use std::env;
use std::sync::Once;
use uuid::Uuid;

fn setup_env() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        if env::var("DATABASE_URL").is_err() {
            panic!("DATABASE_URL must be set for e2e tests");
        }
    });
}

async fn make_pool() -> PgPool {
    notification_service::db::build_pool()
        .await
        .expect("build_pool failed (check DATABASE_URL)")
}

fn unique_test_user() -> Uuid {
    Uuid::new_v4()
}

async fn cleanup_test_notifications(pool: &PgPool, user_id: Uuid) {
    sqlx::query("DELETE FROM notifications WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await
        .ok();
}

fn make_app(
    pool: PgPool,
    bus: EventBus,
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
    let bus_data = web::Data::new(bus);
    App::new()
        .app_data(pool_data)
        .app_data(bus_data)
        .route("/healthz", web::get().to(notification_service::handlers::healthz))
        .route(
            "/v1/notifications",
            web::get().to(notification_service::handlers::list_notifications),
        )
        .route(
            "/v1/notifications",
            web::post().to(notification_service::handlers::create_notification),
        )
        .route(
            "/v1/notifications/{id}/read",
            web::patch().to(notification_service::handlers::mark_notification_read),
        )
        .route(
            "/v1/notifications/ws",
            web::get().to(notification_service::handlers::notification_stream),
        )
}

// =====================================================================
// 1. healthz
// =====================================================================
#[actix_web::test]
async fn e2e_healthz_returns_200() {
    setup_env();
    let pool = make_pool().await;
    let app = actix_test::init_service(make_app(pool, EventBus::new())).await;
    let req = actix_test::TestRequest::get().uri("/healthz").to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = actix_test::read_body_json(resp).await;
    assert_eq!(body["status"], json!("ok"));
    assert_eq!(body["name"], json!("notification-service"));
}

// =====================================================================
// 2. POST /v1/notifications 创建 → 201
// =====================================================================
#[actix_web::test]
async fn e2e_create_notification_returns_201() {
    setup_env();
    let pool = make_pool().await;
    let user_id = unique_test_user();
    let app = actix_test::init_service(make_app(pool.clone(), EventBus::new())).await;
    let req = actix_test::TestRequest::post()
        .uri("/v1/notifications")
        .set_json(CreateNotificationRequest {
            user_id,
            notif_type: "task_completed".to_string(),
            title: "Task finished".to_string(),
            body: "Your task has been translated.".to_string(),
            payload: json!({"task_id": "abc-123"}),
        })
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 201, "create should return 201");
    let body: NotificationResponse = actix_test::read_body_json(resp).await;
    assert_eq!(body.user_id, user_id.to_string());
    assert_eq!(body.notif_type, "task_completed");
    assert_eq!(body.title, "Task finished");
    assert_eq!(body.status, "active");
    assert!(body.read_at.is_none());

    cleanup_test_notifications(&pool, user_id).await;
}

// =====================================================================
// 3. GET /v1/notifications 列表 → 200 + 包含已创建
// =====================================================================
#[actix_web::test]
async fn e2e_list_notifications_returns_200() {
    setup_env();
    let pool = make_pool().await;
    let user_id = unique_test_user();
    let app = actix_test::init_service(make_app(pool.clone(), EventBus::new())).await;

    // create 2
    for i in 0..2 {
        let req = actix_test::TestRequest::post()
            .uri("/v1/notifications")
            .set_json(CreateNotificationRequest {
                user_id,
                notif_type: "test".to_string(),
                title: format!("title-{i}"),
                body: format!("body-{i}"),
                payload: json!({}),
            })
            .to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 201);
    }

    // list
    let list_req = actix_test::TestRequest::get()
        .uri(&format!("/v1/notifications?user_id={user_id}&limit=10"))
        .to_request();
    let list_resp = actix_test::call_service(&app, list_req).await;
    assert_eq!(list_resp.status().as_u16(), 200);
    let list: NotificationListResponse = actix_test::read_body_json(list_resp).await;
    assert!(list.total >= 2, "total should be >= 2, got {}", list.total);
    assert!(list.items.iter().any(|i| i.title == "title-0"));
    assert!(list.items.iter().any(|i| i.title == "title-1"));

    cleanup_test_notifications(&pool, user_id).await;
}

// =====================================================================
// 4. PATCH /v1/notifications/{id}/read → 200
// =====================================================================
#[actix_web::test]
async fn e2e_mark_read_returns_200() {
    setup_env();
    let pool = make_pool().await;
    let user_id = unique_test_user();
    let app = actix_test::init_service(make_app(pool.clone(), EventBus::new())).await;

    // create
    let create_req = actix_test::TestRequest::post()
        .uri("/v1/notifications")
        .set_json(CreateNotificationRequest {
            user_id,
            notif_type: "task_completed".to_string(),
            title: "to-read".to_string(),
            body: "body".to_string(),
            payload: json!({}),
        })
        .to_request();
    let create_resp = actix_test::call_service(&app, create_req).await;
    let created: NotificationResponse = actix_test::read_body_json(create_resp).await;
    let id = created.id.clone();

    // mark read
    let read_req = actix_test::TestRequest::patch()
        .uri(&format!("/v1/notifications/{id}/read?user_id={user_id}"))
        .to_request();
    let read_resp = actix_test::call_service(&app, read_req).await;
    assert_eq!(read_resp.status().as_u16(), 200);
    let body: serde_json::Value = actix_test::read_body_json(read_resp).await;
    assert_eq!(body["marked"], json!(true));
    assert!(body["read_at"].is_string());

    // 二次 mark read → 404
    let read2_req = actix_test::TestRequest::patch()
        .uri(&format!("/v1/notifications/{id}/read?user_id={user_id}"))
        .to_request();
    let read2_resp = actix_test::call_service(&app, read2_req).await;
    assert_eq!(read2_resp.status().as_u16(), 404);
    let err: serde_json::Value = actix_test::read_body_json(read2_resp).await;
    assert_eq!(err["error"], json!("notification_not_found"));

    cleanup_test_notifications(&pool, user_id).await;
}

// =====================================================================
// 5. GET /v1/notifications/ws (SSE) → 200 + Content-Type text/event-stream
// =====================================================================
#[actix_web::test]
async fn e2e_notification_stream_returns_sse() {
    setup_env();
    let pool = make_pool().await;
    let bus = EventBus::new();
    let user_id = unique_test_user();
    let app = actix_test::init_service(make_app(pool.clone(), bus.clone())).await;

    let req = actix_test::TestRequest::get()
        .uri(&format!("/v1/notifications/ws?user_id={user_id}"))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);
    let ct = resp
        .headers()
        .get(actix_web::http::header::CONTENT_TYPE)
        .expect("Content-Type header should be set")
        .to_str()
        .unwrap();
    assert!(
        ct.starts_with("text/event-stream"),
        "Content-Type should be text/event-stream, got {ct}"
    );
}

// =====================================================================
// 6. POST /v1/notifications 空 title → 400
// =====================================================================
#[actix_web::test]
async fn e2e_create_empty_title_returns_400() {
    setup_env();
    let pool = make_pool().await;
    let app = actix_test::init_service(make_app(pool, EventBus::new())).await;
    let req = actix_test::TestRequest::post()
        .uri("/v1/notifications")
        .set_json(CreateNotificationRequest {
            user_id: Uuid::new_v4(),
            notif_type: "test".to_string(),
            title: "".to_string(),
            body: "body".to_string(),
            payload: json!({}),
        })
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 400);
    let body: serde_json::Value = actix_test::read_body_json(resp).await;
    assert_eq!(body["error"], json!("invalid_request"));
}
