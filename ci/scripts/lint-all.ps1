# CATs M0 全 workspace lint 脚本
# 引用: 完成判据 2/3/5
#
# 用法: pwsh ci/scripts/lint-all.ps1
# 等价于:
#   cargo fmt --all -- --check
#   RUSTFLAGS='-D warnings' cargo clippy --workspace --all-features --all-targets --locked
#   cargo deny check licenses

$ErrorActionPreference = 'Stop'

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$repoRoot  = Split-Path -Parent (Split-Path -Parent $scriptDir)

Push-Location $repoRoot
try {
    # --- 1. fmt check ---
    Write-Host "==> [1/3] cargo fmt --all -- --check" -ForegroundColor Cyan
    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) {
        Write-Host "FAIL: fmt check returned $LASTEXITCODE" -ForegroundColor Red
        exit $LASTEXITCODE
    }

    # --- 2. clippy with -D warnings ---
    Write-Host "==> [2/3] cargo clippy -- -D warnings" -ForegroundColor Cyan
    $env:RUSTFLAGS = '-D warnings'
    cargo clippy --workspace --all-features --all-targets --locked
    if ($LASTEXITCODE -ne 0) {
        Write-Host "FAIL: clippy returned $LASTEXITCODE" -ForegroundColor Red
        exit $LASTEXITCODE
    }

    # --- 3. cargo deny licenses ---
    Write-Host "==> [3/3] cargo deny check licenses" -ForegroundColor Cyan
    # 仅在 cargo-deny 已安装时执行；CI runner 上预装，本地若无则跳过
    if (Get-Command cargo-deny -ErrorAction SilentlyContinue) {
        cargo deny check licenses
        if ($LASTEXITCODE -ne 0) {
            Write-Host "FAIL: cargo deny returned $LASTEXITCODE" -ForegroundColor Red
            exit $LASTEXITCODE
        }
    } else {
        Write-Host "SKIP: cargo-deny not installed locally. Run on CI runner." -ForegroundColor Yellow
    }

    Write-Host "==> lint-all OK" -ForegroundColor Green
}
finally {
    Pop-Location
    Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue
}

exit 0
