//! 通知事件 broadcast channel
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1 (notification-service)
//!
//! 设计选择 (per 缺标比错标安全, 守门 #11):
//! - tokio::sync::broadcast 通道: 多 receiver fan-out, 每个 ws/sse 连接独立订阅
//! - 容量 1024 (per Sprint 1 默认); 慢 consumer 丢旧事件 (Lag) 而不是阻塞 producer
//! - 注意: WS endpoint /v1/notifications/ws 接收时, broadcast::Sender 通过 web::Data 共享
//!
//! MVP 简化 (per rustc 1.98 metadata bug + actix-ws 不在 Cargo.lock):
//! - WS 协议实现 deferred to Sprint 2
//! - 当前 /v1/notifications/ws 端点用 SSE (Server-Sent Events) 兼容 EventSource 客户端
//! - SSE 是单向推送 (server -> client), 与 broadcast::Receiver 天然契合
//! - 等 Sprint 2 引入 actix-ws 时, 把 stream 替换为 ws frame codec 即可

use crate::models::SseEvent;
use tokio::sync::broadcast;

/// 事件总线: 静态 broadcast 通道 (per app 进程内全局)
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<SseEvent>,
}

impl EventBus {
    /// 新建 EventBus, 容量 1024
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1024);
        Self { sender }
    }

    /// 订阅 (每个 ws/sse 连接一个 receiver)
    pub fn subscribe(&self) -> broadcast::Receiver<SseEvent> {
        self.sender.subscribe()
    }

    /// 发布通知创建事件
    pub fn publish_notification_created(&self, data: serde_json::Value) {
        let ev = SseEvent {
            event: "notification.created".to_string(),
            data,
        };
        // 忽略错误: 没有 receiver 是预期状态 (没有 ws 连接)
        let _ = self.sender.send(ev);
    }

    /// 发布通知已读事件
    pub fn publish_notification_read(&self, data: serde_json::Value) {
        let ev = SseEvent {
            event: "notification.read".to_string(),
            data,
        };
        let _ = self.sender.send(ev);
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_bus_default_creates_with_capacity() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        bus.publish_notification_created(serde_json::json!({"id": "test"}));
        let ev = rx.try_recv().expect("event should be received");
        assert_eq!(ev.event, "notification.created");
    }

    #[test]
    fn multiple_subscribers_each_receive() {
        let bus = EventBus::new();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();
        bus.publish_notification_created(serde_json::json!({"k": "v"}));
        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
    }
}
