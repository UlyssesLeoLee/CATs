#!/usr/bin/env bash
# regression-test-aigw.sh - 跑 AI 网关集成 4 件套
# 用法: bash crates/cats-mock/scripts/regression-test-aigw.sh

set -euo pipefail

echo "==> AI 网关集成 test"

cargo test --release -p cats-ai-gateway 2>&1 | tail -20

echo "==> ALL AIGW TESTS PASS"