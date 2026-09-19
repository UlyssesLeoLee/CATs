//! 单元测试 — file-service

use cats_common::ErrorCode;
use cats_rbac::{Action, Resource, Role};
use cats_rbac::RbacChecker;

#[test]
fn smoke_version() {
    assert!(file_service::version().starts_with("0.1."));
    assert_eq!(file_service::name(), "file-service");
}

#[tokio::test]
async fn rbac_user_can_create_file() {
    let checker = RbacChecker::new();
    let res = checker
        .check_roles(&[Role::User], Resource::File, Action::Create)
        .await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn rbac_user_can_read_file() {
    let checker = RbacChecker::new();
    let res = checker
        .check_roles(&[Role::User], Resource::File, Action::Read)
        .await;
    assert!(res.is_ok());
}

#[test]
fn cats_error_validation_returns_400() {
    let e = cats_common::CatsError::business(ErrorCode::InvalidRequest, "missing header");
    assert_eq!(e.http_status(), 400);
}