//! project-service RBAC 中间件集成 (per T-03 权限矩阵 + 切片 B-1)
//!
//! 引用: doc/05-其他/管理/CATs_权限矩阵_v1.0.md (commit 03dbede)
//! 引用: cats-rbac crate (per §7 集成说明 + §4 RbacChecker)
//! 引用: ULYS-150 切片 B-1 §"RBAC 中间件挂在每个 endpoint"
//!
//! 设计选择 (per 缺标比错标安全):
//! - inline 在每个 handler 入口调 `enforce()` (per cats-rbac §7 留 5 域 Lead 真人到位)
//! - 用户角色从 `Authorization: Bearer *** 解码 (per auth-service Claims 格式)
//!   → 2026-10-07 起统一为 `X-Cats-User-Id` + `X-Cats-Roles`（边缘注入） (per task-service rbac 同款)
//!   → 生产路径 JWT 校验留 Sprint 2 接 auth-service JWKS
//! - RBAC 错误统一映射到 ErrorBody (per 错误码表 v1.0 §3.3/§3.7)
//!
//! 路由 → 资源映射 (per cats-rbac::Resource):
//! - POST   /v1/projects                  → Resource::Project, Action::Create
//! - GET    /v1/projects                  → Resource::Project, Action::Read
//! - GET    /v1/projects/{id}             → Resource::Project, Action::Read
//! - PATCH  /v1/projects/{id}             → Resource::Project, Action::Update
//! - DELETE /v1/projects/{id}             → Resource::Project, Action::Delete

use crate::models::ErrorBody;
use actix_web::HttpRequest;
use cats_rbac::{Action, RbacChecker, Resource, Role};
use std::sync::Arc;

/// 当前请求的角色集 (由 extract_user_roles 解析)
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

/// 路径 + HTTP 方法 → (Resource, Action) 映射
pub fn route_to_resource_action(path: &str, method: &str) -> Option<(Resource, Action)> {
    // 移除 query string 与尾斜杠归一化
    let normalized = path.split('?').next().unwrap_or(path).trim_end_matches('/');

    match (normalized, method) {
        ("/v1/projects", "POST") => Some((Resource::Project, Action::Create)),
        ("/v1/projects", "GET") => Some((Resource::Project, Action::Read)),
        (p, "GET") if p.starts_with("/v1/projects/") => Some((Resource::Project, Action::Read)),
        (p, "PATCH") if p.starts_with("/v1/projects/") => Some((Resource::Project, Action::Update)),
        (p, "DELETE") if p.starts_with("/v1/projects/") => {
            Some((Resource::Project, Action::Delete))
        }
        _ => None,
    }
}

/// RBAC 强制检查 (inline helper for handlers)
///
/// 失败: 返回 (HTTP status, ErrorBody)
/// 成功: 返回 Ok(AuthContext)
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
            // 路径未映射到资源: 视为 forbidden (per 守门 #11 缺标比错标, 不放行未知路径)
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
    fn extract_user_role_multiple() {
        let req = TestRequest::default()
            .insert_header(("X-Cats-User-Id", uuid::Uuid::new_v4().to_string()))
            .insert_header(("X-Cats-Roles", "User,QualityLead"))
            .to_http_request();
        let auth = extract_user_roles(&req);
        assert_eq!(auth.roles, vec![Role::User, Role::QualityLead]);
    }

    #[test]
    fn extract_missing_header_is_anonymous() {
        let req = TestRequest::default().to_http_request();
        let auth = extract_user_roles(&req);
        assert!(auth.roles.is_empty());
        assert!(!auth.is_authenticated());
    }

    #[test]
    fn extract_non_bearer_scheme_is_anonymous() {
        let req = TestRequest::default()
            .insert_header(("Authorization", "Basic abc"))
            .to_http_request();
        let auth = extract_user_roles(&req);
        assert!(auth.roles.is_empty());
    }

    #[test]
    fn extract_non_cats_role_token_is_anonymous() {
        let req = TestRequest::default()
            .insert_header(("Authorization", "Bearer random-jwt-token"))
            .to_http_request();
        let auth = extract_user_roles(&req);
        assert!(auth.roles.is_empty());
    }

    #[test]
    fn route_to_resource_action_v1_projects() {
        assert_eq!(
            route_to_resource_action("/v1/projects", "POST"),
            Some((Resource::Project, Action::Create))
        );
        assert_eq!(
            route_to_resource_action("/v1/projects", "GET"),
            Some((Resource::Project, Action::Read))
        );
        assert_eq!(
            route_to_resource_action("/v1/projects/abc", "GET"),
            Some((Resource::Project, Action::Read))
        );
        assert_eq!(
            route_to_resource_action("/v1/projects/abc", "PATCH"),
            Some((Resource::Project, Action::Update))
        );
        assert_eq!(
            route_to_resource_action("/v1/projects/abc", "DELETE"),
            Some((Resource::Project, Action::Delete))
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
        let result = enforce(&checker, &req, "/v1/projects", "GET").await;
        let (status, body) = result.expect_err("anonymous should be rejected");
        assert_eq!(status, actix_web::http::StatusCode::UNAUTHORIZED);
        assert_eq!(body.error, "missing_authorization");
    }

    #[tokio::test]
    async fn enforce_user_can_read_projects() {
        let checker = Arc::new(RbacChecker::new());
        let req = TestRequest::default()
            .insert_header(("X-Cats-User-Id", uuid::Uuid::new_v4().to_string()))
            .insert_header(("X-Cats-Roles", "User"))
            .to_http_request();
        let result = enforce(&checker, &req, "/v1/projects", "GET").await;
        assert!(result.is_ok(), "User should be able to read projects");
    }

    #[tokio::test]
    async fn enforce_user_cannot_delete_projects() {
        let checker = Arc::new(RbacChecker::new());
        let req = TestRequest::default()
            .insert_header(("X-Cats-User-Id", uuid::Uuid::new_v4().to_string()))
            .insert_header(("X-Cats-Roles", "User"))
            .to_http_request();
        let result = enforce(&checker, &req, "/v1/projects/abc", "DELETE").await;
        let (status, _body) = result.expect_err("User cannot delete projects");
        assert_eq!(status, actix_web::http::StatusCode::FORBIDDEN);
    }
}
