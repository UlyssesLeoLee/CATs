//! `file-service` smoke test
//!
//! 引用: ci/github-actions/ci-rust-test.yaml 要求 workspace 至少 1 test / crate

use file_service::{name, version};

#[test]
fn version_is_semver_like() {
    let v = version();
    assert!(v.starts_with("0.1."), "expected 0.1.x, got {v}");
}

#[test]
fn name_matches_crate() {
    assert_eq!(name(), "file-service");
}
