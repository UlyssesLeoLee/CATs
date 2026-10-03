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
COMPOSE_FILE="${REPO_ROOT}/deploy/docker-compose-mvp.yml"
# compose 的 project 名决定命名卷的前缀（<project>_cats-mvp-pgdata）。
# 显式固定，避免"在不同目录下跑同名文件得到不同卷"这种隐式差异。
PROJECT="${CATS_COMPOSE_PROJECT:-cats-mvp}"
COMPOSE=(docker compose -p "${PROJECT}" -f "${COMPOSE_FILE}")
HOST="127.0.0.1"

echo "==> [1/5] 检查 docker + docker compose"
docker --version
docker compose version

# ---------------------------------------------------------------------
# 预检：PG 数据卷是不是陈旧的
# ---------------------------------------------------------------------
# PostgreSQL **只在首次初始化空数据目录时**读 POSTGRES_PASSWORD。一旦卷
# 里已经有数据目录，它会被完全忽略——于是 compose 里写的密码不生效，
# 报错只剩一句含糊的：
#   FATAL:  password authentication failed for user "postgres"
# 没有任何线索提示"是你的卷太旧了"。实测踩过：卷 deploy_cats-mvp-pgdata
# 创建于 2026-09-19，换了 compose 密码后 up 就再也连不上。
#
# 所以这里提前检查并给出可执行的提示，而不是让人对着一句认证失败猜。
# ---------------------------------------------------------------------
echo "==> [2/5] 预检 PG 数据卷（project=${PROJECT}）"
PGVOL="${PROJECT}_cats-mvp-pgdata"
if docker volume inspect "$PGVOL" >/dev/null 2>&1; then
  created=$(docker volume inspect "$PGVOL" --format '{{.CreatedAt}}' 2>/dev/null || echo "unknown")
  echo "    找到既有卷 $PGVOL (创建于 ${created})"
  if [ -z "${CATS_FORCE_RECREATE_PG:-}" ]; then
    echo ""
    echo "    ⚠ 这个卷已经存在。若 $PGVOL 连不上，原因是 PostgreSQL 只在首次"
    echo "      初始化空数据目录时读 POSTGRES_PASSWORD，已有数据目录会忽略它。"
    echo "      数据可丢弃时用下面这条重建（会清空该卷里的所有开发数据）："
    echo ""
    echo "        CATS_FORCE_RECREATE_PG=1 bash deploy/scripts/mvp-backend-up.sh"
    echo ""
    echo "      数据要保留则跳过本脚本，手工确认卷里的密码。"
  else
    echo "    CATS_FORCE_RECREATE_PG 已设置 -> 重建卷（数据将丢失）"
    "${COMPOSE[@]}" down -v >/dev/null 2>&1 || true
  fi
else
  echo "    无既有卷，将新建"
fi

echo "==> [3/5] 构建 runtime 镜像（首次较慢，Rust 全量 release）"
"${COMPOSE[@]}" build

echo "==> [4/5] 启动全部 service"
# db-init / kafka-init 是 compose 里的一等 oneshot service，会被自动拉起并
# 按 depends_on 顺序等待完成：
#   db-init    -> 建 8 个 logical database + 逐个应用 migrations + 探针校验
#   kafka-init -> 建 cats.notifications.v1 / cats.audit.v1 两个 topic
# 少了它们，服务能起、healthz 能过，但第一个业务请求就是
# relation does not exist / consumer 永远收不到消息。
"${COMPOSE[@]}" up -d

echo "==> [5/5] 等待并逐个探活"
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

# 探针清单**从 compose 编排里推导**，不再手写。
#
# 真实事故（2026-10-04）: 这份清单原先是手写的 12 行，漏掉了
# translation-core —— 而它恰好是当时唯一坏掉的 service（实际监听 8090，
# 而所有依赖方都指着 50051）。12 路全 200 与"17 容器 Up"并排写在同一节里，
# 读起来像覆盖了全部目标，其实没有。
#
# 手写清单的失败模式是**静默的**：漏掉一个目标不会让脚本失败，只会让
# 报告少一行。所以清单必须由编排本身生成，并且和 lint-compose.py 的
# MUST_DECLARE_COMMAND 用同一份 service 名单。
#
# 端口取 compose 里每个 service 的**宿主端口**（ports 声明的倒数第二段），
# 改端口时不必再回来改这个脚本。
probe_all() {
  # service  ->  宿主端口
  "${COMPOSE[@]}" config --format json 2>/dev/null \
  | python3 -c '
import json, sys
doc = json.load(sys.stdin)
svcs = doc.get("services") or {}

def serves_healthz(name, s):
    """只有真的提供 HTTP /healthz 的才探。

    postgres / kafka 也声明了 ports，但它们不提供 /healthz —— 一并探会
    让脚本永远失败，而那不是"服务坏了"，是探针问错了对象。
    判据用「command 指向 /usr/local/bin/<binary>」（即 12 个应用 service）
    外加 envoy（它的 /healthz 是 direct_response）。
    """
    if name == "envoy":
        return True
    cmd = s.get("command") or []
    if isinstance(cmd, str):
        cmd = [cmd]
    return any(str(c).startswith("/usr/local/bin/") for c in cmd)

rows = []
for name, s in svcs.items():
    if not serves_healthz(name, s):
        continue
    ports = s.get("ports") or []
    if not ports:
        continue
    p = ports[0]
    if isinstance(p, dict):
        p = "%s:%s:%s" % (p.get("host_ip", ""), p.get("published", ""),
                           p.get("target", ""))
    parts = str(p).split(":")
    if len(parts) < 2 or not parts[-2].isdigit():
        continue
    rows.append((name, int(parts[-2])))
for name, port in sorted(rows, key=lambda r: r[1]):
    print("%s\t%d" % (name, port))
' 2>/dev/null
}

probed=0
while IFS=$'\t' read -r pname pport; do
  [ -n "$pport" ] || continue
  probed=$((probed + 1))
  probe "$pname" "$pport"
done <<< "$(probe_all)"

if [ "$probed" -eq 0 ]; then
  echo "!! 一个 service 都没探到 —— 探针清单推导失败，不要把'0 失败'当成通过" >&2
  exit 1
fi

echo ""
echo "  （共探活 ${probed} 个 service，宿主端口来自 compose 编排）"

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
