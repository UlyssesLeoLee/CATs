//! project-service RBAC 中间件集成 (per T-03 权限矩阵 + 切片 B-1)
//!
//! 引用: doc/05-其他/管理/CATs_权限矩阵_v1.0.md (commit 03dbede)
//! 引用: cats-rbac crate (per §7 集成说明 + §4 RbacChecker)
//! 引用: ULYS-150 切片 B-1 §"RBAC 中间件挂在每个 endpoint"
//!
//! 设计选择 (per 缺标比错标安全):
//! - inline 在每个 handler 入口调 `enforce()` (per cats-rbac §7 留 5 域 Lead 真人到位)
//! - 用户角色从 `Authorization: Bearer *** 解码 (per auth-service Claims 格式)
//!   → M1 阶段: 接受 `Bearer cats-role:<role>` 简化格式 (per task-service rbac 同款)
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
use actix_web::http::header;
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

    pub fn principal_id(&self) -> String {
        self.user_id
            .map(|u| u.to_string())
            .unwrap_or_else(|| "anonymous".to_string())
    }
}

/// 从请求头提取用户角色 (M1 简化模式)
/// `Authorization: Bearer cats-role:<role-list>`
///
/// 失败: 返回空 roles (未认证)
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

    // 提取 user_id (M1 简化: `cats-role:<uuid>:<role-list>` 形式可选)
    let user_id = body
        .split(':')
        .next()
        .and_then(|s| uuid::Uuid::parse_str(s.trim()).ok());

    AuthContext { user_id, roles }
}

/// 解析单个角色字符串为 Role enum
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

/// 路径 + HTTP 方法 → (Resource, Action) 映射
pub fn route_to_resource_action(path: &str, method: &str) -> Option<(Resource, Action)> {
    // 移除 query string 与尾斜杠归一化
    let normalized = path.split('?').next().unwrap_or(path).trim_end_matches('/');

    match (normalized, method) {
        ("/v1/notifications", "POST") => Some((Resource::Alert, Action::Create)),
        ("/v1/notifications", "GET") => Some((Resource::Alert, Action::Read)),
        (p, "GET") if p.starts_with("/v1/notifications/") => {
            Some((Resource::Alert, Action::Read))
        }
        (p, "PATCH") if p.starts_with("/v1/notifications/") => {
            Some((Resource::Alert, Action::Update))
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
                message: "Authorization header with Bearer cats-role:<roles> required".to_string(),
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
            .insert_header((header::AUTHORIZATION, "Bearer cats-role:User"))
            .to_http_request();
        let auth = extract_user_roles(&req);
        assert_eq!(auth.roles, vec![Role::User]);
    }

    #[test]
    fn extract_user_role_multiple() {
        let req = TestRequest::default()
            .insert_header((
                header::AUTHORIZATION,
                "Bearer cats-role:User,QualityLead",
            ))
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
            .insert_header((header::AUTHORIZATION, "Basic abc"))
            .to_http_request();
        let auth = extract_user_roles(&req);
        assert!(auth.roles.is_empty());
    }

    #[test]
    fn extract_non_cats_role_token_is_anonymous() {
        let req = TestRequest::default()
            .insert_header((header::AUTHORIZATION, "Bearer random-jwt-token"))
            .to_http_request();
        let auth = extract_user_roles(&req);
        assert!(auth.roles.is_empty());
    }

    #[test]
    fn route_to_resource_action_v1_notifications() {
        assert_eq!(
            route_to_resource_action("/v1/notifications", "POST"),
            Some((Resource::Alert, Action::Create))
        );
        assert_eq!(
            route_to_resource_action("/v1/notifications", "GET"),
            Some((Resource::Alert, Action::Read))
        );
        assert_eq!(
            route_to_resource_action("/v1/notifications/abc/read", "PATCH"),
            Some((Resource::Alert, Action::Update))
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
        let result = enforce(&checker, &req, "/v1/notifications", "GET").await;
        let (status, body) = result.expect_err("anonymous should be rejected");
        assert_eq!(status, actix_web::http::StatusCode::UNAUTHORIZED);
        assert_eq!(body.error, "missing_authorization");
    }

    #[tokio::test]
    async fn enforce_user_can_read_notifications() {
        let checker = Arc::new(RbacChecker::new());
        let req = TestRequest::default()
            .insert_header((header::AUTHORIZATION, "Bearer cats-role:User"))
            .to_http_request();
        let result = enforce(&checker, &req, "/v1/notifications", "GET").await;
        assert!(result.is_ok(), "User should be able to read notifications");
    }
}