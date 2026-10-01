//! 任务事件总线 — 进程内 pub-sub (per ULYS-151 切片 B-2)
//!
//! 引用: doc/05-其他/MVP商业版/_slice_b2_task.md
//! 引用: ULYS-45 子任务 A (af8abb6) 既有实现 (复用其内存事件总线设计)
//!
//! 设计要点 (per ULYS-45 复用 + 切片 B-2 改造):
//! - 在 M1 阶段, 事件总线为进程内内存结构 (无 Kafka 依赖, 不引入新外部依赖)
//!   → 真实跨服务事件总线 (Kafka `task.events` topic) 留 Sprint 2 T-任务落地
//!   → per Sprint 1 §6.9 接口设计书 v2.0 升版路径
//! - 订阅者通过 `tokio::sync::broadcast` 接收事件; 接收失败 = 客户端断连, 自动清理
//! - 任务终态 (TaskTerminated) 触发后, 所有订阅者收到终止事件并自动 drop 流
//! - 心跳 (Heartbeat) 由独立 ticker 驱动, 与上报事件解耦
//! - 切片 B-2 在 ULYS-45 基础上新增: DB 状态变更后必须 publish_task_terminated
//!   → `publish_status_change` 接受 DB TaskStatus + reason, 转换为 SseTaskStatus
//! - `last_event_id` 缓存供"先发最近一次进度"使用 (per SSE 协议最佳实践)
//!
//! 设计决策 (per 缺标比错标安全):
//! - broadcast buffer = 1024: 足够 M1 测试覆盖 (一任务最多几百个进度事件)
//! - 单进程假设: 不假设多副本部署, 跨副本事件分发留 Kafka 阶段

use crate::models::{StageProgressRequest, TaskEvent, TaskStatus};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use uuid::Uuid;

/// 任务事件总线 (进程内单例, AppState 持有 Arc<EventBus>)
pub struct EventBus {
    /// task_id → broadcast::Sender<TaskEvent>
    senders: Mutex<HashMap<Uuid, broadcast::Sender<TaskEvent>>>,
    /// task_id → 最近一次状态 (用于"建立连接后立即知道当前状态")
    last_state: Mutex<HashMap<Uuid, TaskState>>,
    /// broadcast buffer 大小 (足够 M1 测试覆盖, 不假设长生命周期高扇出)
    buffer: usize,
}

/// 任务最近一次状态 (供 SSE 客户端订阅建立时立即知道终态)
#[derive(Debug, Clone)]
pub struct TaskState {
    /// DB 5 态 (切片 B-2 范围); SSE 事件 publish 时再转换为 SseTaskStatus 8 态
    pub status: TaskStatus,
    pub last_event: Option<TaskEvent>,
}

impl EventBus {
    /// 创建新的事件总线
    pub fn new(buffer: usize) -> Self {
        Self {
            senders: Mutex::new(HashMap::new()),
            last_state: Mutex::new(HashMap::new()),
            buffer,
        }
    }

    /// 获取 (惰性创建) task_id 对应的 sender
    fn sender_for(&self, task_id: Uuid) -> broadcast::Sender<TaskEvent> {
        let mut senders = self
            .senders
            .lock()
            .expect("EventBus senders mutex poisoned");
        senders
            .entry(task_id)
            .or_insert_with(|| broadcast::channel(self.buffer).0)
            .clone()
    }

    /// 发布阶段进度事件 (内部 API 触发, per 接口设计书 §3.4)
    ///
    /// 返回 `true` = 新事件被投递 (有订阅者); `false` = 无订阅者 (但状态已记录)
    pub fn publish_stage_progress(&self, task_id: Uuid, req: &StageProgressRequest) -> bool {
        let occurred_at = Utc::now();
        let event = TaskEvent::StageProgress {
            event_id: req.event_id.clone(),
            stage: req.stage,
            status: req.status,
            progress: req.progress,
            result_ref: req.result_ref.clone(),
            metrics: req.metrics.clone(),
            error: req.error.clone(),
            occurred_at,
        };

        // 维护最近一次状态 (即使无订阅者也记录)
        let mut last = self
            .last_state
            .lock()
            .expect("EventBus last_state mutex poisoned");
        let entry = last.entry(task_id).or_insert(TaskState {
            status: TaskStatus::Pending,
            last_event: None,
        });
        entry.last_event = Some(event.clone());

        // 广播给订阅者 (send 返回 Err = 无订阅者, 不视为错误)
        let sender = self.sender_for(task_id);
        drop(last); // 早释放锁, 避免在 send 时持锁
        sender.send(event).is_ok()
    }

    /// 发布任务终态事件 (per 接口设计书 §3.4 状态机终态, 切片 B-2 范围)
    ///
    /// 切片 B-2 新增入口: 接受 DB 5 态 `TaskStatus`, 内部转换为 SSE 8 态 `SseTaskStatus`
    ///
    /// 返回 `true` = 有订阅者接收; `false` = 无订阅者
    pub fn publish_task_terminated(
        &self,
        task_id: Uuid,
        db_status: TaskStatus,
        reason: Option<String>,
    ) -> bool {
        debug_assert!(
            db_status.is_terminal(),
            "publish_task_terminated called with non-terminal status {db_status:?}"
        );
        let sse_status = crate::models::SseTaskStatus::from_db(db_status);
        let event = TaskEvent::TaskTerminated {
            status: sse_status,
            reason,
            occurred_at: Utc::now(),
        };

        let mut last = self
            .last_state
            .lock()
            .expect("EventBus last_state mutex poisoned");
        let entry = last.entry(task_id).or_insert(TaskState {
            status: db_status,
            last_event: None,
        });
        entry.status = db_status;
        entry.last_event = Some(event.clone());
        drop(last);

        let sender = self.sender_for(task_id);
        sender.send(event).is_ok()
    }

    /// 订阅任务事件 (返回 Receiver + 当前状态视图)
    ///
    /// 客户端断连时 Receiver 自动 drop, broadcast::Sender 会在所有 Receiver drop 后
    /// 由下一轮 GC 清理 (本 M1 阶段不显式清理, 不阻塞功能正确性).
    pub fn subscribe(&self, task_id: Uuid) -> Subscription {
        let sender = self.sender_for(task_id);
        let receiver = sender.subscribe();
        let view = self.view(task_id);
        Subscription { receiver, view }
    }

    /// 任务当前视图 (供客户端立即知道当前状态)
    pub fn view(&self, task_id: Uuid) -> TaskView {
        let last = self
            .last_state
            .lock()
            .expect("EventBus last_state mutex poisoned");
        let sender_count = {
            let senders = self
                .senders
                .lock()
                .expect("EventBus senders mutex poisoned");
            senders
                .get(&task_id)
                .map(|s| s.receiver_count())
                .unwrap_or(0)
        };
        let (status, last_event_at) = last
            .get(&task_id)
            .map(|s| (s.status, s.last_event.as_ref().map(event_occurred_at)))
            .unwrap_or((TaskStatus::Pending, None));
        TaskView {
            task_id,
            status,
            last_event_at,
            subscriber_count: sender_count,
        }
    }

    /// 测试/调试用: 列出当前所有 task 的视图
    #[cfg(any(test, debug_assertions))]
    pub fn snapshot(&self) -> Vec<TaskView> {
        let last = self
            .last_state
            .lock()
            .expect("EventBus last_state mutex poisoned");
        last.keys().map(|id| self.view(*id)).collect()
    }
}

/// SSE 订阅句柄
pub struct Subscription {
    pub receiver: broadcast::Receiver<TaskEvent>,
    pub view: TaskView,
}

/// 当前任务视图 (供 SSE 订阅建立时"先发最近一次状态")
#[derive(Debug, Clone)]
pub struct TaskView {
    pub task_id: Uuid,
    pub status: TaskStatus,
    pub last_event_at: Option<chrono::DateTime<chrono::Utc>>,
    pub subscriber_count: usize,
}

/// 从 TaskEvent 中提取 occurred_at (避免在多处重复 match)
fn event_occurred_at(e: &TaskEvent) -> chrono::DateTime<chrono::Utc> {
    match e {
        TaskEvent::StageProgress { occurred_at, .. } => *occurred_at,
        TaskEvent::TaskTerminated { occurred_at, .. } => *occurred_at,
        TaskEvent::Heartbeat { occurred_at } => *occurred_at,
    }
}

/// 把 EventBus 包成 Arc, 供 actix-web AppState 持有
pub type SharedEventBus = Arc<EventBus>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{StageKind, StageStatus};

    #[test]
    fn publish_stage_progress_then_subscribe_returns_event() {
        let bus = EventBus::new(16);
        let task_id = Uuid::new_v4();

        // 先发布事件 (无订阅者)
        let req = StageProgressRequest {
            event_id: "evt_1".to_string(),
            stage: StageKind::Asr,
            status: StageStatus::Started,
            progress: Some(0),
            result_ref: None,
            metrics: None,
            error: None,
        };
        assert!(!bus.publish_stage_progress(task_id, &req)); // 无订阅者 -> false

        // 订阅应能取到最近一次事件 (per SSE 协议 "Last-Event-ID" 重连)
        let sub = bus.subscribe(task_id);
        assert_eq!(sub.view.task_id, task_id);
        assert_eq!(sub.view.status, TaskStatus::Pending);
        // last_event 应记录了上面的 StageProgress
        assert!(sub.view.last_event_at.is_some());
    }

    #[test]
    fn publish_task_terminated_records_terminal_status() {
        let bus = EventBus::new(16);
        let task_id = Uuid::new_v4();

        // 推进到 running (DB 5 态)
        let req = StageProgressRequest {
            event_id: "evt_1".to_string(),
            stage: StageKind::Asr,
            status: StageStatus::InProgress,
            progress: Some(50),
            result_ref: None,
            metrics: None,
            error: None,
        };
        bus.publish_stage_progress(task_id, &req);

        // 终态: completed
        bus.publish_task_terminated(task_id, TaskStatus::Completed, None);

        let view = bus.view(task_id);
        assert_eq!(view.status, TaskStatus::Completed);
        assert!(view.last_event_at.is_some());
    }

    #[test]
    #[should_panic(expected = "non-terminal status")]
    fn debug_assert_terminal_only_on_terminated() {
        // 调用 publish_task_terminated 用 Pending (非终态), 应触发 debug_assert 失败
        let bus = EventBus::new(16);
        let task_id = Uuid::new_v4();
        let _ = bus.publish_task_terminated(task_id, TaskStatus::Pending, None);
    }
}