//! `translation-core` 业务模块导出
//!
//! gRPC 服务: cats.v1.TranslationCoreService (per proto/cats/v1/translation_core.proto)
//! - MatchTM
//! - TranslateSegment
//! - RunQA
//! - BatchTranslate
//!
//! 数据层: project_db (TM + glossary + pgvector)
//! AI 网关: 通过 trait `AiGateway` 注入 (MVP 阶段 = mock 实现, 留 Sprint 3 接 cats-ai-gateway)

pub mod ai_gateway;
pub mod db;
pub mod glossary;
pub mod qa;
pub mod service;
pub mod tm;

pub use service::TranslationCoreServiceImpl;

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
        assert_eq!(NAME, "translation-core");
    }
}