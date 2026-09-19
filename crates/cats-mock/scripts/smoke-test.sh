#!/usr/bin/env bash
# smoke-test.sh - 端到端 smoke
# 用法: bash crates/cats-mock/scripts/smoke-test.sh
# 前置: docker + k3d + cargo + kubectl (per MVP_RUNBOOK.md)

set -euo pipefail

echo "==> [1/7] postgres ready?"
docker exec cats-postgres pg_isready -U postgres >/dev/null
echo "    OK"

echo "==> [2/7] kafka ready?"
docker exec cats-kafka /opt/kafka/bin/kafka-topics.sh --list \
  --bootstrap-server localhost:9092 | grep cats >/dev/null
echo "    OK"

echo "==> [3/7] auth-service healthz"
curl -sf http://localhost:8081/healthz | grep -q '"ok"'
echo "    OK"

echo "==> [4/7] user-service healthz"
curl -sf http://localhost:8082/healthz | grep -q '"ok"'
echo "    OK"

echo "==> [5/7] project-service healthz"
curl -sf http://localhost:8083/healthz | grep -q '"ok"'
echo "    OK"

echo "==> [6/7] cats-bff translate lookup"
TOKEN=$(curl -s -X POST http://localhost:8081/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"demo@cats.local","password":"demo123"}' | jq -r '.access_token')
[ -n "$TOKEN" ] && [ "$TOKEN" != "null" ]
curl -sf -X POST http://localhost:8080/api/v1/translate/lookup \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"source_text":"Hello","source_lang":"en-US","target_lang":"zh-CN"}' \
  | jq -e '.matches | length > 0' >/dev/null
echo "    OK"

echo "==> [7/7] ai-gateway chat"
curl -sf -X POST http://localhost:8090/v1/llm/chat \
  -H 'Content-Type: application/json' \
  -d '{"messages":[{"role":"user","content":"hi"}],"model":"openai"}' \
  | jq -e '.target_text | length > 0' >/dev/null
echo "    OK"

echo "==> ALL SMOKE PASS"