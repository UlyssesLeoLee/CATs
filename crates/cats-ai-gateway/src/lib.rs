//! `cats-ai-gateway` — CATs AI 网关 MVP
//!
//! 引用: doc/02-基础设计/决策/CATs_ADR-010_LLM部署与推理策略_v1.0.md §4 (双路径 + 顺序回退)
//! 引用: doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md §1.3 (统一错误信封)
//! 引用: doc/05-其他/错误码/CATs_错误码表_v1.0.1.md (12 状态错误码)
//!
//! MVP 范围 (per 2026-09-19 落地任务):
//! 1. **provider 抽象**: 4 mock provider (openai / anthropic / gemini / deepseek)
//! 2. **路由器**: 按 model 字段 + 顺序回退策略 (OpenAI→Anthropic→Gemini→DeepSeek)
//! 3. **配额**: 每分钟 token 数限制 (MVP 内存记账, 100_000 tokens/min/org)
//! 4. **重试 + 降级**: 指数退避 3 次 + provider 级 fallback
//! 5. **合规开关**: Cloud / Local 双路径, Local 强制本地 (未实现 → 409)
//! 6. **REST + gRPC 出口**: 给 translation-core + 其他服务调用
//! 7. **集成测试**: routing / quota / compliance / retry 4 件套
//!
//! 不在范围 (per 任务边界):
//! - 真实 OpenAI / Anthropic / Gemini / DeepSeek API 接入 (留 Sprint 3)
//! - 模型微调 / 训练 / 推理优化 (留 V2)
//! - mTLS (per ADR-009, MVP 阶段不上)

#![allow(missing_docs)]
#![allow(clippy::needless_late_init)]  // mock impl 内 OK

pub mod provider;
pub mod router;
pub mod quota;
pub mod retry;
pub mod compliance;
pub mod error;
pub mod api;
pub mod service;

// 由 tonic-build 在编译期生成（per build.rs）
pub mod proto {
    /// cats.llm.v1 — AI 网关 gRPC namespace（per 落地任务 2026-09-19）
    ///
    /// 抑制 lint: 生成代码无 doc 注释
    #[allow(
        missing_docs,
        clippy::all,
        clippy::pedantic,
        clippy::nursery,
        dead_code,
        non_camel_case_types
    )]
    pub mod llm {
        pub mod v1 {
            tonic::include_proto!("cats.llm.v1");
        }
    }
}

/// crate 语义版本 (workspace 同步)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// crate 名称
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// 返回 crate 版本字符串
pub fn version() -> &'static str { VERSION }
/// 返回 crate 名称
pub fn name() -> &'static str { NAME }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_semver_like() {
        let v = version();
        assert!(v.starts_with("0.1."), "version should start with '0.1.', got {v}");
    }

    #[test]
    fn name_is_cats_ai_gateway() {
        assert_eq!(name(), "cats-ai-gateway");
    }

    #[test]
    fn public_modules_are_exported() {
        // 回归保护：防重构时漏 pub
        let _: fn() -> &'static str = provider::ProviderName::as_str;
    }
}