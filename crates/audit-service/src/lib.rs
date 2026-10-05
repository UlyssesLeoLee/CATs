//! `audit-service` — 审计服务
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1
//! 引用: ULYS-153 切片 C-2 — 真实 Kafka consumer
//!
//! M0 阶段：仅暴露 `version()` / `name()` + `consumer` 模块 (Kafka REST proxy)。
//! `consumer::run_consumer_loop` 在 main.rs spawn, 订阅 `cats.audit.v1` topic,
//! 处理逻辑见 `consumer::process_event`。
//!
//! ## 2026-10-05 更正
//!
//! 本文件此前写着「业务 handler/db/model 模块保持孤儿 …… 待后续切片引入
//! deps 后挂接」。那句在写下时是真的，接线完成后就不成立了 —— 现在
//! `db` / `handlers` / `models` / `state` **全部在编译内**，服务也因此第一次
//! 有了业务端点（`handlers::configure` 是唯一路由表，`main.rs` 与集成测试
//! 共用）。上面的 M0 段落保留原样作为历史记录，但不要再拿它描述现状。

pub mod consumer;

// 2026-10-05 接线：这四个文件此前从未被编译。
//
// 它们的依赖链正好卡在本轮接上的两个共享模块上 ——
//   handlers.rs 用 cats_common::{cats_error_to_response, CatsError, ErrorCode}
//             和 cats_rbac::service_helpers::{extract_user_id_and_roles, require_roles}
//   db.rs / models.rs 用 cats_common::{CatsError, ErrorCode}
//
// 也就是说：那两个共享模块写好了却因为自己没有 `mod` 声明而不可用，
// 于是依赖它们的 audit handler 也一起卡死。接上共享模块等于一次性
// 解锁了这条链，`audit-service` 至此才真的有 HTTP 端点
// （此前只有 main.rs 里那个 /healthz）。
pub mod db;
pub mod handlers;
pub mod models;
pub mod state;

pub use consumer::{process_event, run_consumer_loop};

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
        assert_eq!(name(), "audit-service");
    }
}
