# CATs M0 proto 编译脚本
# 引用: crates/proto/build.rs（tonic-build 调用）
# 引用: proto/cats/v1/*.proto
#
# 用法: pwsh ci/scripts/proto-gen.ps1
# 等价于: cd crates/proto && cargo build
#
# 作用:
#   1. cd 到 crates/proto
#   2. cargo build 触发 build.rs
#   3. build.rs 调用 tonic-build 编译 proto/cats/v1/*.proto
#   4. 生成代码在 OUT_DIR（不入仓）

$ErrorActionPreference = 'Stop'

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$repoRoot  = Split-Path -Parent (Split-Path -Parent $scriptDir)

Push-Location "$repoRoot/crates/proto"
try {
    Write-Host "==> Compiling proto via tonic-build..." -ForegroundColor Cyan
    cargo build
    if ($LASTEXITCODE -ne 0) {
        Write-Host "FAIL: cargo build returned $LASTEXITCODE" -ForegroundColor Red
        exit $LASTEXITCODE
    }
    Write-Host "==> proto compile OK" -ForegroundColor Green
}
finally {
    Pop-Location
}

exit 0
