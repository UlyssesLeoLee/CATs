#!/usr/bin/env bash
# regression-test-app.sh - 客户端编译校验(实际 e2e 留给 UI 测试工具)
# 用法: bash crates/cats-mock/scripts/regression-test-app.sh

set -euo pipefail

echo "==> app shell cargo check"
cargo check --manifest-path apps/cats-client/Cargo.toml 2>&1 | tail -5

echo "==> bff cargo check"
cargo check -p cats-bff 2>&1 | tail -5

echo "==> ALL APP CHECKS PASS"