//! `task-service` — 翻译任务编排
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md §6.4
//! 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §6.6

pub mod db;
pub mod handlers;
pub mod models;
pub mod state;

pub use state::AppState;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");

pub fn version() -> &'static str {
    VERSION
}
pub fn name() -> &'static str {
    NAME
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TaskStatus;

    #[test]
    fn version_is_semver_like() {
        let v = version();
        assert!(v.starts_with("0.1."), "version should start with '0.1.', got {v}");
    }

    #[test]
    fn name_is_task_service() {
        assert_eq!(name(), "task-service");
    }

    #[test]
    fn state_machine_pending_to_in_progress_allowed() {
        assert!(TaskStatus::Pending.can_transition_to(&TaskStatus::InProgress));
    }

    #[test]
    fn state_machine_completed_is_terminal() {
        assert!(!TaskStatus::Completed.can_transition_to(&TaskStatus::InProgress));
        assert!(!TaskStatus::Completed.can_transition_to(&TaskStatus::Pending));
    }

    #[test]
    fn state_machine_qa_blocked_can_retry_in_progress() {
        assert!(TaskStatus::QaBlocked.can_transition_to(&TaskStatus::InProgress));
        assert!(TaskStatus::QaBlocked.can_transition_to(&TaskStatus::Failed));
    }

    #[test]
    fn state_machine_status_round_trip() {
        for s in [
            TaskStatus::Pending,
            TaskStatus::InProgress,
            TaskStatus::Completed,
            TaskStatus::QaBlocked,
            TaskStatus::Failed,
        ] {
            assert_eq!(TaskStatus::parse(s.as_str()), Some(s));
        }
        assert_eq!(TaskStatus::parse("unknown"), None);
    }
}