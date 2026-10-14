//! notification-service smoke test
//!
//! 引用: ci/github-actions/ci-rust-test.yaml 要求 workspace 至少 1 test / crate

use cats_mock::name_matches_crate;

name_matches_crate!(env!("CARGO_PKG_NAME"));
