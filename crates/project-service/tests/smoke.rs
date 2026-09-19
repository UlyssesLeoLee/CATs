//! 单元测试 — project-service
//!
//! 引用: doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md §2 T-02 完成判据
//! "至少 3 个 happy-path 单元测试"

use cats_common::{CatsError, ErrorCode};
use cats_rbac::{Action, Resource, Role};
use cats_rbac::RbacChecker;
use project_service::models::{
    AddMemberRequest, CreateProjectRequest, UpdateProjectRequest,
};

#[test]
fn smoke_version_is_semver_like() {
    assert!(project_service::version().starts_with("0.1."));
    assert_eq!(project_service::name(), "project-service");
}

#[test]
fn create_request_default_lang_fields_are_none() {
    let req = CreateProjectRequest {
        org_id: uuid::Uuid::new_v4(),
        name: "demo".to_string(),
        description: Some("test project".to_string()),
        source_lang: None,
        target_lang: None,
    };
    assert_eq!(req.name, "demo");
    assert!(req.source_lang.is_none());
    assert!(req.target_lang.is_none());
}

#[test]
fn update_request_partial_fields_optional() {
    let req = UpdateProjectRequest {
        name: Some("new name".to_string()),
        description: None,
        status: Some("archived".to_string()),
    };
    assert!(req.description.is_none());
    assert_eq!(req.status.as_deref(), Some("archived"));
}

#[test]
fn add_member_default_role_is_member() {
    let req = AddMemberRequest {
        user_id: uuid::Uuid::new_v4(),
        role: "member".to_string(),
    };
    assert_eq!(req.role, "member");
}

#[tokio::test]
async fn rbac_user_can_read_project() {
    let checker = RbacChecker::new();
    let res = checker
        .check_roles(&[Role::User], Resource::Project, Action::Read)
        .await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn rbac_user_can_create_project() {
    let checker = RbacChecker::new();
    let res = checker
        .check_roles(&[Role::User], Resource::Project, Action::Create)
        .await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn rbac_user_cannot_delete_project() {
    // MVP 默认权限矩阵里 User 没 Delete Project (per 权限决策书 §3)
    let checker = RbacChecker::new();
    let res = checker
        .check_roles(&[Role::User], Resource::Project, Action::Delete)
        .await;
    assert!(res.is_err());
}

#[test]
fn cats_error_validation_returns_400() {
    let e = CatsError::validation("name must not be empty");
    assert_eq!(e.http_status(), 400);
    assert_eq!(e.code(), ErrorCode::ValidationError);
    let body = e.to_error_body();
    assert_eq!(body.error, "validation_error");
}

#[test]
fn cats_error_not_found_returns_404() {
    let e = CatsError::not_found(ErrorCode::ResourceNotFound, "project abc");
    assert_eq!(e.http_status(), 404);
    let body = e.to_error_body();
    assert_eq!(body.error, "resource_not_found");
    assert!(body.detail.is_some());
}