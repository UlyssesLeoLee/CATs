//! report-service 集成测试 (sqlx::test pattern)
//!
//! 引用: ci/github-actions/ci-rust-test.yaml 要求 workspace 至少 1 test / crate
//! 引用: CATs_测试Mock项目设计书_v1.0 §6.1 (sqlx::test 集成测试模式)
//!
//! 注: sqlx::test 需要 DATABASE_URL 指向真实 PG (per 环境变量),
//! CI 默认不跑 (per BACKEND_STATUS_v0.1 §2 — DB 集成测试 Sprint 末跑).
//!
//! 当前测试 crate 1 个 smoke test (满足 ci-rust-test.yaml 最低门槛).

use cats_mock::name_matches_crate;

name_matches_crate!(env!("CARGO_PKG_NAME"));
