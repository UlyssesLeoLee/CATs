$ErrorActionPreference = 'Stop'
$crates = @(
    'asr-service','audit-service','auth-service','cats-bff','file-service',
    'ingestion-service','notification-service','ocr-service','office-converter-service',
    'project-service','render-writer-service','report-service','subtitle-service',
    'task-service','translation-core','user-service','worker-service'
)

foreach ($c in $crates) {
    $path = "D:\CATs\crates\$c\tests\smoke.rs"
    if (-not (Test-Path $path)) {
        Write-Host "SKIP: $path not found"
        continue
    }
    $body = @"
//! $c smoke test
//!
//! 引用: ci/github-actions/ci-rust-test.yaml 要求 workspace 至少 1 test / crate
//! 改造: 2026-09-01 per CATs_测试Mock项目设计书_v1.0 §5.2, 16 行 boilerplate 改 1 行宏调用

use cats_mock::smoke::name_matches_crate;

name_matches_crate!(env!("CARGO_PKG_NAME"));
"@
    Set-Content -Path $path -Value $body -Encoding utf8 -NoNewline
    Write-Host "WROTE: $path"
}
Write-Host "DONE: $($crates.Count) files"
