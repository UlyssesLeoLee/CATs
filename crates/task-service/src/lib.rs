//! `task-service` — 任务调度服务 (per ULYS-151 切片 B-2)
//!
//! 引用: doc/05-其他/MVP商业版/_slice_b2_task.md
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md §3.4
//! 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1
//!
//! 模块清单 (per 切片 B-2 交付清单):
//! - [`db`]        — PgPool + CRUD (create / find_by_id / list / update_status)
//! - [`models`]    — DTO + 业务枚举 + DB row + SSE event
//! - [`events`]    — 进程内事件总线 (per ULYS-45 既有实现, 切片 B-2 复用 + PATCH status 联动)
//! - [`rbac`]      — cats-rbac 中间件集成 (路径/方法解析 + AuthContext 提取 + enforce)
//! - [`handlers`]  — 7 endpoints (healthz + 4 REST + 1 SSE + 1 internal)
//!
//! 业务 endpoint:
//! - `POST /v1/tasks`                    — 创建翻译任务
//! - `GET  /v1/tasks`                    — 列出任务
//! - `GET  /v1/tasks/{id}`               — 任务详情
//! - `PATCH /v1/tasks/{id}/status`       — 状态变更 (同时 publish 终态到 SSE)
//! - `GET  /v1/tasks/{id}/events`        — SSE 进度推送
//! - `POST /internal/v1/tasks/{id}/stage-progress` — 内部上报
//!
//! 验收标准 (per 切片 B-2):
//! - 5 个业务 endpoint 全部实现 ✓
//! - SSE 用 actix-web stream + async-stream + tokio::sync::broadcast ✓
//! - RBAC 中间件挂在每个 endpoint (rbac::enforce inline) ✓
//! - 集成测试 ≥3 case (`tests/integration.rs`) ✓
//! - `cargo check -p task-service` 通过 ✓

#![allow(missing_docs)] // 子模块各自有 doc; 这里不再重复

pub mod db;
pub mod events;
pub mod handlers;
pub mod models;
pub mod rbac;

pub use events::{EventBus, SharedEventBus, Subscription, TaskState, TaskView as BusTaskView};
pub use handlers::AppState;
pub use models::{
    CreateTaskRequest, ErrorBody, ListTasksQuery, ListTasksResponse, StageKind, StageMetrics,
    StageProgressRequest, StageResultRef, StageStatus, SseTaskStatus, Task, TaskEvent, TaskStatus,
    TaskType, TaskView, UpdateStatusRequest,
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
    fn name_is_crate_name() {
        assert_eq!(name(), "task-service");
    }

    #[test]
    fn public_modules_are_exported() {
        // 防重构时漏 pub; 仅检查模块名存在, 不调用 fn item (async fn 无法作为 fn pointer)
        let _: fn() -> &'static str = version;
        let _: fn() -> &'static str = name;
        // 子模块存在性 (编译期检查)
        let _: Option<db::ListFilter> = None;
    }
}