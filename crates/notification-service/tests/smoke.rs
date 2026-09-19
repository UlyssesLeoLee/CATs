//! 单元测试 — notification-service

use cats_rbac::{Action, Resource, Role};
use cats_rbac::RbacChecker;
use notification_service::models::KafkaEvent;

#[test]
fn smoke_version() {
    assert!(notification_service::version().starts_with("0.1."));
    assert_eq!(notification_service::name(), "notification-service");
}

#[tokio::test]
async fn rbac_user_can_read_own_notifications() {
    let checker = RbacChecker::new();
    let res = checker.check_roles(&[Role::User], Resource::User, Action::Read).await;
    assert!(res.is_ok());
}

#[test]
fn kafka_event_serde_round_trip() {
    let ev = KafkaEvent {
        event_id: uuid::Uuid::new_v4(),
        org_id: uuid::Uuid::new_v4(),
        user_id: uuid::Uuid::new_v4(),
        event_type: "task.completed".to_string(),
        payload: serde_json::json!({"task_id": "abc"}),
    };
    let bytes = serde_json::to_vec(&ev).unwrap();
    let back: KafkaEvent = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(back.event_type, "task.completed");
    assert_eq!(back.event_id, ev.event_id);
}