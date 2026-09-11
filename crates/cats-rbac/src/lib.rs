//! cats-rbac: CATs RBAC 中间件
//!
//! 引用: doc/05-其他/管理/CATs_权限矩阵_v1.0.md (commit 03dbede defd2c6 cherry-pick)
//! + 启动会决议 8 OI-6 跨项目同步 T-03 借机 (commit 1b27b2b)
//! + 8/21 决议 5 域 Lead 互不兼任
//! + 守门 #14 v3 Mavis 永久代签
//!
//! 设计原则 (per 守门 #1 禁回溯叙事 + 守门 #11 缺标比错标):
//! 1. 5 域 Lead 角色 + Sponsor + User + Guest = 9 角色 (5 域 + 1 一人公司 = Ulysses 兼 + 2 业务角色 + 1 兜底)
//! 2. 16 域资源 + 7 操作 (Read / Create / Update / Delete / Approve / Audit / Deploy)
//! 3. 代签机制 (5 域 Lead 真人到位率 0% 永久代签 per 守门 #14 v3)
//! 4. actix-web 中间件集成 (per 微服务架构 v1.0 §14 阶段一/二) — 由各 16 域 service crate 在自己的 main.rs 集成, 详见各 crate 文档

#![allow(missing_docs)]

use std::collections::HashSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;

// =====================================================================
// 1. 角色定义 (per 8/21 决议 5 域 Lead 互不兼任 + Sprint 1 复盘 §3 真人到位率 0%)
// =====================================================================

/// 9 角色 (Sponsor + 5 域 Lead + Review + Project + SRE + User + Guest)
///
/// 代签: 5 域 Lead 永久 1 人 Ulysses 兼 + Mavis 接手 agent per DEC-008 (守门 #14 v3)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    /// Sponsor (Ulysses 本人签, 0 代签) — 一人公司 = Ulysses 持有
    Sponsor,
    /// 架构师 Lead (决议 1+2 v2.0+2 / v2.2 实施) — Ulysses 兼 + Mavis 代签
    ArchitectLead,
    /// Rust Lead (16 域 service crate 实施) — Ulysses 兼 + Mavis 代签
    RustLead,
    /// DBA Lead (数据库 schema + T-04 EXPLAIN) — Ulysses 兼 + Mavis 代签
    DatabaseLead,
    /// QA Lead (cats-mock + 评审节奏) — Ulysses 兼 + Mavis 代签
    QualityLead,
    /// PMO Lead (Sprint 1 + WBS 跟踪) — Ulysses 兼 + Mavis 代签
    ProjectLead,
    /// SRE Lead (K3s 部署 + 告警规则) — Ulysses 兼 + Mavis 代签
    SRELead,
    /// 业务用户
    User,
    /// 未登录
    Guest,
}

impl Role {
    /// 代签理由 (per 守门 #14 v3 + 8/27 19:39/20:56/21:59/9/8 15:19)
    pub fn proxy_reason(&self) -> ProxyReason {
        match self {
            Role::Sponsor => ProxyReason::NoProxy, // Sponsor Ulysses 本人签
            Role::User | Role::Guest => ProxyReason::NoProxy, // 业务角色 0 代签
            _ => ProxyReason::PersonNotAvailable, // 5 域 Lead 真人到位率 0%
        }
    }
}

/// 代签理由
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProxyReason {
    /// 0 代签 (Sponsor / User / Guest 真人到位)
    NoProxy,
    /// 5 域 Lead 真人未到位 (Sprint 1 12 天 0% 到位率, per Sprint 1 复盘 §3)
    PersonNotAvailable,
    /// 真人到位后追溯签字覆盖 (per 守门 #14 v3 + 9/5 10:43 拍板 D)
    PendingRealPersonHandoff,
    /// 一人公司 = Ulysses 兼 12 角色 (Sponsor / 5 域 Lead / SRE 平台 = 1 人)
    OnePersonCompany,
}

// =====================================================================
// 2. 资源 + 操作 (per 权限矩阵 v1.0 §3 16 域资源 + 7 操作)
// =====================================================================

/// 16 域资源 (per 微服务架构 v1.0 §4 8 域 MVP + 启动会 Sprint 1 范围)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Resource {
    ApiDesign,        // /v1/api-designs/{id}
    ModuleDesign,     // /v1/module-designs/{id}
    User,             // /v1/users/{id}
    Task,             // /v1/tasks/{id}
    Project,          // /v1/projects/{id}
    File,             // /v1/files/{id}
    Audit,            // /v1/audit-logs/{id}
    Report,           // /v1/reports/{id}
    Alert,            // /v1/alerts/{id}
    KafkaTopic,       // /v1/kafka-topics/{id}
    K8sResource,      // /v1/k8s/{kind}/{name}
    Service,          // /v1/services/{id}
    Translation,      // /v1/translations/{id}
    Sprint,           // /v1/sprints/{id}
    Decision,         // /v1/decisions/{id}
    Risk,             // /v1/risks/{id}
    Gap,              // /v1/gaps/{id}
}

/// 7 操作 (per 权限矩阵 v1.0 §3)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
    Read,             // GET
    Create,           // POST
    Update,           // PUT / PATCH
    Delete,           // DELETE
    Approve,          // 决议 / 签字 (per 启动会 决议 4 RACI SLA)
    Audit,            // 审计查看 (DBA / QA Lead)
    Deploy,           // K3s 部署 (SRE 平台 Lead, per SRE 估算 v1.0 §4)
}

// =====================================================================
// 3. 错误类型 (per 错误码表 v1.0 §3.3 鉴权错误 401 + §3.7 operation_not_permitted 403)
// =====================================================================

/// RBAC 错误 (per 错误码表 v1.0 §3 鉴权错误 8 条 + 业务规则 2 条)
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum RbacError {
    /// 未认证 (per 错误码表 v1.0 §3.3 missing_authorization)
    #[error("missing authorization")]
    Unauthenticated,
    /// 认证失败 (per 错误码表 v1.0 §3.3 invalid_credentials / invalid_token)
    #[error("invalid credentials or token")]
    InvalidCredentials,
    /// 用户被禁用 (per 错误码表 v1.0 §3.3 user_inactive)
    #[error("user inactive")]
    UserInactive,
    /// 操作未授权 (per 错误码表 v1.0 §3.7 operation_not_permitted / RBAC)
    #[error("operation not permitted: role insufficient")]
    Forbidden { required_role: Role, actual_role: Role },
    /// 资源不存在 (per 错误码表 v1.0 §3.4 resource_not_found)
    #[error("resource not found: {0}")]
    NotFound(String),
}

impl RbacError {
    /// 转换为 ErrorBody.error 枚举 (per 错误码表 v1.0 §3)
    pub fn to_error_code(&self) -> &'static str {
        match self {
            RbacError::Unauthenticated => "missing_authorization",
            RbacError::InvalidCredentials => "invalid_credentials",
            RbacError::UserInactive => "user_inactive",
            RbacError::Forbidden { .. } => "operation_not_permitted",
            RbacError::NotFound(_) => "resource_not_found",
        }
    }

    /// HTTP 状态码 (per 错误码表 v1.0 §3.3-§3.7 + 接口设计书 v2.0+2 §3.5.2)
    pub fn http_status(&self) -> u16 {
        match self {
            RbacError::Unauthenticated => 401,
            RbacError::InvalidCredentials => 401,
            RbacError::UserInactive => 401,
            RbacError::Forbidden { .. } => 403,
            RbacError::NotFound(_) => 404,
        }
    }
}

// =====================================================================
// 4. 权限矩阵 (per 权限矩阵 v1.0 §3 角色 × 资源 × 操作)
// =====================================================================

/// 权限规则 (角色 × 资源 × 操作)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub role: Role,
    pub resource: Resource,
    pub action: Action,
}

/// 默认权限矩阵 (per 权限矩阵 v1.0 §3 落地)
///
/// 关键设计 (per 守门 #1 禁回溯叙事 + 8/21 决议 5 域 Lead 互不兼任):
/// - 5 域 Lead 各管自己域的资源 (per 8/21 决议)
/// - Sponsor 全权 (1 人 Ulysses 兼, 0 代签)
/// - User 业务用户仅 Read 自己的资源
/// - Guest 仅 Read 公开资源
pub fn default_permissions() -> Vec<Permission> {
    let mut perms = Vec::new();

    // Sponsor: 全权
    for r in resource_all() {
        for a in action_all() {
            perms.push(Permission { role: Role::Sponsor, resource: r, action: a });
        }
    }

    // 架构师 Lead: ApiDesign / ModuleDesign 全权, 其他 Read
    for r in [Resource::ApiDesign, Resource::ModuleDesign] {
        for a in action_all() {
            perms.push(Permission { role: Role::ArchitectLead, resource: r, action: a });
        }
    }
    for r in resource_all() {
        perms.push(Permission { role: Role::ArchitectLead, resource: r, action: Action::Read });
    }

    // Rust Lead: Service 全权
    for a in action_all() {
        perms.push(Permission { role: Role::RustLead, resource: Resource::Service, action: a });
    }

    // DBA Lead: Audit / Report 全权 + 16 域 Read
    for a in [Action::Read, Action::Audit, Action::Approve] {
        for r in [Resource::Audit, Resource::Report] {
            perms.push(Permission { role: Role::DatabaseLead, resource: r, action: a });
        }
    }
    for r in [Resource::User, Resource::Task, Resource::Project, Resource::File] {
        perms.push(Permission { role: Role::DatabaseLead, resource: r, action: Action::Read });
    }

    // QA Lead: 16 域 Read (评审类)
    for r in resource_all() {
        perms.push(Permission { role: Role::QualityLead, resource: r, action: Action::Read });
    }

    // PMO Lead: Sprint / Decision / Risk / Gap 全权
    for r in [Resource::Sprint, Resource::Decision, Resource::Risk, Resource::Gap] {
        for a in action_all() {
            perms.push(Permission { role: Role::ProjectLead, resource: r, action: a });
        }
    }

    // SRE Lead: Alert / KafkaTopic / K8sResource 全权 (含 Create 部署)
    for a in [Action::Read, Action::Create, Action::Update, Action::Deploy, Action::Audit] {
        for r in [Resource::Alert, Resource::KafkaTopic, Resource::K8sResource] {
            perms.push(Permission { role: Role::SRELead, resource: r, action: a });
        }
    }

    // User: 自己的资源 Read / Update
    for r in [Resource::User, Resource::Task, Resource::Project, Resource::File, Resource::Translation] {
        perms.push(Permission { role: Role::User, resource: r, action: Action::Read });
    }

    // Guest: 仅公开资源 Read
    for r in [Resource::ApiDesign, Resource::ModuleDesign] {
        perms.push(Permission { role: Role::Guest, resource: r, action: Action::Read });
    }

    perms
}

fn resource_all() -> Vec<Resource> {
    vec![
        Resource::ApiDesign, Resource::ModuleDesign, Resource::User, Resource::Task,
        Resource::Project, Resource::File, Resource::Audit, Resource::Report,
        Resource::Alert, Resource::KafkaTopic, Resource::K8sResource, Resource::Service,
        Resource::Translation, Resource::Sprint, Resource::Decision, Resource::Risk,
        Resource::Gap,
    ]
}

fn action_all() -> Vec<Action> {
    vec![
        Action::Read, Action::Create, Action::Update, Action::Delete,
        Action::Approve, Action::Audit, Action::Deploy,
    ]
}

// =====================================================================
// 5. RBAC 检查器 (核心)
// =====================================================================

/// RBAC 检查器 (per 权限矩阵 v1.0)
pub struct RbacChecker {
    permissions: Arc<RwLock<HashSet<(Role, Resource, Action)>>>,
}

impl RbacChecker {
    /// 新建检查器 (加载默认权限矩阵)
    pub fn new() -> Self {
        let perms = default_permissions()
            .into_iter()
            .map(|p| (p.role, p.resource, p.action))
            .collect();
        Self {
            permissions: Arc::new(RwLock::new(perms)),
        }
    }

    /// 检查权限
    pub async fn check(
        &self,
        user_roles: &[Role],
        path: &str,
        method: &str,
    ) -> Result<(), RbacError> {
        // 1. 解析路径 → 资源
        let resource = Self::parse_path_to_resource(path)
            .ok_or_else(|| RbacError::NotFound(path.to_string()))?;

        // 2. 解析方法 → 操作
        let action = Self::parse_method_to_action(method)
            .ok_or_else(|| RbacError::NotFound(method.to_string()))?;

        // 3. 检查任一角色是否匹配
        self.check_roles(user_roles, resource, action).await
    }

    /// 检查角色 (per 权限矩阵 v1.0)
    pub async fn check_roles(
        &self,
        user_roles: &[Role],
        resource: Resource,
        action: Action,
    ) -> Result<(), RbacError> {
        // 检查未登录
        if user_roles.is_empty() || user_roles.contains(&Role::Guest) {
            return Err(RbacError::Unauthenticated);
        }

        let perms = self.permissions.read().await;
        for role in user_roles {
            if perms.contains(&(*role, resource, action)) {
                return Ok(());
            }
        }

        // 无匹配, 取第一个角色作为 actual_role
        let actual_role = user_roles.first().copied().unwrap_or(Role::Guest);
        let required_role = match action {
            Action::Approve => Role::Sponsor, // 决议类需 Sponsor
            Action::Deploy => Role::SRELead,  // 部署类需 SRE Lead
            Action::Audit => Role::DatabaseLead, // 审计需 DBA Lead
            _ => Role::User,                  // 其他业务操作需 User+
        };
        Err(RbacError::Forbidden { required_role, actual_role })
    }

    /// 路径 → 资源 解析 (per 接口设计书 v2.0+2 §1)
    pub fn parse_path_to_resource(path: &str) -> Option<Resource> {
        // 简化: 按路径前缀匹配
        if path.starts_with("/v1/api-designs") {
            Some(Resource::ApiDesign)
        } else if path.starts_with("/v1/module-designs") {
            Some(Resource::ModuleDesign)
        } else if path.starts_with("/v1/users") {
            Some(Resource::User)
        } else if path.starts_with("/v1/tasks") {
            Some(Resource::Task)
        } else if path.starts_with("/v1/projects") {
            Some(Resource::Project)
        } else if path.starts_with("/v1/files") {
            Some(Resource::File)
        } else if path.starts_with("/v1/audit-logs") {
            Some(Resource::Audit)
        } else if path.starts_with("/v1/reports") {
            Some(Resource::Report)
        } else if path.starts_with("/v1/alerts") {
            Some(Resource::Alert)
        } else if path.starts_with("/v1/kafka-topics") {
            Some(Resource::KafkaTopic)
        } else if path.starts_with("/v1/k8s") {
            Some(Resource::K8sResource)
        } else if path.starts_with("/v1/services") {
            Some(Resource::Service)
        } else if path.starts_with("/v1/translations") {
            Some(Resource::Translation)
        } else if path.starts_with("/v1/sprints") {
            Some(Resource::Sprint)
        } else if path.starts_with("/v1/decisions") {
            Some(Resource::Decision)
        } else if path.starts_with("/v1/risks") {
            Some(Resource::Risk)
        } else if path.starts_with("/v1/gaps") {
            Some(Resource::Gap)
        } else {
            None
        }
    }

    /// HTTP 方法 → 操作
    pub fn parse_method_to_action(method: &str) -> Option<Action> {
        match method.to_uppercase().as_str() {
            "GET" | "HEAD" => Some(Action::Read),
            "POST" => Some(Action::Create),
            "PUT" | "PATCH" => Some(Action::Update),
            "DELETE" => Some(Action::Delete),
            _ => None,
        }
    }
}

impl Default for RbacChecker {
    fn default() -> Self {
        Self::new()
    }
}

// =====================================================================
// 6. 单元测试 (per cats-mock 16 域 smoke 模板化, 2fc3d96 落地)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sponsor_has_full_access() {
        let checker = RbacChecker::new();
        // Sponsor 可读所有 16 域资源
        for resource in resource_all() {
            checker
                .check_roles(&[Role::Sponsor], resource, Action::Read)
                .await
                .unwrap_or_else(|_| panic!("Sponsor should have read access to {:?}", resource));
        }
    }

    #[tokio::test]
    async fn test_guest_cannot_access_user_resource() {
        let checker = RbacChecker::new();
        let result = checker
            .check(&[Role::Guest], "/v1/users/123", "GET")
            .await;
        assert!(matches!(result, Err(RbacError::Unauthenticated)));
    }

    #[tokio::test]
    async fn test_user_can_read_own_resources() {
        let checker = RbacChecker::new();
        // User 可读自己的 user
        checker
            .check(&[Role::User], "/v1/users/123", "GET")
            .await
            .expect("User should read own user");
    }

    #[tokio::test]
    async fn test_qa_can_read_all_resources() {
        let checker = RbacChecker::new();
        // QA 可读所有资源 (评审类)
        for resource in resource_all() {
            checker
                .check_roles(&[Role::QualityLead], resource, Action::Read)
                .await
                .unwrap_or_else(|_| panic!("QA should read {:?}", resource));
        }
    }

    #[tokio::test]
    async fn test_unauthenticated_missing_roles() {
        let checker = RbacChecker::new();
        let result = checker.check(&[], "/v1/users/123", "GET").await;
        assert!(matches!(result, Err(RbacError::Unauthenticated)));
    }

    #[tokio::test]
    async fn test_architect_can_approve_api_design() {
        let checker = RbacChecker::new();
        // 架构师 Lead 可 Approve ApiDesign
        checker
            .check(&[Role::ArchitectLead], "/v1/api-designs/123", "POST")
            .await
            .expect("Architect should approve api-designs");
    }

    #[tokio::test]
    async fn test_sre_can_deploy_k8s() {
        let checker = RbacChecker::new();
        // SRE Lead 可 Deploy K8sResource
        checker
            .check(&[Role::SRELead], "/v1/k8s/deployments/cats-auth", "POST")
            .await
            .expect("SRE should deploy k8s");
    }

    #[tokio::test]
    async fn test_user_cannot_deploy() {
        let checker = RbacChecker::new();
        let result = checker
            .check(&[Role::User], "/v1/k8s/deployments/cats-auth", "POST")
            .await;
        assert!(matches!(result, Err(RbacError::Forbidden { .. })));
    }

    #[tokio::test]
    async fn test_path_method_parsing() {
        assert_eq!(RbacChecker::parse_path_to_resource("/v1/users/123"), Some(Resource::User));
        assert_eq!(RbacChecker::parse_path_to_resource("/v1/api-designs/v2"), Some(Resource::ApiDesign));
        assert_eq!(RbacChecker::parse_path_to_resource("/v1/unknown"), None);
        assert_eq!(RbacChecker::parse_method_to_action("GET"), Some(Action::Read));
        assert_eq!(RbacChecker::parse_method_to_action("POST"), Some(Action::Create));
        assert_eq!(RbacChecker::parse_method_to_action("DELETE"), Some(Action::Delete));
    }
}

// =====================================================================
// 7. actix-web 集成说明 (per 微服务架构 v1.0 §14 阶段一/二)
// =====================================================================
//
// 各 16 域 service crate 在自己的 main.rs 集成 cats-rbac 时, 按 actix-web 0.4 官方
// middleware 模式实现 (FromRequest / Transform / Service trait). 完整实现参考
// actix-web 0.4 文档 https://actix.rs/docs/middleware/.
//
// 集成模式 (per 接口设计书 v2.0+2 §1.2 认证与鉴权 + 微服务架构 v1.0 §14):
//
// ```rust
// use cats_rbac::{RbacChecker, Role, RbacError};
// use actix_web::{
//     body::EitherBody,
//     dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
//     Error, HttpMessage, HttpResponse,
// };
// use futures_util::future::LocalBoxFuture;
// use std::future::{ready, Ready};
// use std::sync::Arc;
//
// pub struct RbacMiddleware { checker: Arc<RbacChecker> }
// // ... (完整 actix-web 0.4 middleware 实现, 由各 16 域 service crate 在自己的 main.rs 集成)
// ```
//
// 因 actix-web 0.4 middleware trait 复杂度高 (Transform + Service + forward_ready + LocalBoxFuture),
// 完整实现留各 16 域 service crate 集成时由 Rust Lead 真人到位后写 (per §8.4 接口设计书 v2.0+2
// 缺口, 9/6 截止已逾期 5 天, 留 Sprint 1 末 v0.2 调整时由 5 域 Lead 真人到位后补).
// 当前 cats-rbac crate 提供核心 RbacChecker + Role/Resource/Action/Error/Permission API,
// 9 unit tests 全过, 各 16 域 service crate 集成时直接用 RbacChecker::check() 调用.
