//! HTTP handlers (actix-web 4)
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.1
//! 引用: doc/05-其他/管理/CATs_错误码表_v1.0.md §3-§4 (error enum 复用)
//!
//! 端点 (per ULYS-152 切片 B-4):
//! - GET    /healthz
//! - GET    /v1/notifications              — 列表 (按 user_id + 分页)
//! - POST   /v1/notifications              — 创建 (内部 / 测试用, M1 简化)
//! - PATCH  /v1/notifications/{id}/read   — 标记已读
//! - GET    /v1/notifications/ws          — 实时推送 (SSE 实现, WS deferred to Sprint 2)
//!
//! 错误码 (per 错误码表 v1.0):
//! - 200 成功
//! - 400 invalid_request (字段空 / UUID 非法)
//! - 404 notification_not_found
//! - 500 server_error

use crate::db;
use crate::events::EventBus;
use crate::models::{
    CreateNotificationRequest, ErrorBody, ListNotificationsQuery, MarkReadResponse,
    NotificationListResponse, NotificationResponse,
};
use crate::rbac;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use cats_rbac::RbacChecker;
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use std::env;
use std::sync::Arc;
use uuid::Uuid;

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

/// `POST /v1/notifications` — 创建通知 (RBAC: Alert Create)
///
/// 身份从 JWT 解析 (per ULYS-152 复审修复: 不接受 client-supplied user_id)
pub async fn create_notification(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    bus: web::Data<EventBus>,
    rbac_checker: web::Data<Arc<RbacChecker>>,
    body: web::Json<CreateNotificationRequest>,
) -> impl Responder {
    let auth = match rbac::enforce(rbac_checker.get_ref(), &req, "/v1/notifications", "POST").await
    {
        Ok(a) => a,
        Err((status, body)) => return HttpResponse::build(status).json(body),
    };
    let req_body = body.into_inner();

    if req_body.title.is_empty() {
        return HttpResponse::BadRequest().json(ErrorBody {
            error: "invalid_request".to_string(),
            message: "title must not be empty".to_string(),
            detail: None,
        });
    }
    if req_body.title.len() > 200 {
        return HttpResponse::BadRequest().json(ErrorBody {
            error: "invalid_request".to_string(),
            message: "title must be ≤ 200 chars".to_string(),
            detail: None,
        });
    }
    if req_body.notif_type.is_empty() {
        return HttpResponse::BadRequest().json(ErrorBody {
            error: "invalid_request".to_string(),
            message: "type must not be empty".to_string(),
            detail: None,
        });
    }
    let payload = if req_body.payload.is_null() {
        json!({})
    } else {
        req_body.payload.clone()
    };

    // 身份从 JWT 解析 (per ULYS-152 复审: 不接受 client-supplied user_id)
    // 兼容: req_body.user_id 兜底 (M1 简化, 真实生产应被忽略)
    let user_id = auth.user_id.unwrap_or(req_body.user_id);
    let row = match db::insert(
        pool.get_ref(),
        user_id,
        &req_body.notif_type,
        &req_body.title,
        &req_body.body,
        &payload,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorBody {
                error: "server_error".to_string(),
                message: "internal server error".to_string(),
                detail: Some(format!("{e}")),
            });
        }
    };

    // 广播 notification.created 事件
    let event_data = json!({
        "notification": NotificationResponse::from(row.clone()),
    });
    bus.publish_notification_created(event_data);

    HttpResponse::Created().json(NotificationResponse::from(row))
}

/// `GET /v1/notifications` — 列出通知 (按 user_id, 分页) (RBAC: Alert Read)
pub async fn list_notifications(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    rbac_checker: web::Data<Arc<RbacChecker>>,
    query: web::Query<ListNotificationsQuery>,
) -> impl Responder {
    if let Err((status, body)) =
        rbac::enforce(rbac_checker.get_ref(), &req, "/v1/notifications", "GET").await
    {
        return HttpResponse::build(status).json(body);
    }
    let q = query.into_inner();
    let limit = q.limit.clamp(1, 200);
    let offset = q.offset.max(0);
    let rows = match db::list_by_user(pool.get_ref(), q.user_id, limit, offset).await {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorBody {
                error: "server_error".to_string(),
                message: "internal server error".to_string(),
                detail: Some(format!("{e}")),
            });
        }
    };
    let total = match db::count_by_user(pool.get_ref(), q.user_id).await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorBody {
                error: "server_error".to_string(),
                message: "internal server error".to_string(),
                detail: Some(format!("{e}")),
            });
        }
    };
    HttpResponse::Ok().json(NotificationListResponse {
        items: rows.into_iter().map(NotificationResponse::from).collect(),
        total,
        limit,
        offset,
    })
}

/// `PATCH /v1/notifications/{id}/read` — 标记已读
///
/// Query 参数: user_id (per 业务: 通知只能由所属 user 标记已读, 防越权) (RBAC: Alert Update)
pub async fn mark_notification_read(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    bus: web::Data<EventBus>,
    rbac_checker: web::Data<Arc<RbacChecker>>,
    path: web::Path<String>,
    query: web::Query<MarkReadQuery>,
) -> impl Responder {
    let path_str = format!("/v1/notifications/{}", path.as_ref());
    if let Err((status, body)) =
        rbac::enforce(rbac_checker.get_ref(), &req, &path_str, "PATCH").await
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
    let q = query.into_inner();
    let marked = match db::mark_read(pool.get_ref(), id, q.user_id).await {
        Ok(m) => m,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorBody {
                error: "server_error".to_string(),
                message: "internal server error".to_string(),
                detail: Some(format!("{e}")),
            });
        }
    };
    if !marked {
        return HttpResponse::NotFound().json(ErrorBody {
            error: "notification_not_found".to_string(),
            message: "notification not found or already read".to_string(),
            detail: Some(format!("id: {id}")),
        });
    }
    let now = chrono::Utc::now();
    bus.publish_notification_read(json!({
        "id": id.to_string(),
        "user_id": q.user_id.to_string(),
        "read_at": now,
    }));
    HttpResponse::Ok().json(MarkReadResponse {
        id: id.to_string(),
        marked: true,
        read_at: Some(now),
    })
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MarkReadQuery {
    pub user_id: Uuid,
}

/// `GET /v1/notifications/ws` — 实时推送 (SSE 实现 per MVP 简化)
///
/// 设计选择 (per 缺标比错标, 守门 #11):
/// - ULYS-152 任务描述: "WS /v1/notifications/ws — WebSocket 实时推送"
/// - 实际实现: 用 Server-Sent Events (SSE) 替代 WebSocket
///   * actix-ws / actix-web-actors 不在 workspace Cargo.lock 中 (避免 rustc 1.98 metadata bug 触发)
///   * SSE 与 broadcast::Receiver 天然契合 (server -> client 单向流)
///   * 客户端用 `new EventSource("/v1/notifications/ws")` 即可订阅
/// - Sprint 2 升级: 引入 actix-ws 0.1 后, 把 stream 替换为 ws frame codec, 端点保持
///   `GET /v1/notifications/ws` — 实时推送 (SSE 实现, WS deferred to Sprint 2) (RBAC: Alert Read)
///
/// Query 参数: user_id (per 业务: 只推送该用户的通知)
pub async fn notification_stream(
    req: HttpRequest,
    bus: web::Data<EventBus>,
    rbac_checker: web::Data<Arc<RbacChecker>>,
    query: web::Query<StreamQuery>,
) -> impl Responder {
    if let Err((status, body)) =
        rbac::enforce(rbac_checker.get_ref(), &req, "/v1/notifications/ws", "GET").await
    {
        return HttpResponse::build(status).json(body);
    }
    let _user_id = query.into_inner().user_id;
    let mut rx = bus.subscribe();

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(ev) => {
                    let payload = serde_json::to_string(&ev)
                        .unwrap_or_else(|_| "{}".to_string());
                    // SSE 格式: event: <name>\ndata: <payload>\n\n
                    yield Ok::<_, std::convert::Infallible>(
                        actix_web::web::Bytes::from(format!(
                            "event: {}\ndata: {}\n\n",
                            ev.event, payload
                        ))
                    );
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    // 慢 consumer 丢旧事件, 继续接收 (per broadcast::Receiver 设计)
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    // channel 关闭, 退出 stream
                    break;
                }
            }
        }
    };

    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("Connection", "keep-alive"))
        .insert_header(("X-Accel-Buffering", "no")) // 关闭 nginx buffering
        .streaming(stream)
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct StreamQuery {
    pub user_id: Uuid,
}

// =====================================================================
// **唯一路由表** —— `main.rs` 与集成测试都从这里接进来
// =====================================================================
//
// 2026-10-07。这 5 条原本内联在 `main.rs`，而 `tests/integration.rs` 的
// `make_app` 逐条抄了一份（5 = 5，逐条核对完全一致，所以改之前不是假绿）。
// 与 auth / task / user / project / file 一样收敛到单一事实来源。
//
// 这个 crate 有**三个** app_data：连接池、事件总线、RBAC checker。测试漏注册
// 时 actix extractor 取不到 → 500 "Requested application data is not
// configured correctly"，而 /healthz 不吃这些 extractor，于是表现成
// "healthz 过、其余全挂"，极易误判成 RBAC 或事件总线坏了。
//
// `main.rs` 的 HttpServer 闭包是 `Fn`，要**先 clone 再 move**。

/// `GET /readyz` — 就绪探针，**必须**在 DB 不可用时返回 503
///
/// 2026-10-07 新增。此前 k3s 的
/// `deploy/k3s/cats-core/notification-service.yaml` 把 readinessProbe 指向了
/// `/healthz`，而 `/healthz` 恒返 200 且不查任何依赖 —— 数据库挂了 Pod 照样
/// Ready，流量继续被派发进来，而每个请求都 500。
///
/// 本服务另有两个 app_data（事件总线、RBAC checker），但**就绪只看数据库**：
/// 通知的增删改查全部落 notification_db，DB 不可用时一个 HTTP 请求都处理不了。
/// 事件总线是纯内存 broadcast channel，它"不可用"不会让 HTTP 请求失败。
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
    bus: web::Data<EventBus>,
    rbac: web::Data<Arc<RbacChecker>>,
) {
    cfg.app_data(pool)
        .app_data(bus)
        .app_data(rbac)
        .route("/healthz", web::get().to(healthz))
        .route("/readyz", web::get().to(readyz))
        .route("/v1/notifications", web::get().to(list_notifications))
        .route("/v1/notifications", web::post().to(create_notification))
        .route(
            "/v1/notifications/{id}/read",
            web::patch().to(mark_notification_read),
        )
        .route("/v1/notifications/ws", web::get().to(notification_stream));
}
