//! `project-service` — 项目配置服务
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1
//! 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md (projects 表)
//! 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.2 (project_db 接口契约 v1.0.0)
//! 引用: doc/05-其他/管理/CATs_错误码表_v1.0.md §3-§4 (error enum 复用)
//! 引用: ULYS-150 切片 B-1 §"任务范围" — POST/GET/PATCH/DELETE /v1/projects
//!
//! M1 业务实现 (per ULYS-150 切片 B-1 完成判据):
//! - POST   /v1/projects                  — 创建项目
//! - GET    /v1/projects                  — 列出 (分页 + workspace_id 过滤)
//! - GET    /v1/projects/{id}             — 查询
//! - PATCH  /v1/projects/{id}             — 部分更新 (name/source_lang/target_lang/status)
//! - DELETE /v1/projects/{id}             — 软删除 (status='archived')
//! - GET    /healthz
//! - project_db (per Baseline §5.2) + gen_random_uuid() (pgcrypto 已默认启用 per workspace 决定)

pub mod db;
pub mod handlers;
pub mod models;
pub mod rbac;

pub use models::{
    CreateProjectRequest, ErrorBody, GetProjectResponse, ListProjectsItem, ListProjectsQuery,
    ListProjectsResponse, Project, ProjectStatus, UpdateProjectRequest,
};

/// 当前 crate 语义版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 当前 crate 名称
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// 返回 crate 版本字符串
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
        assert!(
            v.starts_with("0.1."),
            "version should start with '0.1.', got {v}"
        );
    }

    #[test]
    fn name_is_project_service() {
        assert_eq!(name(), "project-service");
    }
}