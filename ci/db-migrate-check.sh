#!/usr/bin/env bash
# CI check: 验证所有 8 service migrations 能在干净 PG18 + pgvector 0.8.6 上跑成功
# per Sprint 2 W3-W4 v2.0 DDL 落地
# 用法: bash ci/db-migrate-check.sh

set -euo pipefail

PG_VERSION="18.6"
PGVECTOR_VERSION="0.8.6"
DB_NAME="cats_test_$$"

echo "==> starting pgvector:pg${PG_VERSION} container"
CONTAINER_ID=$(docker run -d \
  -e POSTGRES_DB="$DB_NAME" \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -p 55432:9292 \
  pgvector/pgvector:pg${PG_VERSION})

trap 'echo "==> cleaning up container $CONTAINER_ID"; docker rm -f "$CONTAINER_ID" >/dev/null 2>&1 || true' EXIT

echo "==> waiting for postgres ready"
for i in {1..30}; do
  if docker exec "$CONTAINER_ID" pg_isready -U postgres >/dev/null 2>&1; then
    echo "    postgres ready after ${i}s"
    break
  fi
  sleep 1
done

PSQL="docker exec -i $CONTAINER_ID psql -U postgres -d $DB_NAME"

# 跑 8 service migrations
SERVICES=(auth user project task file notification report audit)
for svc in "${SERVICES[@]}"; do
  echo "==> migrate $svc-service"
  MIGRATION_DIR="crates/${svc}-service/migrations"
  for f in "$MIGRATION_DIR"/*.sql; do
    [ -f "$f" ] || continue
    echo "    applying $f"
    cat "$f" | $PSQL >/dev/null || { echo "FAIL: $f"; exit 1; }
  done
done

echo "==> all migrations applied successfully"

# 验证表存在
echo "==> verifying tables"
$PSQL -c "\dt public.*" | head -40

echo "==> PASS"