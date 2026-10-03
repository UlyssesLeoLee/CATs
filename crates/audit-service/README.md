# audit-service

> CATs 审计服务 (per ULYS-153 切片 C-2)

| 项目 | 内容 |
|---|---|
| Crate 名 | `audit-service` |
| 阶段 | MVP |
| 默认端口 | 8088（由 env `BIND_ADDR` 覆盖） |
| 数据边界 | `audit_db` |
| 镜像 | `harbor.cats.internal/cats/audit-service:0.1.0` |

## 概述

关键操作审计日志（登录、术语变更、任务导出等）+ 真实 Kafka consumer（替代原 30s heartbeat stub）。

引用：[CATs_微服务架构设计书_v1.0 §4.1](../../doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md)（16 服务清单）
引用：[CATs_Kafka物理发布设计_v1.0](../../doc/02-基础设计/部署设计/CATs_Kafka物理发布设计_v1.0.md)（Kafka REST proxy 部署模式）

## API 端点

| Method | Path | 说明 |
|---|---|---|
| GET | `/healthz` | 存活/就绪探针，返回 `{status, app:{name,version}}` |

## Kafka Consumer (per ULYS-153 切片 C-2)

`run_consumer_loop` 启动时 spawn，长轮询订阅 `cats.audit.v1` topic：

| 环境变量 | 默认 | 说明 |
|---|---|---|
| `KAFKA_REST_URL` | (未设) | Kafka REST proxy base URL，**未设则退化为 30s 心跳 no-op** |
| `KAFKA_CONSUMER_GROUP` | `audit-service` | Consumer group ID |
| `KAFKA_AUDIT_TOPIC` | `cats.audit.v1` | 订阅的 topic 名 |
| `DATABASE_URL` | (未设) | audit_db 连接串；**未设则 consumer 任务不启动，HTTP 仍可用** |

### 为什么用 REST proxy 而非直接 rdkafka

- `rdkafka` 0.36 需要 cmake-build + librdkafka 系统库，与 rustc 1.98 metadata bug 叠加编译失败（per BACKEND_STATUS_v0.1 §2）
- OI-3 已记录 rdkafka 0.36 移 K3s 阶段二 (Sprint 3+)
- 当前 MVP 通过 Kafka REST proxy (Confluent / 自建) HTTP 长轮询完成真实订阅
- `process_event` 函数签名不变，K3s 阶段二升级为 feature flag 后只需替换 `run_rest_poll_loop` 内部实现

### 失败重试策略

- 网络/HTTP 错误 → warn + sleep 5s 重连，不退出循环
- 单条消息处理失败 → error + skip（依赖 `ON CONFLICT (event_id) DO UPDATE` 幂等保护）

## 数据边界

- **Schema / 逻辑库**：`audit_db`
- **不读写他人的数据库**（per 架构书 §1.2 原则 4）

## 本地运行

```powershell
# 编译
cargo build -p audit-service

# 运行 (无 Kafka broker, 30s heartbeat stub)
cargo run -p audit-service

# 运行 (真实 REST proxy consumer)
$env:BIND_ADDR = "0.0.0.0:8088"
$env:DATABASE_URL = "postgres://svc_audit:...@pg-audit:5432/audit_db"
$env:KAFKA_REST_URL = "http://kafka-rest:8082"
$env:KAFKA_CONSUMER_GROUP = "audit-service-local"
cargo run -p audit-service

# 健康检查
curl http://127.0.0.1:8088/healthz
```

## 测试

```powershell
cargo test -p audit-service
```

5/5 单元测试通过（验证 schema 解析、JSON 解码、空响应处理、invalid JSON 拒绝路径）。

## 容器化

```bash
docker build -f deploy/docker/Dockerfile.rust --build-arg CRATE_NAME=audit-service -t audit-service:0.1.0 .
```

## Helm 部署

```bash
helm lint deploy/helm/audit-service
helm template deploy/helm/audit-service
```

## 引用基线文档

- [CATs_微服务架构设计书_v1.0 §4.1](../../doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md)
- [CATs_技术基线_v1.0 §1](../../doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md)
- [CATs_Rust技术选型书_v1.0](../../doc/02-基础设计/技术选型/CATs_Rust技术选型书_v1.0.md)
- [CATs_Kafka物理发布设计_v1.0](../../doc/02-基础设计/部署设计/CATs_Kafka物理发布设计_v1.0.md)
- [ULYS-153 切片 C-2 实施记录](../slices/ULYS-153_slice_C_report_audit.md)
