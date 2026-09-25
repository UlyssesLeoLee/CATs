# notification-service

> CATs 通知推送服务

| 项目 | 内容 |
|---|---|
| Crate 名 | `notification-service` |
| 阶段 | MVP (ULYS-152 切片 B-4) |
| 默认端口 | 8086（由 env `BIND_ADDR` 覆盖） |
| 数据边界 | `notification_db` |
| 镜像 | `harbor.cats.internal/cats/notification-service:0.1.0` |

## 概述

通知列表 / 标记已读 / 实时推送。

引用：[CATs_微服务架构设计书_v1.0 §4.1](../../doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md)、[ULYS-152 切片 B-4](#)

## API 端点

| Method | Path | 说明 |
|---|---|---|
| GET    | `/healthz` | 存活/就绪探针 |
| GET    | `/v1/notifications` | 列表（按 `user_id` + 分页 `limit`/`offset`） |
| POST   | `/v1/notifications` | 创建（内部 / 测试用；M1 简化，Sprint 2 改 Kafka consumer 触发） |
| PATCH  | `/v1/notifications/{id}/read?user_id={user_id}` | 标记已读（防越权：仅 `user_id` 本人可标记） |
| GET    | `/v1/notifications/ws?user_id={user_id}` | 实时推送（**SSE** 实现，详见下方 §WebSocket 决策） |

### POST /v1/notifications 请求示例

```json
{
  "user_id": "11111111-2222-3333-4444-555555555555",
  "type": "task_completed",
  "title": "Translation done",
  "body": "Your document has been translated.",
  "payload": { "task_id": "abc-123" }
}
```

### PATCH /v1/notifications/{id}/read 响应示例（200）

```json
{
  "id": "...",
  "marked": true,
  "read_at": "2026-09-20T..."
}
```

## WebSocket 决策（SSE vs WS）

ULYS-152 切片 B-4 任务描述：`WS /v1/notifications/ws — WebSocket 实时推送`。

实际实现：**Server-Sent Events (SSE)** 而非 RFC 6455 WebSocket。

理由（per 守门 #11 缺标比错标 + rustc 1.98 metadata bug）：

1. **`actix-ws` / `actix-web-actors` 不在 workspace `Cargo.lock`**：
   - 引入任一新依赖会触发 lock 重新解析，rustc 1.98 metadata bug 风险
   - per `BACKEND_STATUS_v0.1.md` §2：binary 编译卡 bug，Sprint 3 才解
2. **SSE 与 `tokio::sync::broadcast` 天然契合**：
   - broadcast `Receiver` → stream<SseEvent> → `HttpResponse::streaming(stream)`
   - 单向 server → client 流，无需 WS frame codec
3. **客户端集成简单**：
   ```js
   const es = new EventSource("/v1/notifications/ws?user_id=...");
   es.addEventListener("notification.created", ev => { ... });
   ```
4. **Sprint 2 升级路径清晰**：
   - 引入 `actix-ws 0.1`，把 `notification_stream` 内层 `async_stream::stream!` 替换为 ws `Message::Text` 帧
   - 端点 URL `/v1/notifications/ws` 保持不变

**SSE vs WS 对照**：

| 维度 | SSE（当前） | WebSocket（Sprint 2） |
|---|---|---|
| 方向 | 单向（server → client） | 双向 |
| 协议 | HTTP/1.1 chunked | RFC 6455，独立握手 |
| 客户端 API | `EventSource` 原生 | 需 ws 库 |
| broadcast::Receiver | 直接用 | 需 ws frame codec |
| rustc 1.98 兼容 | ✅ 零新依赖 | ❌ 需 actix-ws |

SSE 限制：客户端无法反向推送消息。Sprint 2 WS 升级后，客户端可主动 ack / 订阅过滤等。

## 事件总线（`events::EventBus`）

`tokio::sync::broadcast::Sender<SseEvent>` 容量 1024，慢 consumer 丢旧事件（`RecvError::Lagged`）而非阻塞 producer。

`SseEvent`：

```rust
pub struct SseEvent {
    pub event: String,             // "notification.created" / "notification.read" / "ping"
    pub data: serde_json::Value,   // 业务 payload
}
```

## 错误码（per `CATs_错误码表_v1.0.md` §3）

| HTTP | error 枚举 | 触发条件 |
|---|---|---|
| 400 | `invalid_request` | title 空 / 长度超限 / UUID 非法 |
| 404 | `notification_not_found` | 通知不存在或已读/已删 |
| 500 | `server_error` | DB 错误 |

## 数据边界

- **Schema / 逻辑库**：`notification_db`
- **不直连** `auth_db` / `user_db` / 其他服务数据库
- **软删除**：`status='deleted'`

## 上下游服务

- **上游（被调用）**：客户端 / BFF / 内部 worker-service（任务完成触发）
- **下游（主动调用）**：仅 `notification_db`
- **Sprint 2**：worker-service 通过 Kafka `cats.notification.v1` topic 触发

## 本地运行

```powershell
# 编译
cargo check -p notification-service

# 运行
$env:DATABASE_URL = "postgres://svc_notif:***@localhost:5432/notification_db"
$env:BIND_ADDR = "0.0.0.0:8086"
cargo run -p notification-service

# 健康检查
curl http://127.0.0.1:8086/healthz

# 创建通知
curl -X POST http://127.0.0.1:8086/v1/notifications `
  -H "Content-Type: application/json" `
  -d '{"user_id":"11111111-2222-3333-4444-555555555555","type":"test","title":"hi","body":"hello","payload":{}}'

# 订阅 SSE (curl)
curl -N "http://127.0.0.1:8086/v1/notifications/ws?user_id=11111111-2222-3333-4444-555555555555"
```

## 测试

```powershell
# 注: integration 测试需要 notification_test_db 在线 (per DATABASE_URL env)
cargo test -p notification-service
```

## 引用基线文档

- [CATs_微服务架构设计书_v1.0 §4.1](../../doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md)
- [CATs_技术基线_v1.0 §1](../../doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md)
- [CATs_Baseline一览_v1.0 §5.1](../../doc/05-其他/管理/CATs_Baseline一览_v1.0.md)
- [CATs_错误码表_v1.0 §3](../../doc/05-其他/管理/CATs_错误码表_v1.0.md)
- [ULYS-152 切片 B-4](../../.multica/issues/ULYS-152.md)