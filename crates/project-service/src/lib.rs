//! `project-service` — 项目管理服务
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1 (MVP 8 核心服务)
//! 引用: doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md §6.3
//! 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §6.4
//! 引用: doc/03-详细设计/模块设计/CATs_模块设计书_v2.0.md §4
//! 引用: doc/05-其他/管理/CATs_权限决策书_v1.0.md §3
//!
//! MVP 端到端最小业务流 (per brief):
//! - POST /v1/projects                  — 创建项目 (per 接口设计 §6.3)
//! - GET  /v1/projects/{id}             — 查询
//! - PUT  /v1/projects/{id}             — 更新
//! - GET  /v1/projects?org_id=...      — 列 org 项目 (分页)
//! - POST /v1/projects/{id}/members     — 加成员
//! - GET  /v1/projects/{id}/members     — 列成员
//!
//! 多租户隔离: org_id 强制 + handler 内跨 org 检查 (per §14 阶段一)
//! RBAC 集成: cats-rbac crate + X-Cats-Roles header 解析 (per §1.2)
//! 错误处理: cats-common::CatsError → cats_error_to_response (per §3.5)

pub mod db;
pub mod handlers;
pub mod models;
pub mod state;

pub use state::AppState;

/// crate 版本字符串
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// crate 名称
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// 返回 crate 版本
pub fn version() -> &'static str {
    VERSION
}

/// 返回 crate 名称
pub fn name() -> &'static str {
    NAME
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_semver_like() {
        let v = version();
        assert!(v.starts_with("0.1."), "version should start with '0.1.', got {v}");
    }

    #[test]
    fn name_is_project_service() {
        assert_eq!(name(), "project-service");
    }
}