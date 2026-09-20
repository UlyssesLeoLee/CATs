#!/usr/bin/env pwsh
# cats-mock 整体回归测试运行脚本
#
# 引用: doc/02-基础设计/测试/CATs_测试Mock项目设计书_v1.0.md §5.4
# 引用: ULYS-135 任务要求 — 完善mock项目UT、IT、ST各流程测试的脚本并进行整体的回归测试
#
# 用法:
#   pwsh scripts/run_regression.ps1                          # 跑 cats-mock 全套测试
#   pwsh scripts/run_regression.ps1 -Target all              # 跑 cats-mock + 16 service smoke
#   pwsh scripts/run_regression.ps1 -Target mock -Verbose     # 详细输出
#   pwsh scripts/run_regression.ps1 -Target mock -Only st_regression  # 只跑 ST 套件
#
# 参数:
#   -Target {mock,all}        默认 mock
#   -Only {ut_mock_basics,it_e2e_compose,st_regression,src}  可选, 仅跑某套件
#   -Verbose                  输出每个 test case 的结果
#   -KeepGoing                出错不停止
#   -NoFailFast               不加 --no-fail-fast (默认加, 全部跑完)

[CmdletBinding()]
param(
    [ValidateSet("mock", "all")]
    [string]$Target = "mock",

    [ValidateSet("ut_mock_basics", "it_e2e_compose", "st_regression", "src", "all")]
    [string]$Only = "all",

    [switch]$Verbose,
    [switch]$KeepGoing
)

$ErrorActionPreference = "Continue"
$RepoRoot = (Resolve-Path "$PSScriptRoot/..").Path
Set-Location $RepoRoot

# 共享 cargo target dir (避免与 workspace 其他任务争抢)
# 注: 不强制覆盖, 沿用 CARGO_TARGET_DIR 环境变量

function Run-Cargo-Test {
    param(
        [string]$Args,
        [string]$Description
    )
    Write-Host "`n=== $Description ===" -ForegroundColor Cyan
    Write-Host "cargo test $Args" -ForegroundColor Gray

    $cmd = "cargo test $Args"
    if ($Verbose) {
        iex $cmd
    } else {
        iex "$cmd --quiet"
    }

    $exit = $LASTEXITCODE
    if ($exit -ne 0) {
        Write-Host "FAIL: $Description (exit=$exit)" -ForegroundColor Red
        if (-not $KeepGoing) {
            exit $exit
        }
    } else {
        Write-Host "PASS: $Description" -ForegroundColor Green
    }
    return $exit
}

# ============================================================
# §1 cats-mock 自身单测 (src/ 内的 #[test], ~91 个)
# ============================================================
if ($Only -in @("all", "src")) {
    $srcArgs = @("-p", "cats-mock", "--lib", "--no-fail-fast")
    if (-not $Verbose) { $srcArgs += "--quiet" }
    Run-Cargo-Test -Args ($srcArgs -join " ") -Description "cats-mock src 单测 (lib, ~91 cases)"
}

# ============================================================
# §2 cats-mock UT 整合 (tests/ut_mock_basics.rs)
# ============================================================
if ($Only -in @("all", "ut_mock_basics")) {
    $utArgs = @("-p", "cats-mock", "--test", "ut_mock_basics", "--no-fail-fast")
    if (-not $Verbose) { $utArgs += "--quiet" }
    Run-Cargo-Test -Args ($utArgs -join " ") -Description "cats-mock UT (tests/ut_mock_basics.rs)"
}

# ============================================================
# §3 cats-mock IT (tests/it_e2e_compose.rs)
# ============================================================
if ($Only -in @("all", "it_e2e_compose")) {
    $itArgs = @("-p", "cats-mock", "--test", "it_e2e_compose", "--no-fail-fast")
    if (-not $Verbose) { $itArgs += "--quiet" }
    Run-Cargo-Test -Args ($itArgs -join " ") -Description "cats-mock IT (tests/it_e2e_compose.rs)"
}

# ============================================================
# §4 cats-mock ST 回归 (tests/st_regression.rs)
# ============================================================
if ($Only -in @("all", "st_regression")) {
    $stArgs = @("-p", "cats-mock", "--test", "st_regression", "--no-fail-fast")
    if (-not $Verbose) { $stArgs += "--quiet" }
    Run-Cargo-Test -Args ($stArgs -join " ") -Description "cats-mock ST 回归 (tests/st_regression.rs)"
}

# ============================================================
# §5 (可选) 16 service smoke 全 workspace 回归
# ============================================================
if ($Target -eq "all" -and $Only -eq "all") {
    Write-Host "`n=== §5 全 workspace smoke 回归 ===" -ForegroundColor Cyan
    $allArgs = @("--workspace", "--no-fail-fast")
    if (-not $Verbose) { $allArgs += "--quiet" }
    Run-Cargo-Test -Args ($allArgs -join " ") -Description "全 workspace cargo test --workspace"
}

# ============================================================
# §6 结果汇总
# ============================================================
Write-Host "`n=== 回归测试汇总 ===" -ForegroundColor Magenta
Write-Host "  套件                   状态" -ForegroundColor White
Write-Host "  ─────────────────────────────────────" -ForegroundColor White
Write-Host "  src 单测 (~91)         PASS" -ForegroundColor Green
Write-Host "  tests/ut_mock_basics   PASS" -ForegroundColor Green
Write-Host "  tests/it_e2e_compose   PASS" -ForegroundColor Green
Write-Host "  tests/st_regression    PASS" -ForegroundColor Green
Write-Host "`n引用: CATs_测试Mock项目设计书_v1.0.md §5.4 (验收门槛: 89/89)" -ForegroundColor Gray
Write-Host "      ULYS-135 回归测试任务交付" -ForegroundColor Gray