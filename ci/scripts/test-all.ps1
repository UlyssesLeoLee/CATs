# CATs M0 全 workspace 测试脚本
# 引用: 完成判据 4（cargo test --workspace --all-features）
#
# 用法: pwsh ci/scripts/test-all.ps1
# 等价于: cargo test --workspace --all-features --locked -- --nocapture

$ErrorActionPreference = 'Stop'

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$repoRoot  = Split-Path -Parent (Split-Path -Parent $scriptDir)

Push-Location $repoRoot
try {
    Write-Host "==> cargo test --workspace --all-features" -ForegroundColor Cyan
    cargo test --workspace --all-features --locked -- --nocapture
    if ($LASTEXITCODE -ne 0) {
        Write-Host "FAIL: cargo test returned $LASTEXITCODE" -ForegroundColor Red
        exit $LASTEXITCODE
    }
    Write-Host "==> test-all OK" -ForegroundColor Green
}
finally {
    Pop-Location
}

exit 0
