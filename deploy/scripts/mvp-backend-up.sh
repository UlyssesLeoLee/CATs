#!/usr/bin/env bash
# mvp-backend-up.sh - CATs MVP backend 一键启动 (per 9/4 17:47 mock 项目偏好, quick 模式)
# 用法: bash deploy/scripts/mvp-backend-up.sh
#
# 启动 17 个容器: postgres + kafka + 2 个 oneshot 初始化 + ai-gateway
#                 + 8 core service + translation-core + worker + bff + envoy
#
# 数据面初始化已内建到 compose（见下方说明），本脚本不再手工建库。

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
COMPOSE=(docker compose -f "${REPO_ROOT}/deploy/docker-compose-mvp.yml")
HOST="127.0.0.1"

echo "==> [1/4] 检查 docker + docker compose"
docker --version
docker compose version

# 原脚本写成 `cd "$(dirname "$0")/.."` 然后用 `deploy/docker-compose-mvp.yml`，
# 路径会解析成 deploy/deploy/... —— 那个文件不存在。改为直接算仓库根。
echo "==> [2/4] 构建 runtime 镜像（首次较慢，Rust 全量 release）"
"${COMPOSE[@]}" build

echo "==> [3/4] 启动全部 service"
# db-init / kafka-init 是 compose 里的一等 oneshot service，会被自动拉起并
# 按 depends_on 顺序等待完成：
#   db-init    -> 建 8 个 logical database + 逐个应用 migrations + 探针校验
#   kafka-init -> 建 cats.notifications.v1 / cats.audit.v1 两个 topic
# 少了它们，服务能起、healthz 能过，但第一个业务请求就是
# relation does not exist / consumer 永远收不到消息。
"${COMPOSE[@]}" up -d

echo "==> [4/4] 等待并逐个探活"
"${COMPOSE[@]}" ps

echo ""
echo "==> 端到端 smoke（每个 service 自己的 /healthz）"
fail=0
probe() {
  local name="$1" port="$2"
  if curl -sf --max-time 5 "http://${HOST}:${port}/healthz" >/dev/null 2>&1; then
    printf '  %-24s OK   (127.0.0.1:%s)\n' "$name" "$port"
  else
    printf '  %-24s FAIL (127.0.0.1:%s)\n' "$name" "$port"
    fail=1
  fi
}

# 端口表与 docker-compose-mvp.yml 的 ports 声明一一对应。
# envoy 是客户端统一入口，走 8080（原先映射在 10000，与注释和本文档都不符）。
probe "envoy (客户端入口)"      8080
probe "auth-service"            8081
probe "user-service"            8082
probe "project-service"         8083
probe "task-service"            8084
probe "file-service"            8085
probe "notification-service"    8086
probe "report-service"          8087
probe "audit-service"           8088
probe "worker-service"          8089
probe "cats-ai-gateway"         8090
probe "cats-bff"                8091

echo ""
if [ "$fail" -ne 0 ]; then
  echo "!! 有 service 未通过 /healthz。逐个看日志：" >&2
  echo "   ${COMPOSE[*]} logs --tail=100 <service>" >&2
  exit 1
fi

echo "==> MVP BACKEND UP"
echo ""
echo "客户端接入地址 (envoy 边缘): http://localhost:8080"
echo "直接 service 端口: 8081-8091 (dev 用)"
