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
//!
//! ## module_switch 接入 (per ULYS-190 §4.4 CATs stage2 brief)
//!
//! CATs cats-mock 通过 `.aci.json` 的 `plugins.<plugin_id>.modules.<module_id>`
//! 三层开关暴露 13 个 module_switch (data 4 + db 3 + http 4 + infra 2)。读法:
//!
//! ```text
//! 跨语言 dispatch:
//!   python crates/cats-mock/scripts/_lib_mock_switch_cats.py crates/cats-mock
//! ```
//!
//! 跨项目範式对齐 IM1.0 stage1 (per G-MS-04 命名一致性 + mock_ws_frames server_frames 聚合模式)。
//! Rust native 版本跨 session (per G-MS-BRIEF-S44-01 跨项目推广); 当前 Python helper 通过
//! subprocess 暴露 (per AGENTS.md 守门 #9)。CI gate `mock-switch-validate.py
//! validate-one cats` (Star ship) 验证 13 module 全部 enabled + cluster_ok=True。
//!
//! mock_switch_trace_format (per .mock-cluster.json) 输出例 (~92 字, 略超 G-MS-08 ~80 字,
//! 跨 session 截断):
//!   `cluster.enabled=True,mode=offline,plugins=[data(4m),db(3m),http(4m),infra(2m)]=13/13 modules`
//!
//! 13 module 拆分 (per §4.4 stage2 命名锁定):
//! - `data` plugin 4 module: user / project / task / audit (per 业务对象 factory)
//! - `db` plugin 3 module: fixture / schema / seed (per DB fixture 角色)
//! - `http` plugin 4 module: server / routes / response / healthz (per HTTP server 角色)
//! - `infra` plugin 2 module: kafka / redis (per 基础设施替身)

// 公共 API 的详细文档见设计书 doc/02-基础设计/测试/CATs_测试Mock项目设计书_v1.0.md
// 设计书是 single source of truth;这里不强制每行 doc comment, 避免 100+ warning 噪音
#![allow(missing_docs)]

pub mod aci_emitter_helper;
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
    fn name_is_cats_mock() {
        assert_eq!(name(), "cats-mock");
    }

    #[test]
    fn public_modules_are_exported() {
        // 4 大模块都能被外部 use (回归保护: 防重构时漏 pub)
        let _: fn() -> usize = || data::UserFactory::default_count();
    }
}
