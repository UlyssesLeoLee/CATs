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

// 2026-10-05 接线第 2 步（共 3 步）：`grpc_clients.rs`（125 行），
// translation-core gRPC 客户端。详见该文件头部的适配说明。
//
// 注意步骤顺序被调整过：原计划是 routes → grpc_clients，但 routes.rs
// `use crate::grpc_clients::{TmCommitAck, TmLookupResponse, TranslationClient}`
// —— 它硬依赖本文件，所以 grpc_clients 必须排在前面。
pub mod grpc_clients;

// 2026-10-05 接线第 1 步（共 3 步）：`upstream_passthrough.rs`（144 行）。
//
// 它与 `src/upstream/` 是两条平行设计 —— 本文件是共享 reqwest client + 裸
// `serde_json::Value` 透传，`src/upstream/` 是三个强类型客户端（main.rs 实际
// 注册路由用的那套）。文件头原本写着"直接接入不可行——依赖的 4 个 API 在当前
// `Config` 上均不存在"，本步已逐条处理，详见该文件头部的新说明。
//
//
// 2026-10-05 接线第 3 步（共 3 步）：`routes.rs`（178 行），
// `/api/v1/*` 前缀那一套路由。接完本 crate 孤儿清零。
pub mod routes;
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
