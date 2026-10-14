//! file-service smoke test
//!
//! 引用: ci/github-actions/ci-rust-test.yaml 要求 workspace 至少 1 test / crate
//! 改造: 2026-09-01 per CATs_测试Mock项目设计书_v1.0 §5.2, 16 行 boilerplate -> 1 行宏调用

use cats_mock::name_matches_crate;

name_matches_crate!(env!("CARGO_PKG_NAME"));
