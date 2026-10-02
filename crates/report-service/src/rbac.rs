//! report-service RBAC 中间件集成 (per ULYS-153 切片 C-1)
//!
//! 引用: doc/05-其他/管理/CATs_权限矩阵_v1.0.md (commit 03dbede)
//! 引用: cats-rbac crate (per §7 集成说明)
//! 引用: ULYS-153 切片 C-1 §"3 endpoint 挂 RBAC"
//!
//! 设计选择 (per 缺标比错标安全):
//! - inline 在每个 handler 入口调 `enforce()` (per cats-rbac §7)
//! - 用户角色从 `Authorization: Bearer *** 解码 (per auth-service Claims 格式)
//!   → M1 阶段: 接受 `Bearer cats-role:<role>` 简化格式
//! - RBAC 错误统一映射到 ErrorBody (per 错误码表 v1.0 §3.3/§3.7)
//!
//! 路由 → 资源映射 (per cats-rbac::Resource):
//! - GET /v1/reports/usage?org_id=&from=&to=            → Resource::Report, Action::Read
//! - GET /v1/reports/translation-volume?project_id=... → Resource::Report, Action::Read
//! - GET /v1/reports/audit-summary?workspace_id=...    → Resource::Report, Action::Read

use crate::models::ErrorBody;
use actix_web::http::header;
use actix_web::HttpRequest;
use cats_rbac::{Action, RbacChecker, Resource, Role};
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct AuthContext {
    pub user_id: Option<uuid::Uuid>,
    pub roles: Vec<Role>,
}

impl AuthContext {
    pub fn is_authenticated(&self) -> bool {
        !self.roles.is_empty() && !self.roles.contains(&Role::Guest)
    }

    pub fn principal_id(&self) -> String {
        self.user_id
            .map(|u| u.to_string())
            .unwrap_or_else(|| "anonymous".to_string())
    }
}

pub fn extract_user_roles(req: &HttpRequest) -> AuthContext {
    let header_val = match req.headers().get(header::AUTHORIZATION) {
        Some(h) => h,
        None => return AuthContext::default(),
    };
    let s = match header_val.to_str() {
        Ok(s) => s,
        Err(_) => return AuthContext::default(),
    };
    let token = match s
        .strip_prefix("Bearer ")
        .or_else(|| s.strip_prefix("bearer "))
    {
        Some(t) => t.trim(),
        None => return AuthContext::default(),
    };
    let body = match token.strip_prefix("cats-role:") {
        Some(b) => b,
        None => return AuthContext::default(),
    };

    let mut roles = Vec::new();
    for part in body.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(role) = parse_role(part) {
            roles.push(role);
        }
    }

    let user_id = body
        .split(':')
        .next()
        .and_then(|s| uuid::Uuid::parse_str(s.trim()).ok());

    AuthContext { user_id, roles }
}

fn parse_role(s: &str) -> Option<Role> {
    Some(match s {
        "Sponsor" => Role::Sponsor,
        "ArchitectLead" => Role::ArchitectLead,
        "RustLead" => Role::RustLead,
        "DatabaseLead" => Role::DatabaseLead,
        "QualityLead" => Role::QualityLead,
        "ProjectLead" => Role::ProjectLead,
        "SRELead" => Role::SRELead,
        "User" => Role::User,
        "Guest" => Role::Guest,
        _ => return None,
    })
}

pub fn route_to_resource_action(path: &str, method: &str) -> Option<(Resource, Action)> {
    let normalized = path.split('?').next().unwrap_or(path).trim_end_matches('/');

    match (normalized, method) {
        ("/v1/reports/usage", "GET") => Some((Resource::Report, Action::Read)),
        ("/v1/reports/translation-volume", "GET") => Some((Resource::Report, Action::Read)),
        ("/v1/reports/audit-summary", "GET") => Some((Resource::Report, Action::Read)),
        _ => None,
    }
}

pub async fn enforce(
    checker: &Arc<RbacChecker>,
    req: &HttpRequest,
    path: &str,
    method: &str,
) -> Result<AuthContext, (actix_web::http::StatusCode, ErrorBody)> {
    let auth = extract_user_roles(req);

    if !auth.is_authenticated() {
        return Err((
            actix_web::http::StatusCode::UNAUTHORIZED,
            ErrorBody {
                error: "missing_authorization".to_string(),
                message: "Authorization header with Bearer cats-role:<roles> required".to_string(),
                detail: None,
            },
        ));
    }

    let (resource, action) = match route_to_resource_action(path, method) {
        Some(ra) => ra,
        None => {
            return Err((
                actix_web::http::StatusCode::FORBIDDEN,
                ErrorBody {
                    error: "route_not_rbac_mapped".to_string(),
                    message: format!("path {path} method {method} not RBAC-mapped"),
                    detail: None,
                },
            ));
        }
    };

    checker
        .check_roles(&auth.roles, resource, action)
        .await
        .map_err(|err| {
            (
                actix_web::http::StatusCode::FORBIDDEN,
                ErrorBody {
                    error: "operation_not_permitted".to_string(),
                    message: format!("{resource:?} {action:?} not permitted"),
                    detail: Some(err.to_string()),
                },
            )
        })?;

    Ok(auth)
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    #[test]
    fn extract_user_role_single() {
        let req = TestRequest::default()
            .insert_header((header::AUTHORIZATION, "Bearer cats-role:User"))
            .to_http_request();
        let auth = extract_user_roles(&req);
        assert_eq!(auth.roles, vec![Role::User]);
    }

    #[test]
    fn extract_missing_header_is_anonymous() {
        let req = TestRequest::default().to_http_request();
        let auth = extract_user_roles(&req);
        assert!(auth.roles.is_empty());
        assert!(!auth.is_authenticated());
    }

    #[test]
    fn route_to_resource_action_v1_reports() {
        assert_eq!(
            route_to_resource_action("/v1/reports/usage", "GET"),
            Some((Resource::Report, Action::Read))
        );
        assert_eq!(
            route_to_resource_action("/v1/reports/translation-volume", "GET"),
            Some((Resource::Report, Action::Read))
        );
        assert_eq!(
            route_to_resource_action("/v1/reports/audit-summary", "GET"),
            Some((Resource::Report, Action::Read))
        );
    }

    #[test]
    fn route_unknown_returns_none() {
        assert_eq!(route_to_resource_action("/v1/unknown", "GET"), None);
        assert_eq!(route_to_resource_action("/healthz", "GET"), None);
    }

    #[tokio::test]
    async fn enforce_anonymous_is_rejected() {
        let checker = Arc::new(RbacChecker::new());
        let req = TestRequest::default().to_http_request();
        let result = enforce(&checker, &req, "/v1/reports/usage", "GET").await;
        let (status, body) = result.expect_err("anonymous should be rejected");
        assert_eq!(status, actix_web::http::StatusCode::UNAUTHORIZED);
        assert_eq!(body.error, "missing_authorization");
    }

    #[tokio::test]
    async fn enforce_user_cannot_read_reports() {
        // GET /v1/reports/usage 映射到 (Resource::Report, Action::Read)。
        // cats-rbac 矩阵里 Report 归 DatabaseLead（同时也在 SRE Lead 的资源集内），
        // Role::User 的 Read 集只有 User / Task / Project / File / Translation，
        // 所以普通 User 被拒是正确行为。
        //
        // 原测试 enforce_user_can_read_reports 断言 result.is_ok()，等于要求把
        // Report 加进 User 权限集 —— 为让测试通过而放宽授权矩阵方向是反的。
        // 与 notification-service 的同类测试一并修正。
        let checker = Arc::new(RbacChecker::new());

        let user_req = TestRequest::default()
            .insert_header((header::AUTHORIZATION, "Bearer cats-role:User"))
            .to_http_request();
        let (status, body) = enforce(&checker, &user_req, "/v1/reports/usage", "GET")
            .await
            .expect_err("User must not read Report-scoped resources");
        assert_eq!(status, actix_web::http::StatusCode::FORBIDDEN);
        assert_eq!(body.error, "operation_not_permitted");

        // 对照：确实持有 Report/Read 的角色应放行
        let dba_req = TestRequest::default()
            .insert_header((header::AUTHORIZATION, "Bearer cats-role:DatabaseLead"))
            .to_http_request();
        assert!(
            enforce(&checker, &dba_req, "/v1/reports/usage", "GET")
                .await
                .is_ok(),
            "DatabaseLead holds Report/Read and must be allowed"
        );
    }
}
