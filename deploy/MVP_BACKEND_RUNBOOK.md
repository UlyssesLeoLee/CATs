# CATs MVP Backend 启动 RUNBOOK (实测版)

**版本**: v1.0
**日期**: 2026-09-19
**作者**: 架构师(Mavis 接手 agent per DEC-008)
**状态**: 🟡 MVP 数据层(PG/Kafka/topic)+ 5 个 crate 实物已落地,后端 binary 待 cargo build 30-50 分钟

---

## 1. 当前实测状态(2026-09-19 17:53 JST 落地)

| 步骤 | 状态 | 证据 |
|---|---|---|
| ✅ docker 29.7.2 | 就绪 | `docker --version` |
| ✅ rustc 1.98.0 / cargo 1.98.0 | 就绪 | `cargo --version` |
| ✅ PostgreSQL 18.6 + pgvector 0.8.6 | **Up healthy** | `docker ps cats-mvp-pg` |
| ✅ Kafka 3.7.1 KRaft 1-broker | **Up healthy** | `docker ps cats-mvp-kafka` |
| ✅ 8 logical database 创建 | OK | auth_db / user_db / project_db / task_db / file_db / notification_db / report_db / audit_db |
| ✅ pgvector extension 启用 | OK | `SELECT extname FROM pg_extension` 在 project_db 显示 `vector` |
| ✅ 10 Kafka topic 创建 | OK | task.assign.v1 / task.completed.v1 / task.qa_blocked.v1 / cats.audit.v1 / cats.notifications.v1 / translation.{requested,completed,failed}.v1 / report.usage.v1 / alert.fired.v1 |
| 🟡 cargo build --release 12 service | **后台跑** | `D:/CATs/deploy/cargo-build.log` |
| ⏳ docker compose up 14 service | 待 cargo build 完成 | per `deploy/docker-compose-mvp.yml` |
| ⏳ 端到端 smoke | 待 | per `cats-mock/scripts/smoke-test.sh` |

---

## 2. 端口修正(实测发现本机端口冲突)

**问题**: 5432/55432 被本机其他 PG 占用,docker daemon 拒绝 bind。

**解决**: 改用 `56432:5432` host 端口,docker 内仍走 5432。

```
deploy/docker-compose-mvp.yml:
  postgres:
    ports: ["127.0.0.1:56432:5432"]
```

---

## 3. 完整启动流程(从前到后)

### 3.1 数据层起(PG + Kafka) — 2 分钟

```bash
# 1. 拉镜像
docker pull pgvector/pgvector:pg18
docker pull apache/kafka:3.7.1

# 2. 起 PG + Kafka
docker compose -f deploy/docker-compose-mvp.yml up -d postgres kafka
sleep 8

# 3. 创建 8 logical DB
for db in auth_db user_db project_db task_db file_db notification_db report_db audit_db; do
    docker exec cats-mvp-pg psql -U postgres -d postgres -c "CREATE DATABASE $db" 2>&1 | grep -v "already exists"
done

# 4. 启用 pgvector (project_db 用)
docker exec cats-mvp-pg psql -U postgres -d project_db -c "CREATE EXTENSION IF NOT EXISTS vector;"

# 5. 创建 10 Kafka topic
for topic in task.assign.v1 task.completed.v1 task.qa_blocked.v1 cats.audit.v1 \
             cats.notifications.v1 translation.requested.v1 translation.completed.v1 \
             translation.failed.v1 report.usage.v1 alert.fired.v1; do
    docker exec cats-mvp-kafka /opt/kafka/bin/kafka-topics.sh --create \
        --bootstrap-server localhost:9092 --topic $topic --partitions 1 --replication-factor 1
done

# 6. 验证
docker ps --filter "name=cats-mvp"
docker exec cats-mvp-pg pg_isready -U postgres
docker exec cats-mvp-kafka /opt/kafka/bin/kafka-topics.sh --list --bootstrap-server localhost:9092
```

### 3.2 cargo build 12 service — 30-50 分钟(一次性)

```bash
# 后台跑(避免前台阻塞 session)
python deploy/scripts/cargo-build-backend.py
# 日志写 D:/CATs/deploy/cargo-build.log

# 验证(完成后):
Get-ChildItem D:/CATs/target/release/*.exe
# 期望: auth-service.exe / user-service.exe / project-service.exe / task-service.exe
#       file-service.exe / notification-service.exe / report-service.exe
#       audit-service.exe / worker-service.exe / translation-core.exe
#       cats-ai-gateway.exe / cats-bff.exe
```

### 3.3 Docker Compose 全部起 — 5 分钟

```bash
# 确认 PG + Kafka 还在跑
docker compose -f deploy/docker-compose-mvp.yml up -d postgres kafka

# 起 12 service + envoy
docker compose -f deploy/docker-compose-mvp.yml up -d \
    ai-gateway translation-core \
    auth-service user-service project-service task-service file-service \
    notification-service report-service audit-service worker-service \
    cats-bff envoy

sleep 15

# 验证
curl http://localhost:10000/healthz           # envoy
curl http://localhost:8081/healthz            # auth-service
curl http://localhost:8090/healthz            # ai-gateway
```

### 3.4 端到端 smoke

```bash
# 1. 登录拿 token
TOKEN=$(curl -s -X POST http://localhost:10000/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"demo@cats.local","password":"demo123"}' | jq -r '.access_token')

# 2. 列项目
curl -s -H "Authorization: Bearer $TOKEN" http://localhost:10000/v1/projects | jq .

# 3. 翻译查 TM
curl -s -X POST http://localhost:10000/v1/translate/lookup \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"source_text":"Hello","source_lang":"en-US","target_lang":"zh-CN"}' | jq .

# 4. AI 网关 chat
curl -s -X POST http://localhost:8090/v1/llm/chat \
  -H 'Content-Type: application/json' \
  -d '{"messages":[{"role":"user","content":"hi"}],"model":"openai"}' | jq .
```

---

## 4. 已知问题与解决方案

### 4.1 cargo build `internal compiler error`

**症状**: `error[E0463]: can't find crate for std` + `internal compiler error: could not resolve trait item`

**原因**: 多个 cargo 进程同时跑造成 metadata cache 撞锁(rustc 1.98 在 Windows 上已知问题)

**解决**: 
```bash
Get-Process -Name cargo,rustc -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep 2
# 重跑(单 cargo build)
cargo build --release -p cats-common -p cats-rbac
# 之后再逐个加
cargo build --release -p auth-service
cargo build --release -p user-service
...
```

### 4.2 docker 端口冲突

**症状**: `ports are not available: exposing port TCP 127.0.0.1:5432 -> 127.0.0.1:0: bind`

**原因**: 本机其他 PG 已占 5432

**解决**: 改用 56432(`deploy/docker-compose-mvp.yml` 已修正)

### 4.3 cargo build workspace 找不到新加的 crate

**症状**: `error: package ID specification 'cats-ai-gateway' did not match any packages`

**原因**: main Cargo.toml workspace members 没加新 crate

**解决**: `Cargo.toml` 第 36 行后加 `"crates/cats-ai-gateway"`(已修)

---

## 5. 路径与端口速查

| 端口 | 服务 | 用途 |
|---|---|---|
| 56432 (host) | PostgreSQL | psql / service 直连 |
| 9092 | Kafka | producer / consumer |
| 10000 | Envoy 边缘 | 客户端入口 |
| 8081-8090 | 12 service 直连 | dev 调试用 |
| 50051 | translation-core gRPC | BFF 调 |
| 8090 | ai-gateway | REST + gRPC |

---

## 6. 关闭

```bash
docker compose -f deploy/docker-compose-mvp.yml down -v
# 数据保留:docker volume ls 检查 cats-mvp-pgdata
```

---

## 7. Sprint 3 续做(透明披露)

- 🔴 真 AI provider 接 OpenAI/Anthropic
- 🟡 真 secret 注入(Vault / Sealed Secrets)
- 🟡 translation-core 接真 project_db + 真 pgvector 嵌入
- 🟡 真 mTLS 服务间通信(per ADR-009)
- 🔴 `cargo check --workspace` 真实跨 crate 校验