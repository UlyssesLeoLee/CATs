# cats-common

> CATs 共享库 — 跨服务共享的类型、错误、日志、可观测性工具集

| 项目 | 内容 |
|---|---|
| Crate 名 | `cats-common` |
| 版本 | `0.1.0`（随 workspace 升级） |
| 阶段 | M0 脚手架 → M1 落地 |
| 依赖基线 | 引用 [CATs_技术基线_v1.0](../../doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md) §1 |

## 概述

`cats-common` 是 **CATs 18 个 crate 共享的基座 crate**。它承担三类职责：

1. **共享类型**：`AppMeta`、`ErrorKind`、`RequestId` 等跨服务通用结构
2. **错误体系**：`anyhow` + `thiserror` 组合（per Rust 选型书 §4.3）
3. **可观测性入口**：`init_tracing()` 统一初始化 tracing subscriber（per §9.1/§9.2）

## 公开 API（M0 阶段）

| 符号 | 用途 |
|---|---|
| `version() -> &'static str` | 返回 crate 版本（用于 `/healthz`） |
| `name() -> &'static str` | 返回 crate 名 |
| `AppMeta` | 名称 + 版本结构，可直接 `serde::Serialize` |
| `init_tracing()` | tracing subscriber 初始化（占位，M1 替换为完整实现） |

## 数据边界

- **无独立数据库**
- **无独立网络端点**
- **不依赖任何业务 service**，仅依赖 workspace 共享 deps

## 上下游服务

- **上游（被依赖）**：所有 16 个 service + `cats-bff` + `cats-proto` 间接引用
- **下游（依赖）**：无（叶节点）

## 路径

- 源文件：[`crates/common/src/lib.rs`](src/lib.rs)
- 文档：仓库根 `README.md` §代码架构与目录约定

## 验证

```powershell
# 仅本 crate
cargo test -p cats-common

# 整 workspace
cargo test --workspace --all-features
```

## 后续路线（M1+）

- M1-S0：完整 `init_tracing()` 实现（OpenTelemetry OTLP exporter + JSON 格式）
- M1-S1：`AppError` + `ErrorKind` 枚举（thiserror 派生）
- M1-S2：`RequestId` UUID v7 包装（per §9.2 trace context 透传）
- M1-S3：`Config` from-env 加载器（per §11）

## 引用基线文档

- [CATs_微服务架构设计书_v1.0 §4.1](../../doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md)（18 服务布局）
- [CATs_技术基线_v1.0 §1](../../doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md)（Rust 1.98.0 + PG 18.6）
- [CATs_Rust技术选型书_v1.0 §4.3/§9/§11](../../doc/02-基础设计/技术选型/CATs_Rust技术选型书_v1.0.md)
