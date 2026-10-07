#!/usr/bin/env bash
# =====================================================================
# CATs MVP 数据层 — migrate_all.sh
# owner: mvp-data worker (per 9/19 JST task)
# 跑全 8 逻辑库 DDL 到本地 docker pgvector:pg18 实例
# =====================================================================
# 用法:
#   docker compose -f crates/cats-mock/k3s/db/docker-compose.yml up -d
#   bash crates/cats-mock/scripts/migrate_all.sh
#
# exit 0 = 全 8 库 schema 落地成功
# exit 非 0 = 任意 psql -f 失败 (脚本立即退出)
# =====================================================================

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
export PGHOST="${PGHOST:-127.0.0.1}"
export PGPORT="${PGPORT:-5432}"
export PGUSER="${PGUSER:-postgres}"
export PGPASSWORD="${PGPASSWORD:-dev_only_local}"
export PGDATABASE="${PGDATABASE:-postgres}"

# 8 逻辑库对应数据库名 (per 数据库设计书 v2.0 §1)
DATABASES=(auth_db user_db project_db task_db file_db notification_db report_db audit_db)

# 0) 确保 8 数据库存在 (每个逻辑库一个)
for db in "${DATABASES[@]}"; do
    echo "==> ensure database ${db}"
    psql -v ON_ERROR_STOP=1 -d postgres -tc "SELECT 1 FROM pg_database WHERE datname='${db}'" \
        | grep -q 1 \
        || psql -v ON_ERROR_STOP=1 -d postgres -c "CREATE DATABASE ${db}"
done

# 1) 跑每 service crate 的 migrations
declare -A SERVICE_DB=(
    [auth-service]=auth_db
    [user-service]=user_db
    [project-service]=project_db
    [task-service]=task_db
    [file-service]=file_db
    [notification-service]=notification_db
    [report-service]=report_db
    [audit-service]=audit_db
)

for svc in "${!SERVICE_DB[@]}"; do
    db="${SERVICE_DB[$svc]}"
    echo ""
    echo "==> migrate ${svc} -> ${db}"
    mig_dir="${REPO_ROOT}/crates/${svc}/migrations"
    if [[ ! -d "${mig_dir}" ]]; then
        echo "    (no migrations dir, skip)"
        continue
    fi
    # sqlx-migrate 排序 .sql 文件名
    while IFS= read -r sql; do
        echo "    apply: ${sql#"${REPO_ROOT}/"}"
        psql -v ON_ERROR_STOP=1 -d "${db}" -f "${sql}"
    done < <(find "${mig_dir}" -maxdepth 1 -name "*.sql" -type f | sort)
done

echo ""
echo "==> done: 8 logical databases initialized"
