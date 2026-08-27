# notification-service

> CATs 通知推送服务

| 项目 | 内容 |
|---|---|
| Crate 名 | `notification-service` |
| 阶段 | MVP |
| 默认端口 | 8086（由 env `BIND_ADDR` 覆盖） |
| 数据边界 | `notification_db` |
| 镜像 | `harbor.cats.internal/cats/notification-service:0.1.0` |

## 概述

站内信/WebSocket 推送/邮件通知，任务完成/失败提醒

引用：[CATs_微服务架构设计书_v1.0 §4.1](../../doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md)（16 服务清单）

## API 端点（M0 占位）

| Method | Path | 说明 |
|---|---|---|
| GET | `/healthz` | 存活/就绪探针，返回 `{status, app:{name,version}}` |

M1 阶段补充：参见 [api/openapi/cats-openapi-v1.yaml](../../api/openapi/cats-openapi-v1.yaml)

## 数据边界

- **Schema / 逻辑库**：`notification_db`
- **不读写他人的数据库**（per 架构书 §1.2 原则 4）
- **不持有业务真相**于 Valkey/Kafka（per §1.2 原则 2）

## 上下游服务

- **上游（被调用）**：客户端 / BFF
- **下游（主动调用）**：`cats-proto`（gRPC 契约）+ 数据库（PostgreSQL 18.6，per [技术基线 §1](../../doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md)）

## 跨服务 gRPC 契约

本服务 M0 阶段无独立 gRPC 接口（M1 阶段视情况引入）。

## 本地运行

```powershell
# 编译
cargo build -p notification-service

# 运行
$env:BIND_ADDR = "0.0.0.0:8086"
cargo run -p notification-service

# 健康检查
curl http://127.0.0.1:8086/healthz
```

## 测试

```powershell
cargo test -p notification-service
```

## 容器化

```bash
docker build -f deploy/docker/Dockerfile.rust --build-arg CRATE_NAME=notification-service -t notification-service:0.1.0 .
```

## Helm 部署

```bash
helm lint deploy/helm/notification-service
helm template deploy/helm/notification-service
```

## 引用基线文档

- [CATs_微服务架构设计书_v1.0 §4.1](../../doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md)
- [CATs_技术基线_v1.0 §1](../../doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md)
- [CATs_Rust技术选型书_v1.0](../../doc/02-基础设计/技术选型/CATs_Rust技术选型书_v1.0.md)
