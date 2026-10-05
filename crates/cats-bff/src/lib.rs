//! `cats-bff` — BFF 聚合服务 (M1 阶段, per 切片 A _slice_a_bff.md)
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1
//! 引用: api/openapi/cats-openapi-v1.0.1.yaml §paths

pub mod config;
pub mod error;
pub mod handlers;
pub mod principal;
pub mod upstream;

// 2026-10-05 接线第 1 步（共 3 步）：`upstream_passthrough.rs`（144 行）。
//
// 它与 `src/upstream/` 是两条平行设计 —— 本文件是共享 reqwest client + 裸
// `serde_json::Value` 透传，`src/upstream/` 是三个强类型客户端（main.rs 实际
// 注册路由用的那套）。文件头原本写着"直接接入不可行——依赖的 4 个 API 在当前
// `Config` 上均不存在"，本步已逐条处理，详见该文件头部的新说明。
//
// 第 2 步 routes.rs、第 3 步 grpc_clients.rs 尚未接入。
pub mod upstream_passthrough;

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
        assert_eq!(name(), "cats-bff");
    }
}
