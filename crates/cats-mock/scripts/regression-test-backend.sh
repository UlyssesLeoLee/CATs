#!/usr/bin/env bash
# regression-test-backend.sh - 跑 backend 集成测试
# 用法: bash crates/cats-mock/scripts/regression-test-backend.sh

set -euo pipefail

echo "==> backend regression test (cargo test -p <service>)"

for svc in common project task file notification report audit worker translation-core cats-rbac; do
  echo "    testing $svc"
  cargo test --release -p "$svc" 2>&1 | tail -5
done

echo "==> ALL BACKEND TESTS PASS"