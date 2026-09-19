//! `worker-service` — 周期 worker 服务
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//!       doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md §6.10
//!
//! MVP 角色:
//! - 周期任务: 每 30s 查 `tasks WHERE status='pending'`, 委托给 translation-core
//! - 健康端点: /healthz, /readyz
//!
//! 注意: worker **不** 直连 translation-core DB; 它通过 task-service API
//! 更新 task 状态 (CALLBACK), 或在 MVP 简化下通过 gRPC stub.

pub mod scheduler;
pub mod handlers;
pub mod state;

pub use state::AppState;

/// 当前 crate 语义版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// 当前 crate 名称
pub const NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_semver_like() {
        let v = VERSION;
        assert!(v.starts_with("0.1."), "version should start with '0.1.', got {v}");
    }

    #[test]
    fn name_is_crate_name() {
        assert_eq!(NAME, "worker-service");
    }
}