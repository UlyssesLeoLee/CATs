//! `notification-service` — 通知推送服务
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1
//! 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.1 (notification_db 接口契约 v1.0.0)
//! 引用: doc/05-其他/管理/CATs_错误码表_v1.0.md §3 (error enum)
//! 引用: doc/05-其他/管理/CATs_权限矩阵_v1.0.md §3 (RBAC)
//! 引用: ULYS-152 切片 B-4
//!
//! M1 业务实现 (per ULYS-152 切片 B-4 完成判据):
//! - GET    /v1/notifications              — 列表 (按 user_id + 分页)
//! - POST   /v1/notifications              — 创建 (内部 / 测试用)
//! - PATCH  /v1/notifications/{id}/read   — 标记已读
//! - GET    /v1/notifications/ws          — 实时推送 (SSE 实现, WS deferred to Sprint 2)
//! - GET    /healthz
//! - 8 逻辑库 notification_db (per Baseline §5.1) + pgcrypto extension
//!
//! 设计选择 (per 缺标比错标安全, 守门 #11):
//! - WS 实时推送用 SSE 替代: actix-ws / actix-web-actors 不在 Cargo.lock 中
//!   (避免 rustc 1.98 metadata bug 触发新依赖解析), SSE 与 broadcast::Receiver 契合
//! - broadcast channel 容量 1024, 慢 consumer 丢旧事件而非阻塞
//! - 通知创建走 HTTP POST (M1 简化); Sprint 2 改 Kafka consumer

pub mod db;
pub mod events;
pub mod handlers;
pub mod models;

pub use events::EventBus;
pub use models::{
    CreateNotificationRequest, ErrorBody, ListNotificationsQuery, MarkReadResponse,
    NotificationListResponse, NotificationRecord, NotificationResponse, SseEvent,
};

/// 当前 crate 语义版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 当前 crate 名称
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// 返回 crate 版本字符串
pub fn version() -> &'static str {
    VERSION
}

/// 返回 crate 名称
pub fn name() -> &'static str {
    NAME
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_semver_like() {
        let v = version();
        assert!(
            v.starts_with("0.1."),
            "version should start with '0.1.', got {v}"
        );
    }

    #[test]
    fn name_is_notification_service() {
        assert_eq!(name(), "notification-service");
    }
}