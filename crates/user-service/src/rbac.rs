//! user-service RBAC（2026-10-07 新增）
//!
//! 引用: doc/05-其他/管理/CATs_权限矩阵_v1.0.md
//! 引用: BACKEND_STATUS §4.1x（鉴权全景）
//!
//! **这个文件为什么存在**：在此之前 `user-service` 的三条业务路由
//! （`POST /v1/users`、`GET /v1/users/{id}`、`PUT /v1/users/{id}`）
//! **完全没有鉴权** —— 24 个 crate 里有 17 个如此，而 user-service 恰好
//! 管的是用户档案本身：任何能连到 8082 的调用方都能创建、读取、修改任意用户档案。
//!
//! 凭据契约与全仓其余服务一致：`X-Cats-User-Id` + `X-Cats-Roles`，
//! 由边缘 envoy 在**验签 JWT 之后**注入；客户端自带的同名头会被
//! route 上的 `request_headers_to_remove` 剥离（lint 规则 16 守着这一点）。
//!
//! **本文件只做资源 × 操作级别的鉴权**。行级（"只能读自己的档案"）**没有**
//! 实现：权限矩阵里 `Role::User` 对 `Resource::User × Read` 是有权限的，
//! 因此任何一个已登录用户都能读任意用户的档案。行级归属属于设计决策，
//! 不在本轮自行发明，记入 §4.1u 待拍板。

use crate::models::ErrorBody;
use actix_web::http::StatusCode;
use actix_web::HttpRequest;
use cats_rbac::{Action, RbacChecker, Resource, Role};

/// 抽取到的调用者身份（与另外 6 个服务同名字段，便于对照）
#[derive(Debug, Clone, Default)]
pub struct AuthContext {
    pub user_id: Option<uuid::Uuid>,
    pub roles: Vec<Role>,
}

impl AuthContext {
    pub fn is_authenticated(&self) -> bool {
        !self.roles.is_empty() && !self.roles.contains(&Role::Guest)
    }
}

/// 从请求头提取用户角色（**全仓唯一凭据契约**）
///
/// 直接复用 `cats_rbac::service_helpers::extract_user_id_and_roles`，
/// 不再各写一份角色名映射。
pub fn extract_user_roles(req: &HttpRequest) -> AuthContext {
    match cats_rbac::service_helpers::extract_user_id_and_roles(req) {
        Ok((user_id, roles)) => AuthContext {
            user_id: Some(user_id),
            roles,
        },
        Err(_) => AuthContext::default(),
    }
}

/// 路径 + 方法 → (资源, 操作)
pub fn route_to_resource_action(path: &str, method: &str) -> Option<(Resource, Action)> {
    let normalized = path.split('?').next().unwrap_or(path).trim_end_matches('/');
    match (normalized, method) {
        ("/v1/users", "POST") => Some((Resource::User, Action::Create)),
        ("/v1/users/{id}", "GET") => Some((Resource::User, Action::Read)),
        ("/v1/users/{id}", "PUT") => Some((Resource::User, Action::Update)),
        _ => None,
    }
}

/// RBAC 闸门。未通过时返回可直接回给调用方的响应体。
pub async fn enforce(
    checker: &RbacChecker,
    req: &HttpRequest,
    path: &str,
    method: &str,
) -> Result<AuthContext, (StatusCode, ErrorBody)> {
    let auth = extract_user_roles(req);

    if !auth.is_authenticated() {
        return Err((
            StatusCode::UNAUTHORIZED,
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
                StatusCode::FORBIDDEN,
                ErrorBody {
                    error: "route_not_rbac_mapped".to_string(),
                    message: format!("path {path} method {method} not RBAC-mapped"),
                    detail: None,
                },
            ))
        }
    };

    checker
        .check_roles(&auth.roles, resource, action)
        .await
        .map_err(|err| {
            (
                StatusCode::FORBIDDEN,
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

    const UID: &str = "00000000-0000-0000-0000-000000000001";

    fn req_with(role: &str) -> actix_web::HttpRequest {
        TestRequest::default()
            .insert_header(("X-Cats-User-Id", UID))
            .insert_header(("X-Cats-Roles", role))
            .to_http_request()
    }

    #[test]
    fn extracts_user_id_and_roles() {
        let req = req_with("User");
        let auth = extract_user_roles(&req);
        assert_eq!(auth.roles, vec![Role::User]);
        assert_eq!(auth.user_id.map(|u| u.to_string()), Some(UID.to_string()));
    }

    #[test]
    fn missing_headers_is_anonymous() {
        let req = TestRequest::default().to_http_request();
        let auth = extract_user_roles(&req);
        assert!(auth.roles.is_empty());
        assert!(!auth.is_authenticated());
    }

    /// 旧契约必须彻底失效：只带 `Authorization` 的一律匿名。
    #[test]
    fn legacy_cats_role_authorization_is_not_a_credential() {
        let req = TestRequest::default()
            .insert_header(("Authorization", "Bearer cats-role:Sponsor"))
            .to_http_request();
        let auth = extract_user_roles(&req);
        assert!(
            !auth.is_authenticated(),
            "旧契约一旦重新生效，就等于允许客户端自填 Sponsor"
        );
    }

    #[test]
    fn guest_is_not_authenticated() {
        let req = req_with("Guest");
        assert!(!extract_user_roles(&req).is_authenticated());
    }

    #[test]
    fn routes_map_to_user_resource() {
        assert_eq!(
            route_to_resource_action("/v1/users", "POST"),
            Some((Resource::User, Action::Create))
        );
        assert_eq!(
            route_to_resource_action("/v1/users/{id}", "GET"),
            Some((Resource::User, Action::Read))
        );
        assert_eq!(
            route_to_resource_action("/v1/users/{id}", "PUT"),
            Some((Resource::User, Action::Update))
        );
        assert_eq!(route_to_resource_action("/healthz", "GET"), None);
    }

    #[tokio::test]
    async fn anonymous_is_401() {
        let checker = RbacChecker::new();
        let req = TestRequest::default().to_http_request();
        let (status, body) = enforce(&checker, &req, "/v1/users/{id}", "GET")
            .await
            .expect_err("匿名必须被拒");
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.error, "missing_authorization");
    }

    #[tokio::test]
    async fn plain_user_cannot_create_users() {
        let checker = RbacChecker::new();
        let req = req_with("User");
        let (status, body) = enforce(&checker, &req, "/v1/users", "POST")
            .await
            .expect_err("User 在矩阵里没有 User:Create");
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body.error, "operation_not_permitted");
    }

    #[tokio::test]
    async fn plain_user_can_read_user_resource() {
        // 如实记录当前矩阵语义：User 对 Resource::User 有 Read。
        // **行级归属未实现**（见本文件顶部说明），所以这条是"已知且被钉住"的行为，
        // 不是"认为它对"。将来若补上行级校验，这条用例必须一起改。
        let checker = RbacChecker::new();
        let req = req_with("User");
        assert!(enforce(&checker, &req, "/v1/users/{id}", "GET")
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn unmapped_route_is_403() {
        let checker = RbacChecker::new();
        let req = req_with("Sponsor");
        let (status, body) = enforce(&checker, &req, "/v1/users/{id}", "DELETE")
            .await
            .expect_err("未映射的路由不该放行");
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body.error, "route_not_rbac_mapped");
    }
}
