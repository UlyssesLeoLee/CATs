# CATs M0 本地开发环境启动脚本（占位）
# 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1（PG 18.6 + CNPG 1.30+ + pgvector 0.8.6）
# 引用: doc/02-基础设计/技术选型/CATs_技术选型书_v2.0.md §2/§5
#
# M0 阶段：占位脚本（per 任务："dev-up.ps1 打印 TODO"）
# M1-S0 阶段：填实 K3s + CNPG 部署逻辑
#
# 预期实施（M1-S0）：
#   1. kind / k3d 启动 K3s 集群
#   2. helm install cnpg 云原生 PG operator
#   3. 部署 dev/cnpg-cluster.yaml（PG 18.6 + pgvector 0.8.6，1 实例）
#   4. 端口转发 5432 给本地 psql
#   5. 等待 cluster ready（kubectl wait cnpg/cluster-ready）

$ErrorActionPreference = 'Stop'

Write-Host "================================================" -ForegroundColor Cyan
Write-Host " CATs M0 dev-up.ps1 (placeholder)               " -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "TODO: 启动 PG 18.6 + CNPG（per RGS-PGV18-INSTALL-SOP）" -ForegroundColor Yellow
Write-Host ""
Write-Host "M0 阶段不实施真实部署；M1-S0 阶段填实以下步骤："
Write-Host "  1. kind/k3d 启动 K3s 集群（1 control-plane + 0 worker 即可）"
Write-Host "  2. helm install cnpg cloudnative-pg v1.30+"
Write-Host "  3. kubectl apply -f deploy/cnpg/cats-dev-cluster.yaml"
Write-Host "     (PG 18.6 + pgvector 0.8.6，1 instance，size=1Gi)"
Write-Host "  4. kubectl wait cnpg/cats-dev --for=condition=Ready --timeout=180s"
Write-Host "  5. kubectl port-forward svc/cats-dev-rw 5432:5432 &"
Write-Host ""
Write-Host "完成判据：psql -h 127.0.0.1 -U cats -d cats -c 'SELECT version();'"
Write-Host "应返回 'PostgreSQL 18.6 ...'"
Write-Host ""

# 退出码 0（M0 占位即为成功）
exit 0
