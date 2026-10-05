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
        CatsError::business(
            ErrorCode::InvalidToken,
            "X-Cats-User-Id is not a valid UUID",
        )
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
///
/// ## 错误优先级 (2026-10-05 修)
///
/// 循环里只保留**最后一个**错误, 于是拒绝时的状态码取决于角色在
/// `X-Cats-Roles` 里的**顺序**:
///
/// | 角色集合 | 逐个判定 | 留下的错误 | HTTP |
/// |---|---|---|---|
/// | `User,Guest` | User→Forbidden, Guest→Unauthenticated | Unauthenticated | **401** |
/// | `Guest,User` | Guest→Unauthenticated, User→Forbidden | Forbidden | **403** |
///
/// 同一组角色、仅顺序不同就给出不同状态码。而 401 意味着"未登录", 对一个
/// 其实已登录、只是权限不够的调用方是**误导性的** —— 会把人引去查认证链
/// 而不是权限矩阵。
///
/// 现在改为：**只要有任一角色被判 `Forbidden`, 就一律按 403 处理**。
/// `Forbidden` 表示"身份已确认但权限不足", 比 `Unauthenticated` 更具体,
/// 应当优先。纯粹全 Guest 的调用方仍然拿 401（那确实是未登录）。
pub async fn require_roles(
    checker: &RbacChecker,
    allowed_roles: &[Role],
    resource: Resource,
    action: Action,
) -> Result<(), CatsError> {
    let mut last_err: Option<RbacError> = None;
    let mut saw_forbidden = false;
    for role in allowed_roles {
        match checker.check_roles(&[*role], resource, action).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                if matches!(e, RbacError::Forbidden { .. }) {
                    saw_forbidden = true;
                }
                last_err = Some(e);
            }
        }
    }
    let actual = allowed_roles.first().copied().unwrap_or(Role::Guest);
    let required = match action {
        Action::Approve => Role::Sponsor,
        Action::Deploy => Role::SRELead,
        Action::Audit => Role::DatabaseLead,
        _ => Role::User,
    };
    let denied = || {
        CatsError::business(
            ErrorCode::OperationNotPermitted,
            format!(
                "operation_not_permitted: required_role={required:?} actual_role={actual:?} resource={resource:?} action={action:?}"
            ),
        )
    };
    // 取 last_err 的具体 reason (Forbidden / Unauthenticated / ...), 保持 RbacError 语义
    Err(match last_err {
        Some(RbacError::Forbidden {
            required_role: _,
            actual_role: _,
        }) => denied(),
        // `saw_forbidden` 守卫是这个修复的全部: 出现过 Forbidden 就走 403,
        // 不管它是不是最后一个错误。
        Some(RbacError::Unauthenticated)
        | Some(RbacError::InvalidCredentials)
        | Some(RbacError::UserInactive)
            if !saw_forbidden =>
        {
            CatsError::business(ErrorCode::Unauthorized, "unauthenticated")
        }
        Some(RbacError::Unauthenticated)
        | Some(RbacError::InvalidCredentials)
        | Some(RbacError::UserInactive) => denied(),
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
        // 2026-10-05：这个用例名字里写着 403，却只断言了 `is_err()` ——
        // 名字表达的意图没有被验证。补上真正的断言。
        assert_eq!(
            res.unwrap_err().code(),
            ErrorCode::OperationNotPermitted,
            "空角色集合应判 403 operation_not_permitted"
        );
    }

    /// 2026-10-05：拒绝时的状态码不应依赖角色在集合里的**顺序**。
    #[tokio::test]
    async fn require_roles_denied_status_is_order_independent() {
        let checker = RbacChecker::new();
        // User 对 Audit/Read 不够格（Forbidden），Guest 单独判是 Unauthenticated。
        let user_then_guest = require_roles(
            &checker,
            &[Role::User, Role::Guest],
            Resource::Audit,
            Action::Read,
        )
        .await;
        let guest_then_user = require_roles(
            &checker,
            &[Role::Guest, Role::User],
            Resource::Audit,
            Action::Read,
        )
        .await;
        let code_a = user_then_guest.unwrap_err().code();
        let code_b = guest_then_user.unwrap_err().code();
        assert_eq!(
            code_a,
            ErrorCode::OperationNotPermitted,
            "[User, Guest] 应判 403（已登录但权限不足）"
        );
        assert_eq!(code_b, code_a, "同一组角色仅顺序不同，状态码必须相同");
    }

    /// 纯 Guest 仍然拿 401 —— 那确实是未登录，不该被 403 吞掉。
    #[tokio::test]
    async fn require_roles_pure_guest_stays_401() {
        let checker = RbacChecker::new();
        let res = require_roles(&checker, &[Role::Guest], Resource::Audit, Action::Read).await;
        assert_eq!(
            res.unwrap_err().code(),
            ErrorCode::Unauthorized,
            "只有一个 Guest 时没有任何角色被判 Forbidden，应保留 401 未登录语义"
        );
    }
}
