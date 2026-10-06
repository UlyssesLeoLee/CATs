//! `translation-core` — 翻译核心编排服务
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1
//!
//! 2026-10-04 接线：本文件原先只声明 `version()` / `name()`，
//! 下面 6 个模块共 550 行**从未被编译**（实证：往 `db.rs` 注入语法错误后
//! `cargo check -p translation-core` 仍退出 0）。现在它们全部进编译，
//! 由 `main.rs` 起一个 tonic gRPC server 承载。
//!
//! - [`service`] — 4 个 RPC 的 gRPC 实现（MatchTM / TranslateSegment / RunQA / BatchTranslate）
//! - [`ai_gateway`] — AI 网关 trait + MVP 的 mock 实现
//! - [`qa`] — QA 规则引擎（术语强制 + 占位符保护）
//! - [`db`] — project_db 数据访问层（TM / glossary）
//! - [`tm`] / [`glossary`] — 上面两个模块的领域视图 re-export

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

pub mod ai_gateway;
pub mod db;
pub mod glossary;
pub mod qa;
pub mod service;
pub mod tm;

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
        assert_eq!(name(), "translation-core");
    }
}
