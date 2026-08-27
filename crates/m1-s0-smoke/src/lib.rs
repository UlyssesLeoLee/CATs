//! CATs M1-S0 兼容性冒烟 (per CATs_技术基线_v1.0 §2.3 OI-3)
//!
//! 验证以下 6 项库在 **Rust 1.98.0** 下能编译 + 通过基础功能测试：
//!
//!   - actix-web 4.x    (Web 框架)
//!   - tonic 0.12        (gRPC)
//!   - yrs 0.18          (CRDT)
//!   - tauri 1.x         (客户端 - 编译期 type-check; UI 由 apps/cats-client/ 实施)
//!   - sqlx 0.8          (PG 异步驱动 - 编译期不连 DB)
//!
//! 编译期通过即代表基线兼容。
//!
//! 不在本 smoke 范围（避免 cmake-build 依赖）:
//!
//!   - rdkafka 0.36  → 由 cats-bff + Kafka worktree 验证（K3s 阶段二，OI-3 §2.3 偏移）
//!
//! 运行时连接验证（PG 18.6 + pgvector 0.8.6 + Kafka）留 M1-Sprint 0 末。

pub mod actix_smoke;
pub mod sqlx_smoke;
pub mod tauri_smoke;
pub mod tonic_smoke;
pub mod yrs_smoke;

pub use actix_smoke::*;
pub use sqlx_smoke::*;
pub use tauri_smoke::*;
pub use tonic_smoke::*;
pub use yrs_smoke::*;

/// M1-S0 smoke crate 版本（与 workspace.package.version 一致）
pub const SMOKE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_crate_self_compiles_and_reports_version() {
        assert!(!SMOKE_VERSION.is_empty());
        println!("M1-S0 smoke version: {}", SMOKE_VERSION);
    }
}
