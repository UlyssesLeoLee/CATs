//! report-service RBAC 中间件集成 (per ULYS-153 切片 C-1)
//!
//! 引用: doc/05-其他/管理/CATs_权限矩阵_v1.0.md (commit 03dbede)
//! 引用: cats-rbac crate (per §7 集成说明)
//! 引用: ULYS-153 切片 C-1 §"3 endpoint 挂 RBAC"
//!
//! 设计选择 (per 缺标比错标安全):
//! - inline 在每个 handler 入口调 `enforce()` (per cats-rbac §7)
//! - 用户角色从 `Authorization: Bearer *** 解码 (per auth-service Claims 格式)
//!   → 2026-10-07 起统一为 `X-Cats-User-Id` + `X-Cats-Roles`（边缘注入）
//! - RBAC 错误统一映射到 ErrorBody (per 错误码表 v1.0 §3.3/§3.7)
//!
//! 路由 → 资源映射 (per cats-rbac::Resource):
//! - GET /v1/reports/usage?org_id=&from=&to=            → Resource::Report, Action::Read
//! - GET /v1/reports/translation-volume?project_id=... → Resource::Report, Action::Read
//! - GET /v1/reports/audit-summary?workspace_id=...    → Resource::Report, Action::Read

use crate::models::ErrorBody;
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

    // 2026-10-07: `user_id` 现在由 `X-Cats-User-Id` 真实填充（旧实现解析的是
    // 剥掉 `cats-role:` 前缀之后的字符串，所以恒为 None）。
}

/// 从请求头提取用户角色（**全仓唯一凭据契约**）
///
/// 2026-10-07 契约统一：凭据一律来自 `X-Cats-User-Id` + `X-Cats-Roles`，
/// 由边缘代理（envoy `jwt_authn`）在**验签 JWT 之后**注入。
///
/// 原来这里认的是 `Authorization: Bearer cats-role:<roles>`。那个格式
/// **没有任何一方在生产**（`deploy/envoy-mvp.yaml` 里既没有 `jwt_authn`
/// 也没有 `request_headers_to_add`），所以它要么恒不命中、要么被调用方
/// 自填成 Sponsor —— 与 audit / worker 走的也是两条不同的路径。
///
/// 现在直接复用 `cats_rbac::service_helpers::extract_user_id_and_roles`，
/// 七个服务走同一套解析，不再各写一份角色名映射。
///
/// 失败: 返回空 roles (未认证)
pub fn extract_user_roles(req: &HttpRequest) -> AuthContext {
    match cats_rbac::service_helpers::extract_user_id_and_roles(req) {
        Ok((user_id, roles)) => AuthContext {
            user_id: Some(user_id),
            roles,
        },
        Err(_) => AuthContext::default(),
    }
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
                // 该 message 里的两个头是由边缘代理（envoy jwt_authn）在**验签 JWT 之后**注入的；
                // 客户端自带的同名头会被 route 上的 request_headers_to_remove 剥离。
                message: "X-Cats-User-Id and X-Cats-Roles headers required".to_string(),
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
            .insert_header(("X-Cats-User-Id", uuid::Uuid::new_v4().to_string()))
            .insert_header(("X-Cats-Roles", "User"))
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
            .insert_header(("X-Cats-User-Id", uuid::Uuid::new_v4().to_string()))
            .insert_header(("X-Cats-Roles", "User"))
            .to_http_request();
        let (status, body) = enforce(&checker, &user_req, "/v1/reports/usage", "GET")
            .await
            .expect_err("User must not read Report-scoped resources");
        assert_eq!(status, actix_web::http::StatusCode::FORBIDDEN);
        assert_eq!(body.error, "operation_not_permitted");

        // 对照：确实持有 Report/Read 的角色应放行
        let dba_req = TestRequest::default()
            .insert_header(("X-Cats-User-Id", uuid::Uuid::new_v4().to_string()))
            .insert_header(("X-Cats-Roles", "DatabaseLead"))
            .to_http_request();
        assert!(
            enforce(&checker, &dba_req, "/v1/reports/usage", "GET")
                .await
                .is_ok(),
            "DatabaseLead holds Report/Read and must be allowed"
        );
    }
}
