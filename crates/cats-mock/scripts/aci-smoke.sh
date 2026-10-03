#!/usr/bin/env bash
# CATs aci-smoke.sh v0.1 — ULYS-191 §4.3.2 brief v0.1
# 调 `cargo test` 间接 emit 一条 assertion (cats-mock 是 dev-deps, 不像 IDE1.0 有 CLI),
# 验 schema 合法, 输出 PASS/FAIL.
#
# 4 步:
#   1. cargo build --release -p cats-mock
#   2. cargo test -p cats-mock --test aci_integration (触发 emit_smoke_assertion + 3 IT)
#   3. 验 10 必填字段全在 (通过 IT-2 test_cats_mock_emits_valid_aci_assertion)
#   4. 验 schema_version (= "0.1.0-draft")

set -euo pipefail

WORKDIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$WORKDIR"

echo "=== CATs aci-smoke.sh v0.1 ==="
echo "workdir: $WORKDIR"
echo "branch: $(git branch --show-current 2>/dev/null || echo 'unknown')"
echo

# Step 1: 编译
echo "[step 1/4] cargo build --release -p cats-mock"
cargo build --release -p cats-mock

# Step 2: emit (通过 cargo test 触发 emit_smoke_assertion 调用)
echo "[step 2/4] cargo test -p cats-mock --test aci_integration"
cargo test -p cats-mock --test aci_integration -- --nocapture 2>&1 | tee /tmp/cats-aci-smoke-test.log

# Step 3: 验 10 必填字段 (cargo test IT-2 已验)
echo "[step 3/4] verify 10 required fields (via IT-2 test_cats_mock_emits_valid_aci_assertion)"
if ! grep -q "test_aci_schema_v0_1_roundtrip ... ok" /tmp/cats-aci-smoke-test.log; then
  echo "FAIL: IT-1 test_aci_schema_v0_1_roundtrip did not pass"
  exit 1
fi
if ! grep -q "test_cats_mock_emits_valid_aci_assertion ... ok" /tmp/cats-aci-smoke-test.log; then
  echo "FAIL: IT-2 test_cats_mock_emits_valid_aci_assertion did not pass"
  exit 1
fi
if ! grep -q "test_aci_emitter_v0_1_compatibility ... ok" /tmp/cats-aci-smoke-test.log; then
  echo "FAIL: IT-3 test_aci_emitter_v0_1_compatibility did not pass"
  exit 1
fi

# Step 4: 验 schema_version (通过 IT-2 已验)
echo "[step 4/4] verify aci_version == 0.1.0-draft (via IT-1 + IT-2)"

echo
echo "PASS: aci-smoke.sh 4 step verify complete"
echo "  - IT-1 schema v0.1 roundtrip ✅"
echo "  - IT-2 cats_mock emit_smoke_assertion ✅"
echo "  - IT-3 cross-language parity ✅"
