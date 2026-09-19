//! `cats_rbac::service_helpers` — 16 service crate 的 RBAC 集成 helper
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §14 (RBAC 阶段一/二)
//! 引用: doc/05-其他/管理/CATs_权限决策书_v1.0.md §3 (角色 × 资源 × 动作)
//!
//! ## 设计要点
//!
//! - 不实现 actix-web Transform middleware (per README §7, 留给 Rust Lead 自定义)
//! - 提供 handler-side helper: 每个 service handler 在路由处理前调 `extract_user_id_and_roles` + `require_roles`
//! - `RbacChecker` 仍由 `RbacChecker::new()` 构造 (默认权限矩阵 per §3 权限决策书)
//!
//! ## 用法
//!
//! ```ignore
//! use cats_rbac::service_helpers::{extract_user_id_and_roles, require_roles};
//! use cats_rbac::{Role, Resource, Action};
//!
//! async fn handler(req: HttpRequest, checker: web::Data<RbacChecker>) -> impl Responder {
//!     let (user_id, _roles) = extract_user_id_and_roles(&req)?;
//!     require_roles(&checker, &[Role::User, Role::Sponsor], Resource::Project, Action::Read).await?;
//!     // ... 业务逻辑
//! }
//! ```
//!
//! ## Gateway 透传 (per 接口设计 §1.2)
//!
//! Envoy Gateway 解析 JWT claims, 注入以下 headers:
//! - `X-Cats-User-Id`: user UUID 字符串
//! - `X-Cats-Org-Id`: org UUID 字符串
//! - `X-Cats-Roles`: comma-separated role names (`"User"`, `"Sponsor"`, `"ArchitectLead"` ...)
//!
//! 业务服务信任该 header (Gateway 已签名校验, 服务不再重复验签, per §1.2).
//! 业务服务只做业务级权限校验 (跨 org 隔离 / 项目成员关系).

use actix_web::HttpRequest;
use uuid::Uuid;

use cats_common::{CatsError, ErrorCode};

use crate::{Action, RbacChecker, RbacError, Resource, Role};

/// 从 `X-Cats-User-Id` / `X-Cats-Roles` header 抽取 user_id + roles
///
/// - 缺 `X-Cats-User-Id` → 401 missing_authorization
/// - UUID 格式错误 → 401 invalid_token
/// - 缺 `X-Cats-Roles` 或 解析为空 → 401 unauthorized
pub fn extract_user_id_and_roles(req: &HttpRequest) -> Result<(Uuid, Vec<Role>), CatsError> {
    let user_id = req
        .headers()
        .get("X-Cats-User-Id")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            CatsError::business(
                ErrorCode::MissingAuthorization,
                "missing X-Cats-User-Id header",
            )
        })?;
    let user_id = Uuid::parse_str(user_id).map_err(|_| {
        CatsError::business(ErrorCode::InvalidToken, "X-Cats-User-Id is not a valid UUID")
    })?;

    let roles = req
        .headers()
        .get("X-Cats-Roles")
        .and_then(|h| h.to_str().ok())
        .map(|s| {
            s.split(',')
                .filter_map(|r| parse_role(r.trim()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if roles.is_empty() {
        return Err(CatsError::business(
            ErrorCode::Unauthorized,
            "no roles provided in X-Cats-Roles header",
        ));
    }

    Ok((user_id, roles))
}

/// 角色字符串 → `Role` enum
///
/// 兼容 PascalCase (`"User"`) + snake_case (`"user"`).
pub fn parse_role(s: &str) -> Option<Role> {
    match s {
        "Sponsor" | "sponsor" => Some(Role::Sponsor),
        "ArchitectLead" | "architect_lead" => Some(Role::ArchitectLead),
        "RustLead" | "rust_lead" => Some(Role::RustLead),
        "DatabaseLead" | "database_lead" => Some(Role::DatabaseLead),
        "QualityLead" | "quality_lead" => Some(Role::QualityLead),
        "ProjectLead" | "project_lead" => Some(Role::ProjectLead),
        "SRELead" | "sre_lead" => Some(Role::SRELead),
        "User" | "user" => Some(Role::User),
        "Guest" | "guest" => Some(Role::Guest),
        _ => None,
    }
}

/// 校验 `allowed_roles` 中是否有任一角色满足 (resource, action)
///
/// 调用方传"允许的角色列表", helper 顺序检查每个角色是否有权, 任一通过即 Ok.
pub async fn require_roles(
    checker: &RbacChecker,
    allowed_roles: &[Role],
    resource: Resource,
    action: Action,
) -> Result<(), CatsError> {
    let mut last_err: Option<RbacError> = None;
    for role in allowed_roles {
        match checker.check_roles(&[*role], resource, action).await {
            Ok(()) => return Ok(()),
            Err(e) => last_err = Some(e),
        }
    }
    let actual = allowed_roles.first().copied().unwrap_or(Role::Guest);
    let required = match action {
        Action::Approve => Role::Sponsor,
        Action::Deploy => Role::SRELead,
        Action::Audit => Role::DatabaseLead,
        _ => Role::User,
    };
    // 取 last_err 的具体 reason (Forbidden / Unauthenticated / ...), 保持 RbacError 语义
    Err(match last_err {
        Some(RbacError::Forbidden { required_role: _, actual_role: _ }) => CatsError::business(
            ErrorCode::OperationNotPermitted,
            format!(
                "operation_not_permitted: required_role={required:?} actual_role={actual:?} resource={resource:?} action={action:?}"
            ),
        ),
        Some(RbacError::Unauthenticated) | Some(RbacError::InvalidCredentials)
        | Some(RbacError::UserInactive) => {
            CatsError::business(ErrorCode::Unauthorized, "unauthenticated")
        }
        Some(RbacError::NotFound(_)) => CatsError::not_found(ErrorCode::ResourceNotFound, "rbac"),
        None => CatsError::business(
            ErrorCode::OperationNotPermitted,
            format!("no role provided for {resource:?} {action:?}"),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_role_supports_pascal_and_snake_case() {
        assert_eq!(parse_role("Sponsor"), Some(Role::Sponsor));
        assert_eq!(parse_role("sponsor"), Some(Role::Sponsor));
        assert_eq!(parse_role("User"), Some(Role::User));
        assert_eq!(parse_role("user"), Some(Role::User));
        assert_eq!(parse_role("unknown"), None);
        assert_eq!(parse_role(""), None);
    }

    #[tokio::test]
    async fn require_roles_user_allowed_for_project_read() {
        let checker = RbacChecker::new();
        let res = require_roles(&checker, &[Role::User], Resource::Project, Action::Read).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn require_roles_user_cannot_delete_project() {
        let checker = RbacChecker::new();
        let res = require_roles(&checker, &[Role::User], Resource::Project, Action::Delete).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn require_roles_empty_allowed_returns_403() {
        let checker = RbacChecker::new();
        let res = require_roles(&checker, &[], Resource::Project, Action::Read).await;
        assert!(res.is_err());
    }
}