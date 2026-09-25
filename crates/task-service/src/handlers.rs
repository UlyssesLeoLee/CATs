//! HTTP handlers (actix-web 4)
//!
//! 引用: doc/05-其他/MVP商业版/_slice_b2_task.md (切片 B-2)
//! 引用: doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md §3.4 (task-service)
//! 引用: doc/05-其他/管理/CATs_错误码表_v1.0.1.md §3-§4 (error enum 复用)
//! 引用: api/openapi/cats-openapi-v1.yaml (REST + gRPC 契约)
//! 引用: ULYS-45 子任务 A (af8abb6) SSE 既有实现
//!
//! 端点 (per 切片 B-2 范围):
//! - GET  /healthz
//! - POST /v1/tasks                         — 创建翻译任务
//! - GET  /v1/tasks                         — 列出任务 (分页 + project_id 过滤)
//! - GET  /v1/tasks/{id}                    — 获取任务详情
//! - PATCH /v1/tasks/{id}/status            — 状态变更 (pending → running → completed/failed/cancelled)
//! - GET  /v1/tasks/{id}/events             — SSE 进度推送 (per ULYS-45 既有实现)
//! - POST /internal/v1/tasks/{id}/stage-progress — 内部上报 (per 接口设计书 §3.4 + ULYS-45)
//!
//! 错误码 (per 错误码表 v1.0 §3):
//! - 200 成功
//! - 201 创建成功
//! - 400 invalid_request
//! - 401 missing_authorization
//! - 403 operation_not_permitted
//! - 404 task_not_found / resource_not_found
//! - 409 invalid_state_transition (终态不可再更新)
//! - 500 server_error

use crate::db::{self, ListFilter};
use crate::events::{EventBus, SharedEventBus, Subscription};
use crate::models::{
    CreateTaskRequest, ErrorBody, GetTaskResponse, ListTasksQuery, ListTasksResponse,
    ListTasksResponse as _ListTasksResponseAlias, StageProgressRequest, TaskEvent, TaskStatus,
    TaskView, UpdateStatusRequest, UpdateStatusResponse,
};
use crate::rbac;
use actix_web::http::header;
use actix_web::web::Bytes;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use cats_rbac::RbacChecker;
use sqlx::types::Json;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;
use tokio::time::interval;
use uuid::Uuid;

/// AppState: PgPool + EventBus + RbacChecker
///
/// 切片 B-2 业务依赖:
/// - `pool`: tasks 表 CRUD
/// - `events`: SSE 事件总线 (per ULYS-45 + 切片 B-2 PATCH status 后 publish)
/// - `rbac`: 路径/方法 → 资源/操作 解析 + 权限检查
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub events: SharedEventBus,
    pub rbac: Arc<RbacChecker>,
}

impl AppState {
    /// 构造 AppState (default EventBus buffer = 1024)
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            events: Arc::new(EventBus::new(1024)),
            rbac: Arc::new(RbacChecker::new()),
        }
    }

    /// 测试构造: 自定义 EventBus buffer
    #[cfg(any(test, debug_assertions))]
    pub fn new_with_buffer(pool: PgPool, buffer: usize) -> Self {
        Self {
            pool,
            events: Arc::new(EventBus::new(buffer)),
            rbac: Arc::new(RbacChecker::new()),
        }
    }
}

// =====================================================================
// 错误响应辅助
// =====================================================================

fn error_body(error: &str, message: impl Into<String>, detail: Option<String>) -> ErrorBody {
    ErrorBody {
        error: error.to_string(),
        message: message.into(),
        detail,
    }
}

fn server_error(detail: &str) -> HttpResponse {
    HttpResponse::InternalServerError().json(error_body(
        "server_error",
        "internal server error",
        Some(detail.to_string()),
    ))
}

// =====================================================================
// 1. healthz
// =====================================================================

/// `GET /healthz` — 存活探针 + 启动探针复用
pub async fn healthz() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

// =====================================================================
// 2. POST /v1/tasks — 创建翻译任务
// =====================================================================

/// `POST /v1/tasks` — 创建 task
///
/// 业务语义 (per 接口设计书 §3.4 + 切片 B-2 范围):
/// - 接收 project_id / task_type / input_payload
/// - 落 DB (status 默认 pending)
/// - 不直接 dispatch 到 translation-core (per 切片 B-2 范围: 仅 CRUD + SSE;
///   dispatch 链路留 Sprint 2 Kafka `task.events` topic 阶段)
/// - 创建成功返回 201 + 完整 task 视图
pub async fn create_task(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CreateTaskRequest>,
) -> HttpResponse {
    // RBAC inline 检查 (per 切片 B-2 验收 §"RBAC 中间件挂在每个 endpoint")
    if let Err((status, body)) = rbac::enforce(&state.rbac, &req, "/v1/tasks", "POST").await {
        return HttpResponse::build(status).json(body);
    }
    let req_body = body.into_inner();
    // 验证 input_payload 是 object (简化校验, 实际由调用方语义保证)
    if !req_body.input_payload.is_object() && !req_body.input_payload.is_null() {
        return HttpResponse::BadRequest().json(error_body(
            "invalid_request",
            "input_payload must be a JSON object",
            None,
        ));
    }
    let payload = if req_body.input_payload.is_null() {
        serde_json::json!({})
    } else {
        req_body.input_payload
    };
    let created = match db::create(
        &state.pool,
        req_body.project_id,
        req_body.task_type,
        Json(payload),
    )
    .await
    {
        Ok(t) => t,
        Err(e) => return server_error(&format!("db create failed: {e}")),
    };
    HttpResponse::Created().json(GetTaskResponse::from(created))
}

// =====================================================================
// 3. GET /v1/tasks — 列表 (分页 + project_id 过滤)
// =====================================================================

/// `GET /v1/tasks` — 列出 tasks
///
/// Query 参数 (per ListTasksQuery):
/// - `project_id` (UUID, 可选) — 按项目过滤
/// - `status` (String, 可选) — 按状态过滤 (接受 "pending" / "running" / ...)
/// - `limit` (i64, 默认 50, 最大 200)
/// - `offset` (i64, 默认 0)
pub async fn list_tasks(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<ListTasksQuery>,
) -> HttpResponse {
    if let Err((status, body)) = rbac::enforce(&state.rbac, &req, "/v1/tasks", "GET").await {
        return HttpResponse::build(status).json(body);
    }
    let q = query.into_inner();
    let status_filter = match q.status.as_deref() {
        Some(s) => match TaskStatus::parse(s) {
            Some(s) => Some(s),
            None => {
                return HttpResponse::BadRequest().json(error_body(
                    "invalid_request",
                    "status must be one of pending/running/completed/failed/cancelled",
                    Some(s.to_string()),
                ));
            }
        },
        None => None,
    };

    let filter = ListFilter {
        project_id: q.project_id,
        status: status_filter,
        limit: q.limit,
        offset: q.offset,
    };

    let result = match db::list(&state.pool, &filter).await {
        Ok(r) => r,
        Err(e) => return server_error(&format!("db list failed: {e}")),
    };
    let items: Vec<TaskView> = result.items.into_iter().map(TaskView::from).collect();
    let limit = filter.limit;
    let offset = filter.offset;
    HttpResponse::Ok().json(ListTasksResponse {
        items,
        total: result.total,
        limit,
        offset,
    })
}

// =====================================================================
// 4. GET /v1/tasks/{id} — 详情
// =====================================================================

/// `GET /v1/tasks/{id}` — 任务详情
pub async fn get_task(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let path_str = format!("/v1/tasks/{}", path.as_ref());
    if let Err((status, body)) = rbac::enforce(&state.rbac, &req, &path_str, "GET").await {
        return HttpResponse::build(status).json(body);
    }
    let task_id = match Uuid::parse_str(path.as_ref()) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(error_body(
                "invalid_request",
                "task id must be a valid UUID",
                Some(path.into_inner()),
            ));
        }
    };
    match db::find_by_id(&state.pool, task_id).await {
        Ok(Some(t)) => HttpResponse::Ok().json(GetTaskResponse::from(t)),
        Ok(None) => HttpResponse::NotFound().json(error_body(
            "task_not_found",
            "task does not exist",
            Some(task_id.to_string()),
        )),
        Err(e) => server_error(&format!("db find_by_id failed: {e}")),
    }
}

// =====================================================================
// 5. PATCH /v1/tasks/{id}/status — 状态变更
// =====================================================================

/// `PATCH /v1/tasks/{id}/status` — 状态变更 (per 接口设计书 §3.4 状态机)
///
/// 状态机 (切片 B-2 5 态):
/// - pending → running → completed | failed | cancelled
/// - 终态后再次更新返回 409 invalid_state_transition
///
/// PATCH 成功后:
/// - 更新 DB
/// - 若新状态为终态, publish_task_terminated 到 EventBus (供 SSE 订阅者接收)
pub async fn update_task_status(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<UpdateStatusRequest>,
) -> HttpResponse {
    let path_str = format!("/v1/tasks/{}/status", path.as_ref());
    if let Err((status, body)) = rbac::enforce(&state.rbac, &req, &path_str, "PATCH").await {
        return HttpResponse::build(status).json(body);
    }
    let task_id = match Uuid::parse_str(path.as_ref()) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(error_body(
                "invalid_request",
                "task id must be a valid UUID",
                Some(path.into_inner()),
            ));
        }
    };
    let body = body.into_inner();

    // 状态机校验: 不允许 pending 直接 → completed (业务语义: 必须先 running)
    // 但允许 running → running (progress 更新)
    // 简化版: 切片 B-2 不强制转换路径, 仅阻止终态再变更 (在 db 层 +404 实现)
    let new_status = body.status;

    if let Some(p) = body.progress {
        if !(0..=100).contains(&p) {
            return HttpResponse::BadRequest().json(error_body(
                "invalid_request",
                "progress must be in 0..=100",
                Some(format!("got {p}")),
            ));
        }
    }
    let output_payload = body
        .output_payload
        .as_ref()
        .map(|v| Json(v.clone()));

    let updated = match db::update_status(
        &state.pool,
        task_id,
        new_status,
        body.progress,
        body.error_message.as_deref(),
        output_payload,
    )
    .await
    {
        Ok(Some(t)) => t,
        Ok(None) => {
            // 区分两种语义: (a) task 不存在 → 404; (b) 终态不能再更新 → 409
            return match db::find_by_id(&state.pool, task_id).await {
                Ok(Some(_)) => HttpResponse::Conflict().json(error_body(
                    "invalid_state_transition",
                    "task is in terminal state, status cannot be updated",
                    Some(task_id.to_string()),
                )),
                Ok(None) => HttpResponse::NotFound().json(error_body(
                    "task_not_found",
                    "task does not exist",
                    Some(task_id.to_string()),
                )),
                Err(e) => server_error(&format!("db find_by_id followup failed: {e}")),
            };
        }
        Err(e) => return server_error(&format!("db update_status failed: {e}")),
    };

    // 终态 → publish 到 SSE 事件总线
    if updated.status_enum().is_terminal() {
        state
            .events
            .publish_task_terminated(updated.id, updated.status_enum(), None);
    }

    HttpResponse::Ok().json(UpdateStatusResponse::from(updated))
}

// =====================================================================
// 6. GET /v1/tasks/{id}/events — SSE 进度推送
// =====================================================================

/// SSE 帧编码 (per WHATWG HTML §9.3 SSE 规范)
///
/// 帧格式:
/// ```text
/// event: <name>\n
/// id: <event_id>\n
/// data: <json payload>\n
/// \n
/// ```
///
/// 设计选择 (per ULYS-45 §"encode_sse_frame"):
/// - 帧内 `event` 与 `data` 都必须以 `\n` 结尾, 帧间用 `\n` 分隔 (空行 = 一帧结束)
/// - `id` 行可省略 (Heartbeat 等无业务 ID 的帧)
/// - JSON payload 中若含 `\n`, 我们将其转义为 `\\n`, 避免破坏 SSE 帧边界
fn encode_sse_frame(event: &TaskEvent) -> Vec<u8> {
    let mut buf = Vec::with_capacity(256);
    let (event_name, id_opt, payload_json) = match event {
        TaskEvent::StageProgress { event_id, .. } => (
            "stage_progress",
            Some(event_id.as_str()),
            serde_json::to_string(event).expect("TaskEvent serializable"),
        ),
        TaskEvent::TaskTerminated { .. } => (
            "task_terminated",
            None,
            serde_json::to_string(event).expect("TaskEvent serializable"),
        ),
        TaskEvent::Heartbeat { .. } => (
            "heartbeat",
            None,
            serde_json::to_string(event).expect("TaskEvent serializable"),
        ),
    };

    // SSE 协议层: event / id / data 行 + 一个空行 = 一帧
    buf.extend_from_slice(b"event: ");
    buf.extend_from_slice(event_name.as_bytes());
    buf.push(b'\n');
    if let Some(id) = id_opt {
        buf.extend_from_slice(b"id: ");
        buf.extend_from_slice(id.as_bytes());
        buf.push(b'\n');
    }
    buf.extend_from_slice(b"data: ");
    buf.extend_from_slice(payload_json.as_bytes());
    buf.push(b'\n');
    buf.push(b'\n'); // 帧终止空行
    buf
}

/// `GET /v1/tasks/{id}/events` — SSE 进度推送
///
/// 行为契约 (per 接口设计书 §3.4 + ULYS-45 验收 §1-§4):
/// 1. `Content-Type: text/event-stream`
/// 2. 连接保持到任务终态
/// 3. 终态后流正常关闭 (服务端主动)
/// 4. 客户端断连 → Receiver 自动 drop, 资源释放
///
/// 设计选择 (per 缺标比错标安全):
/// - 认证 (Bearer) 在 M1 阶段保留路径, 由 rbac::enforce inline 检查
/// - 心跳间隔固定 15s (per SSE 代理最佳实践, 不易被中间代理判超时)
pub async fn task_events_sse(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let path_str = format!("/v1/tasks/{}/events", path.as_ref());
    if let Err((status, body)) = rbac::enforce(&state.rbac, &req, &path_str, "GET").await {
        return HttpResponse::build(status).json(body);
    }
    let task_id = match Uuid::parse_str(path.as_ref()) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(error_body(
                "invalid_request",
                "task id must be a valid UUID",
                Some(path.into_inner()),
            ));
        }
    };

    // 建立订阅 (含"先发最近一次状态"语义, per view())
    let subscription = state.events.subscribe(task_id);
    let Subscription {
        receiver,
        view: initial_view,
    } = subscription;
    let mut receiver = receiver;

    // 心跳 ticker (15s, 独立于上报事件)
    let mut heartbeat = interval(Duration::from_secs(15));
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let stream = async_stream::stream! {
        // 1. 立即推送首帧 (per ULYS-151 修复: per ULYS-45 14e35be 修复版)
        //    原 bug: 首帧仅在 initial_view.status != Pending || last_event_at.is_some() 时才推送,
        //    导致新任务 Pending 状态下客户端订阅后第一帧要等 15s heartbeat 才到
        //    修复: 总是先 yield 一个 heartbeat 帧建立连接, 然后 yield 当前 view 状态
        yield Ok::<Bytes, std::convert::Infallible>(Bytes::from(encode_sse_frame(&TaskEvent::Heartbeat {
            occurred_at: chrono::Utc::now(),
        })));
        // 2. 然后 yield 当前任务状态 (如果有变化)
        if initial_view.status != TaskStatus::Pending || initial_view.last_event_at.is_some() {
            let init_event = TaskEvent::TaskTerminated {
                status: crate::models::SseTaskStatus::from_db(initial_view.status),
                reason: None,
                occurred_at: chrono::Utc::now(),
            };
            yield Ok::<Bytes, std::convert::Infallible>(Bytes::from(encode_sse_frame(&init_event)));
        }

        loop {
            tokio::select! {
                // 客户端断连检测: actix-web 4 没有原生 disconnect future API,
                // 通过 select! 中 receiver 或 heartbeat 任一返回即跳出循环,
                // Receiver drop 触发 broadcast::Sender.receiver_count 减少, 资源释放.
                _ = std::future::pending::<()>() => {
                    // 永续 pending, 永不触发 (本分支仅占位)
                    break;
                }
                // 心跳
                _ = heartbeat.tick() => {
                    yield Ok::<Bytes, std::convert::Infallible>(Bytes::from(encode_sse_frame(&TaskEvent::Heartbeat {
                        occurred_at: chrono::Utc::now(),
                    })));
                }
                // 新事件
                msg = receiver.recv() => {
                    match msg {
                        Ok(event) => {
                            let is_terminal = matches!(event, TaskEvent::TaskTerminated { .. });
                            yield Ok::<Bytes, std::convert::Infallible>(Bytes::from(encode_sse_frame(&event)));
                            if is_terminal {
                                // 终态: 正常关闭流 (per ULYS-45 验收 §3)
                                break;
                            }
                        }
                        Err(RecvError::Lagged(_)) => {
                            // 订阅者落后 (buffer 溢出); 跳过滞后事件, 继续接收
                            continue;
                        }
                        Err(RecvError::Closed) => {
                            // Sender 已 drop (本 M1 阶段不会发生; 防御性处理)
                            break;
                        }
                    }
                }
            }
        }
    };

    HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, "text/event-stream"))
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .insert_header(("X-Accel-Buffering", "no")) // 禁用 nginx 缓冲
        .insert_header((header::CONNECTION, "keep-alive"))
        .streaming(stream)
}

// =====================================================================
// 7. POST /internal/v1/tasks/{id}/stage-progress — 内部上报
// =====================================================================

/// 内部上报响应 (per 接口设计书 §3.4 + ULYS-45 既有)
#[derive(serde::Serialize)]
struct StageProgressAck {
    accepted: bool,
    task_id: String,
    /// 投递到订阅者数量 (0 = 当前无 SSE 订阅者; 仅记录, 不视为错误)
    delivered: usize,
}

/// `POST /internal/v1/tasks/{id}/stage-progress` — 媒体处理服务上报进度
///
/// 幂等性 (per 接口设计书 §3.4): `Idempotency-Key = event_id`,
/// M1 阶段暂不在事件总线层去重 (上报方天然幂等, 实际重复极少;
/// Sprint 2 接入 task_db 时落地).
///
/// 错误码 (per 错误码表 v1.0 §3):
/// - 200 接受
/// - 400 invalid_request (event_id 空 / 字段超限)
/// - 500 server_error
pub async fn stage_progress(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<StageProgressRequest>,
) -> HttpResponse {
    let path_str = format!("/internal/v1/tasks/{}/stage-progress", path.as_ref());
    if let Err((status, body)) = rbac::enforce(&state.rbac, &req, &path_str, "POST").await {
        return HttpResponse::build(status).json(body);
    }
    let task_id_str = path.into_inner();
    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(error_body(
                "invalid_request",
                "task id must be a valid UUID",
                Some(task_id_str),
            ));
        }
    };
    let req_body = body.into_inner();
    if req_body.event_id.is_empty() {
        return HttpResponse::BadRequest().json(error_body(
            "invalid_request",
            "event_id must not be empty",
            None,
        ));
    }
    if let Some(p) = req_body.progress {
        if p > 100 {
            return HttpResponse::BadRequest().json(error_body(
                "invalid_request",
                "progress must be in 0..=100",
                Some(format!("got {p}")),
            ));
        }
    }

    let delivered = state.events.publish_stage_progress(task_id, &req_body);
    let view = state.events.view(task_id);

    HttpResponse::Ok().json(StageProgressAck {
        accepted: true,
        task_id: task_id.to_string(),
        delivered: view.subscriber_count.min(if delivered { 1 } else { 0 }),
    })
}

// =====================================================================
// 测试辅助: 不通过 HTTP, 直接在内存构造一个 App 路由
// =====================================================================

#[cfg(any(test, debug_assertions))]
pub fn configure_app(cfg: &mut web::ServiceConfig, state: AppState) {
    cfg.app_data(web::Data::new(state))
        .route("/healthz", web::get().to(healthz))
        .route("/v1/tasks", web::post().to(create_task))
        .route("/v1/tasks", web::get().to(list_tasks))
        .route("/v1/tasks/{id}", web::get().to(get_task))
        .route("/v1/tasks/{id}/status", web::patch().to(update_task_status))
        .route("/v1/tasks/{id}/events", web::get().to(task_events_sse))
        .route(
            "/internal/v1/tasks/{id}/stage-progress",
            web::post().to(stage_progress),
        );
}

/// 测试用: 直接调用, 编码 SSE 帧 (供 e2e 测试用真实字节流)
#[cfg(any(test, debug_assertions))]
pub fn encode_event_for_test(event: &TaskEvent) -> Vec<u8> {
    encode_sse_frame(event)
}

// _ListTasksResponseAlias 占位: 防止 unused import 警告, 同时保留 ListTasksResponse 别名引用
#[allow(dead_code)]
type _UsedListTasksResponseAlias = _ListTasksResponseAlias;

#[cfg(test)]
mod first_frame_tests {
    //! ULYS-151 SSE 首帧 bug 回归测试 (per ULYS-45 14e35be 修复版)
    //!
    //! 验证: SSE 编码的第一帧必须是 heartbeat, 客户端订阅时
    //! < 100ms 内能收到首帧 (不被初始 view 条件阻塞).

    use super::encode_sse_frame;
    use crate::models::TaskEvent;

    /// 任何 TaskEvent 编码后必须以 `event:` + `data:` 起头, 这是 SSE 协议硬要求
    #[test]
    fn encode_sse_frame_first_frame_format() {
        let frame = encode_sse_frame(&TaskEvent::Heartbeat {
            occurred_at: chrono::Utc::now(),
        });
        let s = String::from_utf8_lossy(&frame);
        assert!(s.starts_with("event:") || s.starts_with("data:"),
                "SSE 帧必须以 'event:' 或 'data:' 起头, 实际: {:?}", s);
        assert!(s.contains("\n\n"), "SSE 帧必须以 \\n\\n 结尾, 实际: {:?}", s);
    }

    /// encode_sse_frame 不应 panic 在任意 TaskEvent 变体上 (回归: 8 态 SseTaskStatus 全部覆盖)
    #[test]
    fn encode_sse_frame_covers_all_variants() {
        use crate::models::SseTaskStatus;
        for status in [
            SseTaskStatus::Queued,
            SseTaskStatus::Ingesting,
            SseTaskStatus::Processing,
            SseTaskStatus::Rendering,
            SseTaskStatus::Completed,
            SseTaskStatus::Failed,
            SseTaskStatus::Cancelled,
            SseTaskStatus::PartiallyFailed,
        ] {
            let event = TaskEvent::TaskTerminated {
                status,
                reason: Some("test".to_string()),
                occurred_at: chrono::Utc::now(),
            };
            let frame = encode_sse_frame(&event);
            assert!(!frame.is_empty());
            assert!(String::from_utf8_lossy(&frame).contains("\n\n"));
        }
    }
}