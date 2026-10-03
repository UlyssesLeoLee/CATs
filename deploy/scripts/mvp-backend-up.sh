#!/usr/bin/env bash
# mvp-backend-up.sh - CATs MVP backend 一键启动 (per 9/4 17:47 mock 项目偏好,quick 模式)
# 用法: bash deploy/scripts/mvp-backend-up.sh
#
# 启动: postgres + kafka + ai-gateway + 8 services + translation-core + worker + bff + envoy

set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> [1/6] 检查 docker + docker compose"
docker --version
docker compose version

echo "==> [2/6] 创建 logical databases (8 service 库)"
docker compose -f deploy/docker-compose-mvp.yml up -d postgres
sleep 5
for db in auth_db user_db project_db task_db file_db notification_db report_db audit_db; do
  echo "    CREATE DATABASE $db"
  docker exec cats-mvp-pg psql -U postgres -d postgres -c "CREATE DATABASE $db" 2>&1 | grep -v "already exists" || true
done

echo "==> [3/6] 启 kafka (等待 ready)"
docker compose -f deploy/docker-compose-mvp.yml up -d kafka
sleep 10
docker compose -f deploy/docker-compose-mvp.yml ps kafka

echo "==> [4/6] 启 cats-ai-gateway + translation-core"
docker compose -f deploy/docker-compose-mvp.yml up -d ai-gateway translation-core
sleep 5

echo "==> [5/6] 启 8 核心 service + bff"
docker compose -f deploy/docker-compose-mvp.yml up -d \
    auth-service user-service project-service task-service file-service \
    notification-service report-service audit-service worker-service cats-bff
sleep 10

echo "==> [6/6] 启 envoy 边缘"
docker compose -f deploy/docker-compose-mvp.yml up -d envoy
sleep 3

echo ""
echo "==> 验证 (10 秒后端到端 smoke)"
sleep 10
curl -sf http://localhost:8080/healthz && echo " [envoy OK]" || echo " [envoy FAIL]"
curl -sf http://localhost:8081/healthz && echo " [auth-service OK]" || echo " [auth-service FAIL]"
curl -sf http://localhost:8090/healthz && echo " [ai-gateway OK]" || echo " [ai-gateway FAIL]"

echo ""
echo "==> MVP BACKEND UP"
echo ""
echo "客户端接入地址 (envoy 边缘): http://localhost:8080"
echo "直接 service 端口: 8081-8090 (dev 用)"