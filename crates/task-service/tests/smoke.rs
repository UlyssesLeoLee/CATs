//! 单元测试 — task-service

use cats_common::{CatsError, ErrorCode};
use cats_rbac::{Action, Resource, Role};
use cats_rbac::RbacChecker;
use task_service::models::{CreateTaskRequest, TaskStatus, UpdateTaskStatusRequest};

#[test]
fn smoke_version() {
    assert!(task_service::version().starts_with("0.1."));
    assert_eq!(task_service::name(), "task-service");
}

#[test]
fn create_task_request_default_media_type() {
    let req = CreateTaskRequest {
        project_id: uuid::Uuid::new_v4(),
        org_id: uuid::Uuid::new_v4(),
        title: "translate chapter 1".to_string(),
        description: None,
        media_type: "document".to_string(),
        source_lang: Some("en-US".to_string()),
        target_lang: Some("ja-JP".to_string()),
        assigned_to: None,
    };
    assert_eq!(req.media_type, "document");
}

#[test]
fn update_status_request_parses_qa_violations() {
    let req = UpdateTaskStatusRequest {
        status: "qa_blocked".to_string(),
        qa_violations: Some(serde_json::json!([{"rule": "glossary_miss"}])),
    };
    assert_eq!(req.status, "qa_blocked");
    assert!(req.qa_violations.is_some());
}

#[tokio::test]
async fn rbac_user_can_read_task() {
    let checker = RbacChecker::new();
    let res = checker.check_roles(&[Role::User], Resource::Task, Action::Read).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn rbac_user_can_create_task() {
    let checker = RbacChecker::new();
    let res = checker
        .check_roles(&[Role::User], Resource::Task, Action::Create)
        .await;
    assert!(res.is_ok());
}

#[test]
fn state_machine_in_progress_to_completed() {
    assert!(TaskStatus::InProgress.can_transition_to(&TaskStatus::Completed));
}

#[test]
fn state_machine_rejects_skip_transitions() {
    assert!(!TaskStatus::Pending.can_transition_to(&TaskStatus::Completed));
    assert!(!TaskStatus::Pending.can_transition_to(&TaskStatus::QaBlocked));
}

#[test]
fn cats_error_qa_blocked_returns_422() {
    let e = CatsError::business(ErrorCode::QaBlocked, "term missing");
    assert_eq!(e.http_status(), 422);
    let body = e.to_error_body();
    assert_eq!(body.error, "qa_blocked");
}

#[test]
fn cats_error_conflict_returns_409() {
    let e = CatsError::conflict(ErrorCode::Conflict, "invalid transition");
    assert_eq!(e.http_status(), 409);
}