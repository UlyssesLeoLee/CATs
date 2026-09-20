//! `report-service` — 报表服务
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1
//! 引用: ULYS-153 切片 C-1 — 3 报表 endpoint + 跨表聚合 SQL
//!
//! M1 业务实现 (per ULYS-153 切片 C-1 完成判据):
//! - GET /v1/reports/usage?from=&to=&org_id=
//!     — 按 action/resource_type 分组的统计 (e.g. login 数 / task 创建数 / 翻译完成数)
//! - GET /v1/reports/translation-volume?project_id=&from=&to=
//!     — 翻译量统计 (audit_logs 中 action='translate.completed' 过滤 by project_id)
//! - GET /v1/reports/audit-summary?workspace_id=&from=&to=
//!     — 审计摘要 (总事件数 / 独立 actor 数 / 按 action 分组 topN / 最近事件时间)
//!
//! 数据源说明 (per 切片 C-1 §数据源 + 微服务架构书 §1.2 原则 4):
//! - 默认从 report_db 自有 schema 聚合 (本切片 MVP)
//! - 跨服务 audit_logs JOIN 需要 ops 侧 GRANT `svc_report` SELECT ON audit_logs
//!   + dblink / postgres_fdw 跨库访问, 详见 README §跨服务聚合配置

pub mod db;
pub mod handlers;
pub mod models;

pub use handlers::{
    audit_summary, healthz, healthz_response, translation_volume, usage_report, HealthResponse,
};
pub use models::{
    ActionCountRow, AuditSummary, AuditSummaryResponse, TranslationVolumeRow,
    TranslationVolumeResponse, UsageReportItem, UsageReportResponse,
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
        assert_eq!(name(), "report-service");
    }
}
