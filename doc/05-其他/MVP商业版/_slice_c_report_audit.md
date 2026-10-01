# 切片 C: report-service + audit-service 真实 Kafka consumer

**Slice**: C — 辅助 2 服务
**目标 crate**: `crates/report-service/` + `crates/audit-service/`
**基线**: `agent/minimaxm3/d11b0bce579f`

## 切片 C-1: report-service

### 任务范围

报表聚合查询（基于 audit-service 表 + 各服务表 JOIN）：

- `GET /v1/reports/usage?from=&to=` — 项目使用量统计
- `GET /v1/reports/translation-volume?project_id=` — 翻译量
- `GET /v1/reports/audit-summary?workspace_id=` — 审计摘要

### 数据源

跨服务 SQL 视图：建议在 audit-service 表 + task-service 表 + translation-core tm_update 表上做聚合查询（直接 SQL 聚合，不要拉数据到内存）。

### 复用

- `crates/audit-service/src/db.rs` — 看 schema
- `crates/cats-rbac/src/lib.rs`

## 切片 C-2: audit-service 真实 Kafka consumer

### 任务范围

替换 audit-service 当前 30s heartbeat stub 为真实 Kafka consumer：

- 引入 `rdkafka` crate（已在 auth-service/audit.rs 有 rdkafka 引用模式）
- `run_consumer_loop` 真正订阅 `cats.audit.v1` topic
- 收到消息 → 调 `process_event` → 写 audit_log 表

### 现有代码

`crates/audit-service/src/consumer.rs` 已有 `process_event` 函数 + 30s heartbeat stub `run_consumer_loop`。需要替换后者。

### 复用

- `crates/auth-service/src/audit.rs` — rdkafka Producer 模式可参考
- `crates/cats-common/src/lib.rs` — Kafka config env

## 交付清单

1. `crates/report-service/` — 完整 3 endpoint 服务（同其他服务结构）
2. `crates/audit-service/src/consumer.rs` — 真实 rdkafka 实现
3. `crates/audit-service/Cargo.toml` — rdkafka 依赖（per §约束必须显式授权才能加新依赖 — 已有 Cargo.lock 引用，可直接复用）

## 验收标准

- report-service 3 endpoint 实现 + 跨表聚合 SQL
- audit-service consumer 真实订阅（即使 binary 编不出，cargo check 应过）
- cargo check -p report-service -p audit-service 通过

## 诚实披露

rustc 1.98 bug 仍在。cargo check 单 crate 应过。

## 作者署名

架构师(Mavis 接手 agent per DEC-008) <architecture@mavis.local>

## 引用

- 父: ULYS-125
- 审计: §4.3
- Kafka 设计: `doc/02-基础设计/部署设计/CATs_Kafka物理发布设计_v1.0.md`
- 错误码: `doc/05-其他/错误码/CATs_错误码表_v1.0.1.md`
