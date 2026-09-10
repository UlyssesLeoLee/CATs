$ErrorActionPreference = 'Stop'
$crates = @(
    'asr-service','audit-service','auth-service','cats-bff','file-service',
    'ingestion-service','notification-service','ocr-service','office-converter-service',
    'project-service','render-writer-service','report-service','subtitle-service',
    'task-service','translation-core','user-service','worker-service'
)

foreach ($c in $crates) {
    $path = "D:\CATs\crates\$c\Cargo.toml"
    $content = Get-Content -Path $path -Raw
    if ($content -match 'cats-mock\.workspace') {
        Write-Host "ALREADY: $c"
        continue
    }
    # 在最后一行前插入 dev-dependency
    # 文件末尾通常是 [lints] 段或 dependencies 段, 简单做法: 在文件末尾追加
    $appendix = @"

[dev-dependencies]
cats-mock = { workspace = true }
"@
    Add-Content -Path $path -Value $appendix
    Write-Host "APPENDED: $c"
}
Write-Host "DONE"
