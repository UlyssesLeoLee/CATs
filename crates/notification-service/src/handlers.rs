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
    let auth = match rbac::enforce(rbac_checker.get_ref(), &req, "/v1/notifications", "POST").await {
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
    if let Err((status, body)) = rbac::enforce(rbac_checker.get_ref(), &req, "/v1/notifications", "GET").await {
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
    if let Err((status, body)) = rbac::enforce(rbac_checker.get_ref(), &req, &path_str, "PATCH").await {
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
/// `GET /v1/notifications/ws` — 实时推送 (SSE 实现, WS deferred to Sprint 2) (RBAC: Alert Read)
///
/// Query 参数: user_id (per 业务: 只推送该用户的通知)
pub async fn notification_stream(
    req: HttpRequest,
    bus: web::Data<EventBus>,
    rbac_checker: web::Data<Arc<RbacChecker>>,
    query: web::Query<StreamQuery>,
) -> impl Responder {
    if let Err((status, body)) = rbac::enforce(rbac_checker.get_ref(), &req, "/v1/notifications/ws", "GET").await {
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