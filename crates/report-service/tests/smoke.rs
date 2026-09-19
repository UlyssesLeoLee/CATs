//! 单元测试 — report-service

use cats_rbac::{Action, Resource, Role};
use cats_rbac::RbacChecker;

#[test]
fn smoke_version() {
    assert!(report_service::version().starts_with("0.1."));
    assert_eq!(report_service::name(), "report-service");
}

#[tokio::test]
async fn rbac_quality_lead_can_read_reports() {
    let checker = RbacChecker::new();
    let res = checker
        .check_roles(&[Role::QualityLead], Resource::Report, Action::Read)
        .await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn rbac_user_cannot_read_reports() {
    let checker = RbacChecker::new();
    let res = checker
        .check_roles(&[Role::User], Resource::Report, Action::Read)
        .await;
    // MVP 默认权限矩阵 User 不应能 Read Report
    assert!(res.is_err());
}