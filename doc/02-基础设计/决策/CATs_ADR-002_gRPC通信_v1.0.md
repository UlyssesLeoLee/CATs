# CATs ADR-002：服务间通信协议选型

> **文档编号**：CATs-ADR-002
> **版本**：v1.0
> **创建日**：2026-08-26
> **作者**：架构师
> **状态**：已接受
> **取代**：—

---

## 1. 背景

15 微服务 + 4 共享库，服务间通信是底座。要决策：

- 同步通信：REST vs gRPC
- 异步通信：消息队列（Kafka vs NATS vs Redis Streams）
- 浏览器入口：BFF（Backend-for-Frontend）协议

## 2. 选项

| 选项 | 描述 | 优势 | 劣势 |
|------|------|------|------|
| **A. REST + JSON** | 通用 HTTP/JSON | 简单、调试工具多 | 文本协议体积大、强类型缺失、性能一般 |
| **B. gRPC + Protocol Buffers** ✅ 服务间 | 二进制、强类型、双向流 | 高性能、强契约、流式支持 | 浏览器原生不友好、调试门槛 |
| **C. GraphQL** | 客户端驱动查询 | 灵活查询 | 服务端复杂度高、N+1 风险 |
| **D. Kafka** ✅ 异步事件 | 事件溯源、Outbox、解耦 | 高吞吐、持久化、生态成熟 | 运维重、消息顺序需设计 |

服务间 = **gRPC（proto3）**；客户端入口 = **BFF 转 REST/JSON**（浏览器友好）；异步事件 = **Kafka**。

## 3. 决策

| 通信场景 | 协议 | 备注 |
|----------|------|------|
| 服务间同步 | **gRPC（proto3）** | 单一 .proto 仓库，buf 工具链 |
| 服务间异步 | **Kafka**（v3.x，KRaft 模式） | Outbox 模式 + Debezium CDC |
| 浏览器 ↔ BFF | REST/JSON + WebSocket（协同） | BFF 聚合 + 协议转换 |
| BFF ↔ 内部服务 | gRPC | 复用 service 契约 |
| 协同 WebSocket | Yjs over WebSocket | 见 collab-ws 服务设计 |

## 4. 影响

- **正面**：服务间契约强、性能可预测、事件流可回放
- **负面**：
  - proto 仓库需独立治理（buf.yaml + CI 校验）
  - Kafka 集群运维成本 → 已有 OLU 预算
  - BFF 需聚合层，引入额外一跳
- **风险**：
  - gRPC 调试需 grpcurl/grpcui → 已在 53 任务规划
  - 消息幂等性必须由业务保证 → 已在 L-7 Outbox 模式识别

## 5. 关联

- 上游：ADR-001
- 下游：API 设计书 v2.0、可观测性平台设计 v1.0
- 阻塞项：—
