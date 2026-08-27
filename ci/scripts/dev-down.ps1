# CATs M0 本地开发环境关闭脚本（占位）
# 引用: ci/scripts/dev-up.ps1

$ErrorActionPreference = 'Stop'

Write-Host "================================================" -ForegroundColor Cyan
Write-Host " CATs M0 dev-down.ps1 (placeholder)              " -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "TODO: 关闭 PG 18.6 + CNPG（per dev-up.ps1 实施）" -ForegroundColor Yellow
Write-Host ""
Write-Host "M1-S0 阶段填实：kubectl delete -f deploy/cnpg/cats-dev-cluster.yaml"
Write-Host "或 kind delete cluster --name cats-dev"
Write-Host ""

exit 0
