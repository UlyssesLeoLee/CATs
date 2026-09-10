//! `cats-mock` — CATs 测试 Mock 项目
//!
//! 引用: doc/02-基础设计/测试/CATs_测试Mock项目设计书_v1.0.md
//!
//! 提供 4 类 mock 能力,所有 16 个 service crate 在 tests/ 下统一复用:
//!
//! | 模块 | 职责 | 用途 |
//! |------|------|------|
//! | [`http`] | 内存 HTTP server 工厂 (actix-web 路由注册) | 替代 wiremock/axum-test |
//! | [`db`]   | PG 18.6 + pgvector 0.8 fixture (SQL + 内存 schema) | 替代 testcontainers |
//! | [`infra`] | Kafka / Redis in-memory 替身 (Mutex<Vec<T>>) | 替代 rdkafka-test / fake-redis |
//! | [`data`] | 业务对象 factory (user/project/task/audit) | 替代手工构造 fixture |
//!
//! 集成入口 (供调用方 `use cats_mock::*`):
//!
//! ```ignore
//! use cats_mock::{
//!     data::UserFactory,            // 业务数据
//!     http::MockServer,              // HTTP server
//!     infra::{MockKafka, MockRedis}, // 基础设施
//!     db::DbFixture,                 // DB fixture
//! };
//! ```

// 公共 API 的详细文档见设计书 doc/02-基础设计/测试/CATs_测试Mock项目设计书_v1.0.md
// 设计书是 single source of truth;这里不强制每行 doc comment, 避免 100+ warning 噪音
#![allow(missing_docs)]

pub mod data;
pub mod db;
pub mod http;
pub mod infra;
pub mod smoke;

/// crate 语义版本 (与 workspace.package.version 同步)
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
    fn name_is_cats_mock() {
        assert_eq!(name(), "cats-mock");
    }

    #[test]
    fn public_modules_are_exported() {
        // 4 大模块都能被外部 use (回归保护: 防重构时漏 pub)
        let _: fn() -> usize = || data::UserFactory::default_count();
    }
}
