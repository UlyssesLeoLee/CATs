# CATs M1-S0 本地开发环境启动脚本
# 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1（PG 18.6 + pgvector 0.8.6）
# 引用: docs/01-需求/部署记录/CATs_OI-4_PG_18.6_实测报告_v1.0.md
# 引用: doc/02-基础设计/技术选型/CATs_技术选型书_v2.0.md §2/§5
#
# M1-S0 阶段: 已填实 WSL Ubuntu 24.04 上装 PG 18.6 + pgvector 0.8.6 + 建 8 逻辑库 + 8 user
# M1-Sprint 0+ 阶段: 升级为 kind/k3d + CNPG 1.30+ (per 微服务架构设计 §5.6)
#
# 前置: $env:UbuntuPW 已设（per 2026-08-27 11:06 JST 安全约束，pipe 模式不打印）
#   - 推荐: $env:UbuntuPW | wsl --user root --exec bash -c '...'
# 完成判据: SELECT version() 返回 'PostgreSQL 18.6 ...'

$ErrorActionPreference = 'Stop'

Write-Host "================================================" -ForegroundColor Cyan
Write-Host " CATs M1-S0 dev-up.ps1 (WSL PG 18.6 + pgvector) " -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""

if (-not $env:UbuntuPW) {
    Write-Host "ERROR: `$env:UbuntuPW 未设置（per 2026-08-27 11:06 JST 安全约束）" -ForegroundColor Red
    Write-Host "请先: `$env:UbuntuPW = 'YOUR_PASSWORD'" -ForegroundColor Yellow
    exit 1
}

# 验证 WSL 可用
$wslStatus = wsl --status 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: wsl 不可用" -ForegroundColor Red
    exit 1
}

# Step 1: 加 PGDG 源（幂等）
Write-Host "Step 1: 加 PGDG 源" -ForegroundColor Green
$env:UbuntuPW | wsl --user root --distribution Ubuntu --exec bash -c @"
set -e
curl -fsSL https://www.postgresql.org/media/keys/ACCC4CF8.asc | \
  gpg --dearmor -o /usr/share/keyrings/pgdg-keyring.gpg 2>/dev/null
if [ ! -f /etc/apt/sources.list.d/pgdg.list ]; then
  echo 'deb [signed-by=/usr/share/keyrings/pgdg-keyring.gpg] http://apt.postgresql.org/pub/repos/apt noble-pgdg main' > /etc/apt/sources.list.d/pgdg.list
fi
apt-get update -qq
"@

# Step 2: 装 PG 18.6 + pgvector 0.8.6
Write-Host "Step 2: 装 postgresql-18 + postgresql-18-pgvector" -ForegroundColor Green
$env:UbuntuPW | wsl --user root --distribution Ubuntu --exec bash -c @"
set -e
DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
  postgresql-18 postgresql-18-pgvector
pg_ctlcluster 18 main start || service postgresql start
"@

# Step 3: 建 8 逻辑库 + 8 user
Write-Host "Step 3: 建 8 逻辑库 + 8 user（per Baseline §5.1）" -ForegroundColor Green
$initSqlPath = 'C:\Users\leo19\.minimax\cats-pg-init.sql'
$wslInit = wsl --user root --distribution Ubuntu --exec bash -c 'mktemp -t cats-init.XXXXXX.sql'
Get-Content -LiteralPath $initSqlPath -Raw | wsl --user root --distribution Ubuntu --exec bash -c "cat > $wslInit && chmod 644 $wslInit && chown postgres:postgres $wslInit"
$env:UbuntuPW | wsl --user root --distribution Ubuntu --exec bash -c "sudo -u postgres psql -v ON_ERROR_STOP=1 -f $wslInit"

# Step 4: 在 project_db 装 pgvector 扩展
Write-Host "Step 4: 在 project_db 装 pgvector 0.8.6" -ForegroundColor Green
$env:UbuntuPW | wsl --user root --distribution Ubuntu --exec bash -c "sudo -u postgres psql -d project_db -c 'CREATE EXTENSION IF NOT EXISTS vector;'"

# Step 5: 验收
Write-Host "Step 5: 验收" -ForegroundColor Green
$verifyScript = @'
SELECT version();
SELECT extname, extversion FROM pg_extension WHERE extname = 'vector';
SELECT datname FROM pg_database WHERE datname LIKE '%_db' ORDER BY datname;
'@
$env:UbuntuPW | wsl --user root --distribution Ubuntu --exec bash -c "sudo -u postgres psql -t -c \"$verifyScript\""

Write-Host ""
Write-Host "完成！PG 18.6 + pgvector 0.8.6 + 8 逻辑库就绪" -ForegroundColor Green
Write-Host "下一步: cargo test -p cats-m1-s0-smoke（验证 Rust 1.98.0 兼容性）" -ForegroundColor Cyan
exit 0
