//! `cats-proto` — CATs gRPC 协议契约
//!
//! 引用: doc/02-基础设计/技术选型/CATs_Rust技术选型书_v1.0.md §5.3 + §8.2
//! 引用: proto/cats/v1/*.proto（4 份：auth / common / media / translation_core）
//!
//! M0 阶段：仅暴露模块入口与 `version()`。
//! 真正的 gRPC client/server trait 由 `tonic-build` 在编译期生成，
//! 通过 `cats_proto::cats::v1::*` 命名空间访问。
//!
//! ## 用法（M1 阶段）
//!
//! ```ignore
//! use cats_proto::cats::v1::auth_service_client::AuthServiceClient;
//! use cats_proto::cats::v1::LoginRequest;
//! ```

/// 重新导出 tonic-build 生成的 `cats.v1` 模块
///
/// 该模块由 `build.rs` 在编译期生成，包含：
/// - `auth_service_client` / `auth_service_server`
/// - `media_processing_service_client` / `media_processing_service_server`
/// - `translation_core_service_client` / `translation_core_service_server`
/// - 消息类型（`LoginRequest`, `MatchTMRequest`, ...）
/// - 枚举（`LanguageCode`, `TaskStatus`, ...）
pub mod cats {
    /// 由 tonic-build 生成的 v1 协议模块
    ///
    /// 抑制 lint：生成代码无 doc 注释，且会被 clippy 各种规则刷出
    /// "全部允许"组合避免 workspace 严格 lint 误伤。
    #[allow(
        missing_docs,
        clippy::all,
        clippy::pedantic,
        clippy::nursery,
        dead_code,
        non_camel_case_types
    )]
    pub mod v1 {
        tonic::include_proto!("cats.v1");
    }
}

/// 当前 crate 版本（与 workspace.package.version 同步）
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
    fn name_is_cats_proto() {
        assert_eq!(name(), "cats-proto");
    }

    #[test]
    fn v1_module_exposes_common() {
        // 仅做存在性检查（编译期即保证）
        let _type_check: fn() -> Option<cats::v1::Pagination> = || None;
    }
}
