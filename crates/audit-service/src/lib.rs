//! `audit-service` 业务模块导出
//!
//! 端点 (per 接口设计 §6.7):
//! - GET    /healthz
//! - GET    /readyz
//! - GET    /v1/audit-logs?org_id=...&limit=...&offset=...
//! - POST   /v1/audit-logs/test  (手动 ingest 一条)
//!
//! Kafka 消费者: 订阅 `cats.audit.v1`,每条事件落 audit_logs 表 (per DB 设计 v2.0 §4.8)

pub mod consumer;
pub mod db;
pub mod handlers;
pub mod models;
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
        assert_eq!(NAME, "audit-service");
    }
}