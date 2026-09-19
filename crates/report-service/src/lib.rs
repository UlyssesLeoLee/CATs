//! `report-service` — 报表统计 (跨 task_db 只读)
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §6.8

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

    #[test]
    fn version_is_semver_like() {
        assert!(version().starts_with("0.1."));
    }

    #[test]
    fn name_is_report_service() {
        assert_eq!(name(), "report-service");
    }
}