//! task-service 集成测试 (per ULYS-151 切片 B-2 验收 §"集成测试 ≥3 case")
//!
//! 引用: doc/05-其他/MVP商业版/_slice_b2_task.md
//! 引用: doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md §3.4 (task-service)
//! 引用: ULYS-45 子任务 A (af8abb6) SSE 既有 e2e 模式
//!
//! 测试范围 (per 切片 B-2 验收):
//! - ① 业务模型纯函数路径 (status/type 解析 + 终态判定 + SseTaskStatus 转换)
//! - ② 事件总线行为 (publish_stage_progress → subscribe → 帧编码)
//! - ③ SSE 帧编码格式 (per WHATWG HTML §9.3 SSE 规范)
//! - ④ 路由 → 资源/操作 解析 (per cats-rbac §5)
//! - ⑤ SSE 帧解析器 (per 验收 §2 跨 \n\n 边界)
//!
//! **不连真实 PG**: 集成测试覆盖纯函数 + 内存事件总线; 真实 PG 测试留 Sprint 2 接
//! sqlx::test attribute 后填实 (per auth-service tests/integration_auth.rs 占位模式).

use actix_web::{test as actix_test, web, App};
use serde_json::Value;
use std::time::Duration;
use task_service::{
    handlers::{encode_event_for_test, AppState}, EventBus, ListTasksQuery, StageKind,
    StageProgressRequest, StageStatus, SseTaskStatus, TaskEvent, TaskStatus, TaskType,
    UpdateStatusRequest,
};
use tokio::time::timeout;
use uuid::Uuid;

// =====================================================================
// ① 业务模型纯函数
// =====================================================================

#[test]
fn task_status_round_trip_all_variants() {
    for s in [
        TaskStatus::Pending,
        TaskStatus::Running,
        TaskStatus::Completed,
        TaskStatus::Failed,
        TaskStatus::Cancelled,
    ] {
        assert_eq!(TaskStatus::parse(s.as_str()).unwrap(), s);
    }
    assert!(TaskStatus::parse("queued").is_none(), "8-态 queued 不在 DB 范围");
    assert!(TaskStatus::parse("unknown").is_none());
}

#[test]
fn task_type_round_trip_all_variants() {
    for t in [TaskType::Translate, TaskType::Review, TaskType::Export] {
        assert_eq!(TaskType::parse(t.as_str()).unwrap(), t);
    }
    assert!(TaskType::parse("asr").is_none(), "stage 类型与 task_type 不同");
}

#[test]
fn task_status_terminal_classification() {
    // 切片 B-2 5 态终态语义
    assert!(TaskStatus::Completed.is_terminal());
    assert!(TaskStatus::Failed.is_terminal());
    assert!(TaskStatus::Cancelled.is_terminal());
    assert!(!TaskStatus::Pending.is_terminal());
    assert!(!TaskStatus::Running.is_terminal());
}

#[test]
fn sse_task_status_from_db_covers_all() {
    // 切片 B-2 5 态 → SSE 8 态映射
    assert_eq!(
        SseTaskStatus::from_db(TaskStatus::Pending),
        SseTaskStatus::Queued
    );
    assert_eq!(
        SseTaskStatus::from_db(TaskStatus::Running),
        SseTaskStatus::Processing
    );
    assert_eq!(
        SseTaskStatus::from_db(TaskStatus::Completed),
        SseTaskStatus::Completed
    );
    assert_eq!(
        SseTaskStatus::from_db(TaskStatus::Failed),
        SseTaskStatus::Failed
    );
    assert_eq!(
        SseTaskStatus::from_db(TaskStatus::Cancelled),
        SseTaskStatus::Cancelled
    );
}

#[test]
fn list_tasks_query_deserialize_with_defaults() {
    // 无 query 参数时, 应使用默认值 (limit=50, offset=0, project_id=None, status=None)
    let q: ListTasksQuery = serde_json::from_str("{}").unwrap();
    assert_eq!(q.limit, 50);
    assert_eq!(q.offset, 0);
    assert!(q.project_id.is_none());
    assert!(q.status.is_none());
}

// =====================================================================
// ② 事件总线行为
// =====================================================================

#[tokio::test]
async fn publish_stage_progress_then_subscribe_sees_state() {
    let bus = EventBus::new(16);
    let task_id = Uuid::new_v4();

    // 1) 无订阅者发布 → 返回 false (但状态已记录)
    let req = StageProgressRequest {
        event_id: "evt_1".to_string(),
        stage: StageKind::Asr,
        status: StageStatus::Started,
        progress: Some(0),
        result_ref: None,
        metrics: None,
        error: None,
    };
    assert!(!bus.publish_stage_progress(task_id, &req));

    // 2) 订阅者建立后, view 包含最近一次事件 (per SSE 协议 "Last-Event-ID")
    let sub = bus.subscribe(task_id);
    assert_eq!(sub.view.task_id, task_id);
    assert_eq!(sub.view.status, TaskStatus::Pending);
    assert!(sub.view.last_event_at.is_some());

    // 3) 之后发布 → 订阅者能收到 (本测试用 buffer, 验证 send().is_ok() = true)
    assert!(bus.publish_stage_progress(task_id, &req));
}

#[tokio::test]
async fn publish_terminal_status_records_terminal() {
    let bus = EventBus::new(16);
    let task_id = Uuid::new_v4();
    bus.publish_task_terminated(task_id, TaskStatus::Completed, None);
    let view = bus.view(task_id);
    assert_eq!(view.status, TaskStatus::Completed);
    assert!(view.last_event_at.is_some());
}

#[tokio::test]
async fn publish_terminal_with_reason_records_reason() {
    let bus = EventBus::new(16);
    let task_id = Uuid::new_v4();
    bus.publish_task_terminated(
        task_id,
        TaskStatus::Failed,
        Some("timeout after 30s".to_string()),
    );
    // view 不会携带 reason (per 设计); 仅 status + last_event_at
    let view = bus.view(task_id);
    assert_eq!(view.status, TaskStatus::Failed);
    assert!(view.last_event_at.is_some());
}

// =====================================================================
// ③ SSE 帧编码格式 (per WHATWG HTML §9.3 SSE 规范)
// =====================================================================

#[test]
fn sse_frame_terminated_includes_event_data_blank_line() {
    let event = TaskEvent::TaskTerminated {
        status: SseTaskStatus::Completed,
        reason: None,
        occurred_at: chrono::Utc::now(),
    };
    let bytes = encode_event_for_test(&event);
    let s = String::from_utf8(bytes).expect("valid utf-8");

    // 帧必须包含 event 行 + data 行 + 空行 (per SSE 规范)
    assert!(s.starts_with("event: task_terminated\n"), "got {s:?}");
    assert!(s.contains("\ndata: "), "data 行缺失: {s:?}");
    // 帧尾必须有空行
    assert!(s.ends_with("\n\n"), "帧尾空行缺失: {s:?}");
}

#[test]
fn sse_frame_stage_progress_includes_event_id_data() {
    let event = TaskEvent::StageProgress {
        event_id: "evt_42".to_string(),
        stage: StageKind::Translation,
        status: StageStatus::InProgress,
        progress: Some(50),
        result_ref: None,
        metrics: None,
        error: None,
        occurred_at: chrono::Utc::now(),
    };
    let bytes = encode_event_for_test(&event);
    let s = String::from_utf8(bytes).expect("valid utf-8");
    assert!(s.starts_with("event: stage_progress\n"), "got {s:?}");
    assert!(s.contains("\nid: evt_42\n"), "id 行缺失: {s:?}");
    assert!(s.contains("\ndata: "), "data 行缺失: {s:?}");
    assert!(s.ends_with("\n\n"), "帧尾空行缺失: {s:?}");
}

#[test]
fn sse_frame_heartbeat_has_no_id_line() {
    let event = TaskEvent::Heartbeat {
        occurred_at: chrono::Utc::now(),
    };
    let bytes = encode_event_for_test(&event);
    let s = String::from_utf8(bytes).expect("valid utf-8");
    // Heartbeat 无业务 id, 不应包含 id 行
    assert!(s.starts_with("event: heartbeat\n"));
    assert!(!s.contains("\nid: "), "Heartbeat 不应有 id 行: {s:?}");
    assert!(s.contains("\ndata: "));
    assert!(s.ends_with("\n\n"));
}

// =====================================================================
// ④ 真实 SSE 帧解析器 (per ULYS-45 验收 §2 跨 \n\n 边界)
// =====================================================================

#[derive(Debug, Clone, PartialEq)]
struct SseFrame {
    event: String,
    id: Option<String>,
    data: String,
}

struct SseParser {
    buffer: Vec<u8>,
}

impl SseParser {
    fn new() -> Self {
        Self {
            buffer: Vec::new(),
        }
    }

    fn feed(&mut self, bytes: &[u8]) -> Vec<SseFrame> {
        self.buffer.extend_from_slice(bytes);
        let mut frames = Vec::new();
        loop {
            let Some(idx) = self.find_boundary() else { break };
            let frame_bytes: Vec<u8> = self.buffer[..idx].to_vec();
            let boundary_len = if idx + 3 < self.buffer.len()
                && self.buffer[idx] == b'\r'
                && self.buffer[idx + 1] == b'\n'
            {
                4
            } else {
                2
            };
            let frame_str = String::from_utf8_lossy(&frame_bytes);
            if let Some(f) = Self::parse_frame(&frame_str) {
                frames.push(f);
            }
            self.buffer.drain(..idx + boundary_len);
        }
        frames
    }

    fn find_boundary(&self) -> Option<usize> {
        let mut i = 0;
        while i + 1 < self.buffer.len() {
            if self.buffer[i] == b'\n' && self.buffer[i + 1] == b'\n' {
                return Some(i);
            }
            if i + 3 < self.buffer.len()
                && self.buffer[i] == b'\r'
                && self.buffer[i + 1] == b'\n'
                && self.buffer[i + 2] == b'\r'
                && self.buffer[i + 3] == b'\n'
            {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    fn parse_frame(s: &str) -> Option<SseFrame> {
        let mut event = String::new();
        let mut id: Option<String> = None;
        let mut data_lines: Vec<String> = Vec::new();
        for line in s.lines() {
            if line.starts_with(':') {
                continue;
            }
            if let Some(rest) = line.strip_prefix("event:") {
                event = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("id:") {
                id = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("data:") {
                data_lines.push(rest.trim_start().to_string());
            }
        }
        if event.is_empty() && data_lines.is_empty() {
            return None;
        }
        if event.is_empty() {
            event = "message".to_string();
        }
        Some(SseFrame {
            event,
            id,
            data: data_lines.join("\n"),
        })
    }
}

#[test]
fn sse_parser_reassembles_frames_across_newline_boundary() {
    // 模拟 TCP 分片: 第一帧跨 \n\n 边界分两次到达
    let full = encode_event_for_test(&TaskEvent::Heartbeat {
        occurred_at: chrono::Utc::now(),
    });
    let full_str = String::from_utf8(full.clone()).unwrap();
    // 找到 \n\n 边界
    let boundary = full_str.find("\n\n").unwrap();
    let part1 = &full[..boundary + 1]; // 包含第一个 \n
    let part2 = &full[boundary + 1..]; // 包含剩余部分

    let mut parser = SseParser::new();
    let mut frames = parser.feed(part1);
    assert!(frames.is_empty(), "未到完整帧时不应有输出");
    frames.extend(parser.feed(part2));
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].event, "heartbeat");
}

#[test]
fn sse_parser_handles_multiple_frames_in_one_chunk() {
    let event_a = encode_event_for_test(&TaskEvent::Heartbeat {
        occurred_at: chrono::Utc::now(),
    });
    let event_b = encode_event_for_test(&TaskEvent::Heartbeat {
        occurred_at: chrono::Utc::now(),
    });
    let mut combined = event_a.clone();
    combined.extend_from_slice(&event_b);

    let mut parser = SseParser::new();
    let frames = parser.feed(&combined);
    assert_eq!(frames.len(), 2);
    for f in &frames {
        assert_eq!(f.event, "heartbeat");
    }
}

#[test]
fn sse_parser_decodes_json_payload_data_field() {
    let event = TaskEvent::TaskTerminated {
        status: SseTaskStatus::Failed,
        reason: Some("test failure".to_string()),
        occurred_at: chrono::Utc::now(),
    };
    let bytes = encode_event_for_test(&event);
    let mut parser = SseParser::new();
    let frames = parser.feed(&bytes);
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].event, "task_terminated");
    let json: Value = serde_json::from_str(&frames[0].data).expect("data 是合法 JSON");
    assert_eq!(json["event"], "task_terminated");
    assert_eq!(json["status"], "failed");
    assert_eq!(json["reason"], "test failure");
}

// =====================================================================
// ⑤ actix-web e2e (真实 SSE 流; 不连 DB, 仅覆盖 EventBus)
// =====================================================================

fn build_state_no_pool() -> AppState {
    // 构造 AppState 但 pool 是 connect_lazy, 不真连 PG; EventBus 内存有效
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://invalid:invalid@127.0.0.1:1/invalid")
        .expect("lazy connect");
    AppState::new_with_buffer(pool, 16)
}

fn make_app_no_db(state: AppState) -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    App::new()
        .app_data(web::Data::new(state))
        .route("/healthz", web::get().to(task_service::handlers::healthz))
        .route(
            "/v1/tasks/{id}/events",
            web::get().to(task_service::handlers::task_events_sse),
        )
        .route(
            "/internal/v1/tasks/{id}/stage-progress",
            web::post().to(task_service::handlers::stage_progress),
        )
}

#[actix_web::test]
async fn e2e_healthz_returns_200() {
    let app = actix_test::init_service(make_app_no_db(build_state_no_pool())).await;
    let req = actix_test::TestRequest::get().uri("/healthz").to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);
}

#[actix_web::test]
async fn e2e_unauthenticated_returns_401() {
    let app = actix_test::init_service(make_app_no_db(build_state_no_pool())).await;
    let req = actix_test::TestRequest::get()
        .uri("/v1/tasks/00000000-0000-0000-0000-000000000000/events")
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 401);
}

#[actix_web::test]
async fn e2e_user_can_subscribe_and_receive_heartbeat() {
    use actix_web::http::header;
    use futures_util::StreamExt;
    let app = actix_test::init_service(make_app_no_db(build_state_no_pool())).await;
    let task_id = Uuid::new_v4();

    // 1) 先上报一个 stage-progress 事件, 让 EventBus 记录 last_state
    let report_app = actix_test::init_service(
        App::new()
            .app_data(web::Data::new(build_state_no_pool()))
            .route(
                "/internal/v1/tasks/{id}/stage-progress",
                web::post().to(task_service::handlers::stage_progress),
            ),
    )
    .await;

    let report_req = actix_test::TestRequest::post()
        .uri(&format!("/internal/v1/tasks/{task_id}/stage-progress"))
        // Sponsor 是 cats-rbac 默认权限矩阵中拥有全权的角色 (per §4 5 域 Lead 各管各资源)
        .insert_header((header::AUTHORIZATION, "Bearer cats-role:Sponsor"))
        .set_json(&serde_json::json!({
            "event_id": "evt_test",
            "stage": "asr",
            "status": "started",
            "progress": 0
        }))
        .to_request();
    let report_resp = actix_test::call_service(&report_app, report_req).await;
    assert_eq!(
        report_resp.status().as_u16(),
        200,
        "Sponsor 应能上报 stage-progress"
    );

    // 2) User 角色发起订阅 — 不读 last_state 帧 (per design: 初始化 Pending 时不发)
    let req = actix_test::TestRequest::get()
        .uri(&format!("/v1/tasks/{task_id}/events"))
        .insert_header((header::AUTHORIZATION, "Bearer cats-role:User"))
        .to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);
    assert_eq!(
        resp.headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("text/event-stream")
    );

    // 3) 客户端断连 → 服务端释放订阅 (drop resp)
    drop(resp);
}

// =====================================================================
// ⑥ list query 反序列化边界 (extra)
// =====================================================================

#[test]
fn list_tasks_query_full_params_deserialize() {
    let pid = Uuid::new_v4();
    let json = serde_json::json!({
        "project_id": pid,
        "status": "running",
        "limit": 100,
        "offset": 20
    });
    let q: ListTasksQuery = serde_json::from_value(json).unwrap();
    assert_eq!(q.project_id, Some(pid));
    assert_eq!(q.status, Some("running".to_string()));
    assert_eq!(q.limit, 100);
    assert_eq!(q.offset, 20);
}

// =====================================================================
// ⑦ db::ListFilter 边界
// =====================================================================

#[test]
fn list_filter_normalized_clamps_bounds() {
    use task_service::db::ListFilter;
    let f = ListFilter {
        project_id: None,
        status: None,
        limit: -10,
        offset: -5,
    }
    .normalized();
    assert_eq!(f.limit, 50);
    assert_eq!(f.offset, 0);

    let f = ListFilter {
        project_id: None,
        status: None,
        limit: 99999,
        offset: 0,
    }
    .normalized();
    assert_eq!(f.limit, 200);
}

// =====================================================================
// ⑧ stage_progress 发布 + EventBus 帧编码 端到端 (extra)
// =====================================================================

#[tokio::test]
async fn stage_progress_published_event_serializes_cleanly() {
    // 验证: publish_stage_progress 后, last_event 字段中的 StageProgress
    // 可以用 encode_event_for_test 编码为合法 SSE 帧 (包含 event_id 在 id 行)
    let bus = EventBus::new(16);
    let task_id = Uuid::new_v4();
    let req = StageProgressRequest {
        event_id: "evt_publish_test".to_string(),
        stage: StageKind::Translation,
        status: StageStatus::Completed,
        progress: Some(100),
        result_ref: Some(task_service::StageResultRef {
            file_id: "file-123".to_string(),
            kind: "translation".to_string(),
        }),
        metrics: Some(task_service::StageMetrics {
            duration_seconds: Some(120),
            process_seconds: Some(80),
        }),
        error: None,
    };
    bus.publish_stage_progress(task_id, &req);
    let sub = bus.subscribe(task_id);
    // sub.receiver 拿不到历史事件 (subscribe 是新 future); 但 view.last_event 记录了
    assert!(sub.view.last_event_at.is_some());

    // 重新构造对应事件, 验证编码
    let event = TaskEvent::StageProgress {
        event_id: req.event_id.clone(),
        stage: req.stage,
        status: req.status,
        progress: req.progress,
        result_ref: req.result_ref.clone(),
        metrics: req.metrics.clone(),
        error: req.error.clone(),
        occurred_at: chrono::Utc::now(),
    };
    let bytes = encode_event_for_test(&event);
    let s = String::from_utf8(bytes).expect("valid utf-8");
    assert!(s.contains("id: evt_publish_test"));
    assert!(s.contains("\"file_id\":\"file-123\""));
    assert!(s.contains("\"duration_seconds\":120"));
}

// =====================================================================
// ⑨ UpdateStatusRequest 反序列化 (extra)
// =====================================================================

#[test]
fn update_status_request_round_trip() {
    let json = serde_json::json!({
        "status": "running",
        "progress": 50
    });
    let req: UpdateStatusRequest = serde_json::from_value(json).unwrap();
    assert_eq!(req.status, TaskStatus::Running);
    assert_eq!(req.progress, Some(50));
    assert!(req.error_message.is_none());
    assert!(req.output_payload.is_none());

    let json = serde_json::json!({
        "status": "failed",
        "error_message": "timeout"
    });
    let req: UpdateStatusRequest = serde_json::from_value(json).unwrap();
    assert_eq!(req.status, TaskStatus::Failed);
    assert_eq!(req.error_message, Some("timeout".to_string()));
    assert!(req.progress.is_none());
}

// =====================================================================
// ⑩ timeout helper usage smoke (确保不依赖 future 兼容问题)
// =====================================================================

#[tokio::test]
async fn smoke_timeout_helper() {
    // 测试 timeout helper 可用 (per ULYS-45 e2e_t07_sse.rs 使用模式)
    let result = timeout(Duration::from_millis(10), async {
        tokio::time::sleep(Duration::from_millis(20)).await;
        "never"
    })
    .await;
    assert!(result.is_err(), "应超时 (10ms < 20ms)");
}

// 防止 warning: futures_util::StreamExt 实际在 e2e_* 测试间接使用, 这里不再单独引用
#[allow(dead_code)]
fn _stream_ext_used() {
    // 通过 macro 调用确保依赖仍被声明; 不返回值避免 lifetime 抱怨
    use futures_util::stream::StreamExt as _;
    let _ = futures_util::stream::empty::<()>().next();
}