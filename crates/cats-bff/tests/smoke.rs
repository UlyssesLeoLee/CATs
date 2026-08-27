//! `cats-bff` smoke test
//!
//! 引用: ci/github-actions/ci-rust-test.yaml 要求 workspace 至少 1 test / crate

use cats_bff::{name, version};

#[test]
fn version_is_semver_like() {
    let v = version();
    assert!(v.starts_with("0.1."), "expected 0.1.x, got {v}");
}

#[test]
fn name_matches_crate() {
    assert_eq!(name(), "cats-bff");
}
