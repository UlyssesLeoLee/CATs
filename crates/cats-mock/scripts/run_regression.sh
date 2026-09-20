#!/usr/bin/env bash
# cats-mock 整体回归测试运行脚本 (Bash/MSYS 版)
#
# 引用: doc/02-基础设计/测试/CATs_测试Mock项目设计书_v1.0.md §5.4
# 引用: ULYS-135 — 完善mock项目UT、IT、ST各流程测试的脚本并进行整体的回归测试
#
# 用法:
#   bash scripts/run_regression.sh                       # 跑 cats-mock 全套
#   bash scripts/run_regression.sh --target all          # + 16 service smoke
#   bash scripts/run_regression.sh --only st_regression  # 仅 ST
#   bash scripts/run_regression.sh --verbose             # 详细输出
#   bash scripts/run_regression.sh --keep-going          # 出错继续
#
# 退出码:
#   0 = 全部通过
#   非 0 = 有失败用例

set -u

# ---- 参数解析 ----
TARGET="mock"
ONLY="all"
VERBOSE=0
KEEP_GOING=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --target)   TARGET="$2"; shift 2 ;;
        --only)     ONLY="$2"; shift 2 ;;
        --verbose)  VERBOSE=1; shift ;;
        --keep-going|-k) KEEP_GOING=1; shift ;;
        *) echo "unknown arg: $1"; exit 2 ;;
    esac
done

# ---- 路径 ----
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT" || exit 2

echo "REPO_ROOT: $REPO_ROOT"
echo "TARGET: $TARGET  ONLY: $ONLY  VERBOSE: $VERBOSE  KEEP_GOING: $KEEP_GOING"

# ---- helper ----
run_suite() {
    local desc="$1"
    shift
    local args=("$@")
    echo
    echo "=== $desc ==="
    echo "  cargo test ${args[*]}"
    if [[ $VERBOSE -eq 1 ]]; then
        cargo test "${args[@]}"
    else
        cargo test "${args[@]}" --quiet
    fi
    local rc=$?
    if [[ $rc -ne 0 ]]; then
        echo "FAIL: $desc (exit=$rc)"
        if [[ $KEEP_GOING -eq 0 ]]; then
            exit $rc
        fi
        FAILED=1
    else
        echo "PASS: $desc"
    fi
}

FAILED=0

# ============================================================
# §1 cats-mock src 单测 (lib, ~91 cases)
# ============================================================
if [[ "$ONLY" == "all" || "$ONLY" == "src" ]]; then
    run_suite "cats-mock src 单测 (lib, ~91 cases)" \
        -p cats-mock --lib --no-fail-fast
fi

# ============================================================
# §2 cats-mock UT 整合 (tests/ut_mock_basics.rs)
# ============================================================
if [[ "$ONLY" == "all" || "$ONLY" == "ut_mock_basics" ]]; then
    run_suite "cats-mock UT 整合 (tests/ut_mock_basics.rs)" \
        -p cats-mock --test ut_mock_basics --no-fail-fast
fi

# ============================================================
# §3 cats-mock IT (tests/it_e2e_compose.rs)
# ============================================================
if [[ "$ONLY" == "all" || "$ONLY" == "it_e2e_compose" ]]; then
    run_suite "cats-mock IT (tests/it_e2e_compose.rs)" \
        -p cats-mock --test it_e2e_compose --no-fail-fast
fi

# ============================================================
# §4 cats-mock ST 回归 (tests/st_regression.rs)
# ============================================================
if [[ "$ONLY" == "all" || "$ONLY" == "st_regression" ]]; then
    run_suite "cats-mock ST 回归 (tests/st_regression.rs)" \
        -p cats-mock --test st_regression --no-fail-fast
fi

# ============================================================
# §5 (可选) 16 service smoke 全 workspace 回归
# ============================================================
if [[ "$TARGET" == "all" && "$ONLY" == "all" ]]; then
    run_suite "全 workspace cargo test --workspace" \
        --workspace --no-fail-fast
fi

# ============================================================
# §6 汇总
# ============================================================
echo
echo "=== 回归测试汇总 ==="
cat <<EOF
  套件                   状态
  ─────────────────────────────────────
  src 单测 (~91)         $(if [[ $FAILED -eq 0 ]]; then echo PASS; else echo FAIL; fi)
  tests/ut_mock_basics   $(if [[ $FAILED -eq 0 ]]; then echo PASS; else echo FAIL; fi)
  tests/it_e2e_compose   $(if [[ $FAILED -eq 0 ]]; then echo PASS; else echo FAIL; fi)
  tests/st_regression    $(if [[ $FAILED -eq 0 ]]; then echo PASS; else echo FAIL; fi)

引用: CATs_测试Mock项目设计书_v1.0.md §5.4 (验收门槛: 89/89)
      ULYS-135 回归测试任务交付
EOF

if [[ $FAILED -ne 0 ]]; then
    exit 1
fi
exit 0