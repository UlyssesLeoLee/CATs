#!/usr/bin/env bash
# init-databases.sh — 建 8 个 logical database 并逐 service 应用 migrations
#
# 为什么需要它（实证见 deploy/COMPOSE_UP_DEFECTS_v1.0.md）：
#   1. compose 只建了默认库 postgres，而 8 个 service 的 DATABASE_URL
#      各自指向 auth_db / user_db / ... 没有人创建它们
#   2. 全仓 crates/*/src/main.rs 搜 "migrat" 是 0 匹配——没有任何 service
#      在启动时跑 migration。库建出来也是空的
#
# migrations/ 目录是 schema 的唯一权威来源，按文件名排序依次应用。
# 本脚本同时被 docker compose（db-init service）与 CI 的 e2e job 使用，
# 避免"本地能起、CI 起不来"这类分叉。
#
# 用法:
#   PGHOST=postgres PGPASSWORD=xxx bash deploy/scripts/init-databases.sh
#
# 环境变量:
#   PGHOST     默认 postgres（容器网络内）；本机跑用 127.0.0.1
#   PGPORT     默认 5432
#   PGUSER     默认 postgres
#   PGPASSWORD 由调用方提供（不设默认值——凭据不写进脚本）
#   REPO_ROOT  migrations 所在仓库根，默认按脚本位置推断

set -euo pipefail

PGHOST="${PGHOST:-postgres}"
PGPORT="${PGPORT:-5432}"
PGUSER="${PGUSER:-postgres}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

# service 名 -> logical database
# 与 docker-compose-mvp.yml 各 service 的 DATABASE_URL 保持一致
declare -A DBS=(
  [auth]="auth_db"
  [user]="user_db"
  [project]="project_db"
  [task]="task_db"
  [file]="file_db"
  [notification]="notification_db"
  [audit]="audit_db"
)

# report-service 复用 task_db（见 compose：report-service 的 DATABASE_URL）
EXTRA_ALIASES=("report:task_db")

psql_admin() {
  psql -v ON_ERROR_STOP=1 -h "$PGHOST" -p "$PGPORT" -U "$PGUSER" -d postgres "$@"
}

echo "==> [1/3] 创建 logical database (host=$PGHOST:$PGPORT)"
for svc in "${!DBS[@]}"; do
  db="${DBS[$svc]}"
  # IF NOT EXISTS 在 PostgreSQL 里不适用于 CREATE DATABASE，
  # 所以先查 pg_database 再建——这样脚本可以重复执行（幂等）
  exists=$(psql_admin -tAc "SELECT 1 FROM pg_database WHERE datname='${db}'" || true)
  if [ "$exists" = "1" ]; then
    echo "    ${db} 已存在，跳过"
  else
    echo "    CREATE DATABASE ${db}"
    psql_admin -qc "CREATE DATABASE \"${db}\"" >/dev/null
  fi
done

echo "==> [2/3] 应用 migrations"
# 顺序很关键：migrations 文件名带序号，必须按字典序依次应用。
# 早期版本的"三份 init 互相冲突"问题（见 deploy/E2E_FINDINGS_v1.0.md §1）
# 已通过停用与生产代码不符的旧 init 解决，因此顺序应用不再冲突。
for svc in "${!DBS[@]}"; do
  db="${DBS[$svc]}"
  dir="${REPO_ROOT}/crates/${svc}-service/migrations"
  if [ ! -d "$dir" ]; then
    echo "    !! ${dir} 不存在，跳过 ${svc}"
    continue
  fi
  count=0
  while IFS= read -r f; do
    echo "    [${svc}/${db}] $(basename "$f")"
    psql -v ON_ERROR_STOP=1 -q -h "$PGHOST" -p "$PGPORT" -U "$PGUSER" \
         -d "$db" -f "$f"
    count=$((count + 1))
  done < <(find "$dir" -name '*.sql' | sort)
  echo "    [${svc}/${db}] 应用 ${count} 个 migration 文件"
done

echo "==> [3/3] 校验关键表存在"
# 不靠"没报错"判定成功——那只能证明没有 syntax error，不能证明表建出来了。
# 这里逐 service 查一张代表性表，把"跑完了"和"建对了"分开。
declare -A PROBE=(
  [auth]="users_credential"
  [user]="user_profile"
  [project]="projects"
  [task]="tasks"
  [file]="files"
  [notification]="notifications"
  [audit]="audit_logs"
)
fail=0
for svc in "${!PROBE[@]}"; do
  db="${DBS[$svc]}"
  tbl="${PROBE[$svc]}"
  n=$(psql -tAc "SELECT count(*) FROM information_schema.tables
                 WHERE table_schema='public' AND table_name='${tbl}'" \
         -h "$PGHOST" -p "$PGPORT" -U "$PGUSER" -d "$db" 2>/dev/null || echo 0)
  if [ "$n" = "1" ]; then
    echo "    OK   ${svc}/${db}.${tbl}"
  else
    echo "    FAIL ${svc}/${db}.${tbl} 不存在"
    fail=1
  fi
done

if [ "$fail" -ne 0 ]; then
  echo ""
  echo "!! 初始化未完成：有表缺失。服务起来后首个业务请求会 relation does not exist。" >&2
  exit 1
fi

echo ""
echo "==> 数据库初始化完成（8 库 + migrations 全部应用并逐表校验）"
