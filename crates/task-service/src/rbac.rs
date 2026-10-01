//! task-service RBAC 中间件集成 (per T-03 权限矩阵 + 切片 B-2)
//!
//! 引用: doc/05-其他/管理/CATs_权限矩阵_v1.0.md (commit 03dbede defd2c6 cherry-pick)
//! 引用: cats-rbac crate (per §7 集成说明 + §4 RbacChecker)
//! 引用: doc/02-基础设计/接口设计/CATs_接口设计书_v2.0.md §3.5 (RBAC)
//!
//! 设计选择 (per 缺标比错标安全):
//! - 本切片不实施完整 actix-web `Transform` middleware (per cats-rbac §7 留 5 域 Lead 真人到位)
//! - 改为 inline 在每个 handler 入口调 `enforce()` → 简洁, 0 隐式行为
//! - 用户角色从 `Authorization: Bearer <jwt>` 解码 (per auth-service Claims 格式)
//!   → M1 阶段: 接受 `Bearer cats-role:<role>` 简化格式 (per ULYS-45 阶段二)
//!   → 生产路径 JWT 校验留 Sprint 2 接 auth-service JWKS 时落地
//! - RBAC 错误统一映射到 ErrorBody (per 错误码表 v1.0 §3.3/§3.7)
//!
//! 路由 → 资源映射 (per cats-rbac::Resource):
//! - GET/POST/PATCH/DELETE /v1/tasks/* → Resource::Task
//! - GET /v1/tasks/{id}/events → Resource::Task (SSE 也是 task 资源)
//! - 内部 POST /internal/v1/tasks/{id}/stage-progress → Resource::Task
//!   (per 内部路由约定, 鉴权由 mTLS / 服务账号保证; 切片 B-2 仅做 role 检查兜底)

use crate::models::ErrorBody;
use actix_web::http::header;
use actix_web::HttpRequest;
use cats_rbac::{Action, RbacChecker, RbacError, Resource, Role};
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
/// `Authorization: Bearer cats-role:User` → roles = [Role::User]
/// `Authorization: Bearer cats-role:User,QualityLead` → roles = [User, QualityLead]
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
    // 仅接受 Bearer scheme
    let token = match s
        .strip_prefix("Bearer ")
        .or_else(|| s.strip_prefix("bearer "))
    {
        Some(t) => t.trim(),
        None => return AuthContext::default(),
    };
    // M1 简化模式: cats-role:<comma-list>
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
    AuthContext {
        user_id: None, // M1 简化: 不解 user_id
        roles,
    }
}

fn parse_role(s: &str) -> Option<Role> {
    match s {
        "Sponsor" => Some(Role::Sponsor),
        "ArchitectLead" => Some(Role::ArchitectLead),
        "RustLead" => Some(Role::RustLead),
        "DatabaseLead" => Some(Role::DatabaseLead),
        "QualityLead" => Some(Role::QualityLead),
        "ProjectLead" => Some(Role::ProjectLead),
        "SRELead" => Some(Role::SRELead),
        "User" => Some(Role::User),
        "Guest" => Some(Role::Guest),
        _ => None,
    }
}

/// 路由路径 + HTTP method → (Resource, Action) 解析 (per cats-rbac §5 path/method 解析)
///
/// 注意: cats-rbac 内置的 parse_path_to_resource 仅识别 `/v1/tasks` 前缀,
/// 不区分 `internal/v1/tasks` 内部路由. 本 helper 显式覆盖内部路由.
pub fn route_to_resource_action(path: &str, method: &str) -> Option<(Resource, Action)> {
    // 内部路由显式映射 (内部上报, mTLS 兜底; 切片 B-2 仅做 role 检查)
    if path.starts_with("/internal/v1/tasks") {
        return match method.to_uppercase().as_str() {
            "POST" => Some((Resource::Task, Action::Create)),
            "GET" => Some((Resource::Task, Action::Read)),
            _ => None,
        };
    }
    // /v1/tasks/* → Task 资源
    if path.starts_with("/v1/tasks") {
        let action = match method.to_uppercase().as_str() {
            "GET" | "HEAD" => Action::Read,
            "POST" => Action::Create,
            "PUT" | "PATCH" => Action::Update,
            "DELETE" => Action::Delete,
            _ => return None,
        };
        return Some((Resource::Task, action));
    }
    None
}

/// RBAC 校验失败时构造 ErrorBody + HTTP 状态码
pub fn rbac_error_to_response(err: &RbacError) -> (actix_web::http::StatusCode, ErrorBody) {
    use actix_web::http::StatusCode;
    let status = StatusCode::from_u16(err.http_status())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let body = match err {
        RbacError::Unauthenticated => ErrorBody::new(
            "missing_authorization",
            "Authorization header missing or invalid",
        ),
        RbacError::InvalidCredentials => {
            ErrorBody::new("invalid_credentials", "invalid token")
        }
        RbacError::UserInactive => ErrorBody::new("user_inactive", "user is inactive"),
        RbacError::Forbidden {
            required_role,
            actual_role,
        } => ErrorBody::new(
            "operation_not_permitted",
            format!(
                "operation requires {required_role:?}, actual {actual_role:?}"
            ),
        ),
        RbacError::NotFound(p) => ErrorBody::new("resource_not_found", p.clone()),
    };
    (status, body)
}

/// 同步 helper: 在 handler 内 inline 检查 RBAC
///
/// 返回 `Ok(())` = 通过; `Err((status, body))` = 失败, 由调用方转 HttpResponse
pub async fn enforce(
    checker: &Arc<RbacChecker>,
    req: &HttpRequest,
    path: &str,
    method: &str,
) -> Result<(), (actix_web::http::StatusCode, ErrorBody)> {
    let auth = extract_user_roles(req);
    let (resource, action) = route_to_resource_action(path, method)
        .ok_or_else(|| {
            rbac_error_to_response(&RbacError::NotFound(path.to_string()))
        })?;
    match checker.check_roles(&auth.roles, resource, action).await {
        Ok(()) => Ok(()),
        Err(e) => Err(rbac_error_to_response(&e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    #[test]
    fn extract_single_role() {
        let req = TestRequest::default()
            .insert_header((header::AUTHORIZATION, "Bearer cats-role:User"))
            .to_http_request();
        let auth = extract_user_roles(&req);
        assert_eq!(auth.roles, vec![Role::User]);
        assert!(auth.is_authenticated());
    }

    #[test]
    fn extract_multiple_roles() {
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
    fn route_to_resource_action_v1_tasks() {
        assert_eq!(
            route_to_resource_action("/v1/tasks", "GET"),
            Some((Resource::Task, Action::Read))
        );
        assert_eq!(
            route_to_resource_action("/v1/tasks", "POST"),
            Some((Resource::Task, Action::Create))
        );
        assert_eq!(
            route_to_resource_action("/v1/tasks/abc/events", "GET"),
            Some((Resource::Task, Action::Read))
        );
        assert_eq!(
            route_to_resource_action("/v1/tasks/abc/status", "PATCH"),
            Some((Resource::Task, Action::Update))
        );
    }

    #[test]
    fn route_to_resource_action_internal_tasks() {
        assert_eq!(
            route_to_resource_action("/internal/v1/tasks/abc/stage-progress", "POST"),
            Some((Resource::Task, Action::Create))
        );
    }

    #[tokio::test]
    async fn enforce_anonymous_is_rejected() {
        let checker = Arc::new(RbacChecker::new());
        let req = TestRequest::default().to_http_request();
        let result = enforce(&checker, &req, "/v1/tasks", "GET").await;
        let (status, body) = result.expect_err("anonymous should be rejected");
        assert_eq!(status, actix_web::http::StatusCode::UNAUTHORIZED);
        assert_eq!(body.error, "missing_authorization");
    }

    #[tokio::test]
    async fn enforce_user_can_read_tasks() {
        let checker = Arc::new(RbacChecker::new());
        let req = TestRequest::default()
            .insert_header((header::AUTHORIZATION, "Bearer cats-role:User"))
            .to_http_request();
        let result = enforce(&checker, &req, "/v1/tasks", "GET").await;
        assert!(result.is_ok(), "User should be able to read tasks");
    }

    #[tokio::test]
    async fn enforce_user_cannot_delete_tasks() {
        let checker = Arc::new(RbacChecker::new());
        let req = TestRequest::default()
            .insert_header((header::AUTHORIZATION, "Bearer cats-role:User"))
            .to_http_request();
        let result = enforce(&checker, &req, "/v1/tasks/abc", "DELETE").await;
        let (status, body) = result.expect_err("User cannot delete tasks");
        assert_eq!(status, actix_web::http::StatusCode::FORBIDDEN);
        assert_eq!(body.error, "operation_not_permitted");
    }
}