//! `cats-bff` — BFF 聚合服务 (M1 阶段, per 切片 A _slice_a_bff.md)
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1
//! 引用: api/openapi/cats-openapi-v1.0.1.yaml §paths

// 2026-10-07 删除三个模块：`grpc_clients.rs`(125) / `upstream_passthrough.rs`(144)
// / `routes.rs`(178)，共 447 行。
//
// 它们是一条"接了 3 步但没挂上服务"的链：`routes::configure` 是链的顶端，而它
// 唯一的调用者是一个测试文件，`main.rs` 从不调用。判定它是废弃草稿的依据是规格：
//   - `api/openapi/cats-openapi-v1.0.1.yaml` 的 7 条 path 里没有 `/api/v1` 前缀
//   - 也没有任何 translate 端点，而 `routes::translate_commit` 本身就是 501 stub
//   - `deploy/envoy-mvp.yaml` 的 8 条 route 里没有 cats_bff
// 即"编译通过、类型检查通过、但运行时不可达"。路由表已收进
// `handlers::configure_routes`，`main.rs` 与两个集成测试挂的是同一张。
pub mod config;
pub mod error;
pub mod handlers;
pub mod principal;
pub mod upstream;

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
