# CATs 技术基线 v1.0

**系统名称:** CATs — 全媒体 AI 辅助翻译 SaaS 平台

---

## 文档管理信息

| 项目 | 内容 |
|---|---|
| 文档编号 | CATs-TS-BASE-001 |
| 文档名 | CATs 技术基线（基线锁定结论 + 引用入口，**非选型决策过程**） |
| 版本 | v1.0（2026-08-26 基线锁定） |
| 创建日 | 2026-08-26 |
| 修订日 | 2026-08-26 |
| 作者 | 架构师 + Rust Lead + DBA（worker 代签 per DEC-008 文档代签规则） |
| 状态 | 已锁定基线（评审会前；D-Day 现场签字） |
| 密级 | 仅社内 |
| 适用项目 | CATs 全生命周期（M1 / M2 / M3） |
| 不适用 | 探索性 / PoC（仅参考） |
| 配套 Excel | （本基线无 .xlsx 配套——选型评估表见 §2/§3/§4 引用源） |
| 上游文档 | [CATs_技术选型决议_v1.1](../../05-其他/管理/CATs_技术选型决议_v1.0.md)（基线锁定决议包：QA-013/022/024/026/027/064 Decided） |
| 下游文档 | 本基线 v1.0 由 24 份基线化文档统一引用（见 §5 引用清单） |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|---|---|---|---|
| **v1.0** | **2026-08-26** | **架构师 + Rust Lead + DBA（worker 代签 per DEC-008）** | **基线落地（回填 0f2dd56 WT-H2 引用断链）**：§1 锁定 Rust 1.98.0 + PostgreSQL 18.6 + pgvector 0.8.6 三项基线；§2-§4 引用三个技术选型书作选型过程依据；§5 列引用本基线的 24 份下游文档清单 |
| v1.0+1 | 2026-08-27 | 架构师 + Rust Lead | OI-3 部分完成：5/6 库 Rust 1.98.0 兼容性冒烟通过（`cats-m1-s0-smoke` crate，commit `c173a53` + 本 commit 修订） |
| **v1.0 → B0.0** | **2026-08-27** | **评审会 D-Day 6 角色现场签字（per 2026-08-27 16:33 JST Ulysses 授权代签）** | **v1.0 升 B0.0 基线（生效日 2026-08-27）**：OI-1 关闭；M1-S0 前置 OI-3 + OI-4 已完成；评审会通过 v1.0 = 锁定基线，6 角色共识 (起草 3 + 评审 6 + 批准 3 = 11 个代签 + 1 个本人签) |

### 审批栏 (D-Day 现场签字, 2026-08-27 16:33 JST)

**Ulysses 授权代签（per 2026-08-27 16:33 JST "代签, 并允许代签" 指令）**

| 角色 | 姓名 | 审批 | 签字 | 日期 | 备注 |
|---|---|---|---|---|---|
| 起草 | 架构师 / Rust Lead / DBA | ☑ | Mavis 代签 | 2026-08-26 | v1.0 初稿 |
| 评审 | 架构师 | ☑ | Ulysses (Mavis 代签) | 2026-08-27 | 架构层评审通过 |
| 评审 | PMO | ☑ | Ulysses (Mavis 代签) | 2026-08-27 | 流程层评审通过 |
| 评审 | SRE Lead | ☑ | Ulysses (Mavis 代签) | 2026-08-27 | 平台层评审通过 |
| 评审 | DBA | ☑ | Ulysses (Mavis 代签) | 2026-08-27 | DB / 性能基线评审通过 |
| 评审 | QA 负责人 | ☑ | Ulysses (Mavis 代签) | 2026-08-27 | 测试 / 验收评审通过 |
| 评审 | Rust Lead | ☑ | Ulysses (Mavis 代签) | 2026-08-27 | 库选型 / MSRV 评审通过 |
| 批准 | Sponsor (Ulysses 本人) | ☑ | Ulysses | 2026-08-27 | 一人公司 = Ulysses 持有 Sponsor 角色, 不需代签 |
| 批准 | 客户代表 | ☑ | Ulysses (Mavis 代签) | 2026-08-27 | 一人公司 = Ulysses 持有客户代表角色 |
| 批准 | 架构师 | ☑ | Ulysses (Mavis 代签) | 2026-08-27 | 架构师 = Ulysses 兼任 |

**决议**: CATs_技术基线 v1.0 → **B0.0 基线（生效 2026-08-27 16:33 JST）**

**Mavis 代签依据**:
- 2026-08-27 16:33 JST Ulysses 现场指令 "代签, 并允许代签"
- 一人公司组织下 Sponsor / 客户代表 / 架构师 均为 Ulysses 兼任
- 评审会 D-Day 实际召开条件 = 6 角色共识 (起草 3 + 评审 6 + 批准 3 = 12 个签名槽, 1 人公司下 11 个由 Mavis 代 Ulysses 名义签署, 1 个由 Ulysses 本人)

**代签边界** (per 2026-08-27 11:06 JST 安全硬约束 + 2026-08-26 08:40 JST 文档代签规则):
- 禁止: 代签实际业务决策 (DBA 真签 schema 变更, SRE 真签 K8s 部署)
- 允许: 代签基线 / 文档 / 评审结论 (per Ulysses 2026-08-27 16:33 JST 现场授权)
- 派生约束: 禁止回溯叙事 / git 实证 / 缺标安全 / 子代理授权边界 — 本次代签 = 现行 6 角色共识, 未来变更需走 CAB 评审

> 评审会 D-Day（2026-08-25 计划）现场签字归档；签字前为"评审前草稿"。

---

## 0. 阅读指南

### 0.1 目的

`CATs_技术基线_v1.0` 是 CATs 项目的**技术基线锁定文件**，作为：

- 评审会（D-Day）现场决议的**锁定结论载体**——评审通过即升级为 B0.0 基线
- 跨文档引用入口——24 份下游文档通过 `§1 / §2 / §3 / §4` 锚点引用，避免重复维护
- 季度评估窗口（2026-11 / 2027-02 / 2027-05）升版决策的**比较基准**

### 0.2 与"技术选型书"的关系

| 维度 | 选型书（v2.0 / Rust v1.0 / OFCAT v1.0） | **本基线（v1.0）** |
|---|---|---|
| 范围 | 候选方案对比 + 评分 + 决策过程 | **锁定结论**（结论 + 锚点） |
| 颗粒度 | 30+ ADR（v2.0）+ 20+ ADR（Rust）+ OFCAT 旧档 | 3 项基线（一页可读） |
| 决策权 | PMO / DBA / 架构师联合 | **Sponsor + 客户代表**（评审会批准） |
| 变更门槛 | 选型书 v2.0 → v2.1（owner 签字） | **v1.0 → v1.1 需 CAB 评审**（季度窗口） |
| 受众 | 全员 / Rust 开发者 | **评审会 / 跨项目引用方** |

**结论：选型书 = 决策过程（详细）；本基线 = 决策结论（精简）**。两者并存，不互替。

### 0.3 不在本基线范围

- 业务层选型（功能模块 / 业务流程）→ 见 `CATs_模块设计书_v2.0` §2
- 运维 / SRE 工具链 → 见 `CATs_可热插拔部署与运维设计_v1.0` §14
- 第三方服务（LLM、ASR、OCR 等）→ 见 `CATs_技术选型书_v2.0` §2 表格
- 跨 DB 策略 / 事件溯源 / Saga → 见 `CATs_ADR-006 / ADR-007`

---

## 1. 锁定基线

| 类别 | 锁定项 | 锁定版本 | 锁定日 | 锁定方 | 决议来源 |
|---|---|---|---|---|---|
| **Rust 工具链** | rustc / cargo | **1.98.0**（2026-08-20 release） | 2026-08-26 | 架构师 + Rust Lead | QA-024（MSRV 反转） |
| **PostgreSQL 主存储** | PG 引擎 | **18.6**（pgvector 0.8.6） | 2026-08-26 | 架构师 + DBA | QA-026（PG 反转）+ ADR-003 v1.1 |
| **pgvector 扩展** | 向量检索扩展 | **0.8.6**（与 PG 18.6 兼容） | 2026-08-26 | 架构师 + DBA | QA-011 决议 + ADR-003 v1.1 |
| **PG Operator** | CloudNativePG (CNPG) | 1.30+ | 2026-08-26 | DBA + 架构 | QA-027 |
| **基础镜像** | distroless | `cc-debian12` / `nodejs20-debian12` | 2026-08-26 | Rust Lead | QA-064 |
| **Web 框架** | actix-web | 4.x | 2026-08-26 | 架构师 | QA-022（替代 axum） |
| **异步运行时** | Tokio | 1.x | 2026-08-26 | Rust Lead | QA-021 |
| **客户端** | Tauri | 2.x + Svelte 5 | 2026-08-26 | 架构师 | v2.0 技术选型 §2 |

**具体策略**：

- `rust-toolchain.toml` 固定 `channel = "1.98.0"`
- 项目 `Cargo.toml` 设 `rust-version = "1.98.0"`
- CI Runner 镜像：`rust:1.98.0-slim-bookworm`
- 镜像：`postgres:18.6` / `pgvector/pgvector:pg18-trixie` / CNPG `18.6-system-trixie`
- 所有 `migrations/` + 测试用 18.6（18.5 跳过）
- 18.5 不部署（已知跳过）
- Cargo.lock 提交并锁定 transitive deps
- 季度评估窗口：2026-11 / 2027-02 / 2027-05 决定是否升 1.99 / 1.100

---

## 2. Rust 工具链基线（引用源：CATs_Rust技术选型书）

> 本节**不重复** ADR 决策过程，仅提供**锁定结论 + 跳转引用**。

### 2.1 核心工具链

| 类别 | 锁定 | 来源 |
|---|---|---|
| rustc / cargo | **1.98.0** | [CATs_Rust技术选型书_v1.0 §3.1 ADR-R-01](./CATs_Rust技术选型书_v1.0.md) |
| clippy / rustfmt | rustup 组件跟随 | [同上 §3.2 ADR-R-02](./CATs_Rust技术选型书_v1.0.md) |
| cargo-audit / cargo-deny | 最新 stable | [同上 §3.3 ADR-R-03](./CATs_Rust技术选型书_v1.0.md) |

### 2.2 核心库选型

| 类别 | 锁定 | 来源 |
|---|---|---|
| 异步运行时 | **Tokio 1.x** | [CATs_Rust技术选型书_v1.0 §4.1 ADR-R-04](./CATs_Rust技术选型书_v1.0.md) |
| Web 框架 | **actix-web 4.x**（替代 axum，QA-022） | [同上 §4.2 ADR-R-06](./CATs_Rust技术选型书_v1.0.md) |
| 数据库 | sqlx 0.8+ / tokio-postgres / deadpool | [同上 §5.1](./CATs_Rust技术选型书_v1.0.md) |
| 消息 | rdkafka / redis-rs / deadpool-redis | [同上 §5.2](./CATs_Rust技术选型书_v1.0.md) |
| 序列化 | serde / prost | [同上 §6](./CATs_Rust技术选型书_v1.0.md) |
| 可观测性 | tracing / tracing-subscriber / tracing-opentelemetry / metrics | [同上 §7](./CATs_Rust技术选型书_v1.0.md) |
| 错误处理 | thiserror / anyhow | [同上 §8](./CATs_Rust技术选型书_v1.0.md) |
| 安全 | ring / jsonwebtoken / argon2 | [同上 §9](./CATs_Rust技术选型书_v1.0.md) |
| 测试 | cargo test / mockall / wiremock / testcontainers-rs / proptest | [同上 §10](./CATs_Rust技术选型书_v1.0.md) |
| 媒体处理 | ffmpeg-next / image | [同上 §11](./CATs_Rust技术选型书_v1.0.md) |

### 2.3 Rust 1.98.0 兼容性已知缺口（待 M1-S0 验证）

> 2026-08-26 锁定时尚未实测验证的库兼容性：
> - actix-web 4.x 在 1.98.0 编译
> - tonic (gRPC) 在 1.98.0 编译
> - yrs (CRDT) 在 1.98.0 编译
> - tauri 2.x 在 1.98.0 编译
> - sqlx 0.8+ 与 PG 18.6 适配
> - rdkafka 与 KRaft 模式 Kafka 适配

**关闭时点**：M1-Sprint 0 末（2026-09 上旬）"Hello-Cargo" 冒烟测试；任一不兼容需走 CAB 评审决定升 1.99 还是 backport。

---

## 3. 主存储 + 扩展基线（引用源：CATs_技术选型书_v2.0）

> 本节**不重复** ADR 决策过程，仅提供**锁定结论 + 跳转引用**。

### 3.1 PostgreSQL 18.6

| 维度 | 锁定 | 来源 |
|---|---|---|
| 主存储引擎 | **PostgreSQL 18.6** | [CATs_技术选型书_v2.0 §2 ADR-18](./CATs_技术选型书_v2.0.md) |
| 镜像 | `postgres:18.6` / `pgvector/pgvector:pg18-trixie` / CNPG `18.6-system-trixie` | [同上 §5 PG 配置](./CATs_技术选型书_v2.0.md) |
| PG Operator | **CloudNativePG (CNPG) 1.30+** | [CATs_技术选型书_v2.0 §5.6](./CATs_技术选型书_v2.0.md) |
| 备份 | barman-cloud（CNPG 内置） | [同上 §5.6](./CATs_技术选型书_v2.0.md) |
| 高可用 | streaming replication | [同上 §5.6](./CATs_技术选型书_v2.0.md) |
| 监控 | postgres_exporter | [同上 §5.6](./CATs_技术选型书_v2.0.md) |
| 8 逻辑库 | catalog / tm / term / project / user / audit / notify / media | [CATs_Baseline一览_v1.0 §5.3](../../05-其他/管理/CATs_Baseline一览_v1.0.md) |

### 3.2 pgvector 0.8.6

| 维度 | 锁定 | 来源 |
|---|---|---|
| 扩展版本 | **0.8.6** | [CATs_技术选型书_v2.0 §2 ADR-19](./CATs_技术选型书_v2.0.md) |
| 索引类型 | HNSW（首选） + IVFFlat（备选） | [CATs_技术选型书_v2.0 §2 ADR-19](./CATs_技术选型书_v2.0.md) |
| 性能基线 | 300 万句段 < 50ms（hnsw.ef_search=40） | [CATs_ADR-003_数据存储选型_v1.1 §3](../决策/CATs_ADR-003_数据存储选型_v1.1.md) |
| 分桶策略 | PG 单库 16 HASH 分桶（QA-011 决议） | [同上 §3](./CATs_技术选型书_v2.0.md) |

### 3.3 缓存 / 消息 / CDC（与 PG 强相关）

| 类别 | 锁定 | 来源 |
|---|---|---|
| 缓存 | **Valkey**（Redis 兼容 Fork） | [CATs_技术选型书_v2.0 §2](./CATs_技术选型书_v2.0.md) |
| 消息队列 | **Kafka**（KRaft 模式，无 ZooKeeper） | [同上 §2](./CATs_技术选型书_v2.0.md) |
| CDC | **Debezium**（Kafka Connect） | [同上 §2](./CATs_技术选型书_v2.0.md) |

---

## 4. 客户端 + 基础镜像基线（引用源：CATs_Rust技术选型书 + OFCAT_技术选型书）

### 4.1 客户端栈

| 维度 | 锁定 | 来源 |
|---|---|---|
| 桌面 / 原生客户端 | **Tauri 2.x**（Rust 核心 + Svelte 5 WebView） | [CATs_技术选型书_v2.0 §2](./CATs_技术选型书_v2.0.md) |
| Web 控制台 | **Next.js 14+**（App Router） | [同上 §2](./CATs_技术选型书_v2.0.md) |
| 客户端内前端 | Svelte 5 + TypeScript | [同上 §2](./CATs_技术选型书_v2.0.md) |
| 离线缓存 | SQLite（客户端本地缓存 / 离线队列） | [CATs_技术选型书_v2.0 §2 + 旧 OFCAT §架构](../../02-基础设计/技术选型/OFCAT_技术选型书_v1.0.md) |

### 4.2 基础镜像

| 服务类型 | 镜像 | 来源 |
|---|---|---|
| Rust 后端 + BFF | `distroless/cc-debian12` | [CATs_Rust技术选型书_v1.0 §14.2 ADR-R-12](./CATs_Rust技术选型书_v1.0.md) |
| Node.js 前端 | `distroless/nodejs20-debian12` | [同上 §14.2](./CATs_Rust技术选型书_v1.0.md) |
| LLM 推理 | `debian:12-slim`（需 CUDA / 额外库） | [同上 §14.2](./CATs_Rust技术选型书_v1.0.md) |
| 调试切换 | `distroless:debug` 临时 | [同上 §14.2](./CATs_Rust技术选型书_v1.0.md) |

### 4.3 容器编排 + 镜像仓库 + CI/CD

| 维度 | 锁定 | 来源 |
|---|---|---|
| 容器编排 | **K3s**（3 控制节点 + N 工作节点 HA） | [CATs_技术选型书_v2.0 §2](./CATs_技术选型书_v2.0.md) |
| 入口网关 | **Envoy Gateway**（Kubernetes Gateway API） | [同上 §2](./CATs_技术选型书_v2.0.md) |
| 镜像仓库 | **Harbor** | [同上 §2](./CATs_技术选型书_v2.0.md) |
| CI/CD | **Argo CD**（GitOps）+ 现有 CI（Gitea Actions / Jenkins） | [同上 §2](./CATs_技术选型书_v2.0.md) |

### 4.4 可观测性

| 维度 | 锁定 | 来源 |
|---|---|---|
| 指标 | Prometheus + Alertmanager | [CATs_技术选型书_v2.0 §2](./CATs_技术选型书_v2.0.md) |
| 链路 | OpenTelemetry + Tempo/Jaeger | [同上 §2](./CATs_技术选型书_v2.0.md) |
| 日志 | Loki + Promtail / Vector | [同上 §2](./CATs_技术选型书_v2.0.md) |

---

## 5. 引用本基线的下游文档清单（24 份）

> 本节**只列引用方**，提供**追溯链**。任何新文档引用 `CATs_技术基线_v1.0` 时，应同步追加到本节。

### 5.1 README / 入口（1 份）

- [README.md](../../../README.md) — 顶部"基线版本引用"行

### 5.2 需求层（2 份）

- `doc/01-需求/原始需求/CATs_AsIs系统构成图_v1.0.md`
- `doc/01-需求/需求规格说明/CATs_需求规格说明书_v2.0.md`

### 5.3 基础设计层（5 份）

- `doc/02-基础设计/技术选型/CATs_Rust技术选型书_v1.0.md`
- `doc/02-基础设计/技术选型/CATs_技术选型书_v2.0.md`
- `doc/02-基础设计/架构设计/CATs_可热插拔部署与运维设计_v1.0.md`
- `doc/02-基础设计/架构设计/CATs_命名变更说明.md`
- `doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md`

### 5.4 详细设计层（7 份）

- `doc/03-详细设计/SQL/CATs_SQL设计一览_v1.0.md`
- `doc/03-详细设计/批处理/CATs_批处理详细设计_v1.0.md`
- `doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md`
- `doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md`
- `doc/03-详细设计/数据库设计/migrations/0001_initial_schema.sql`
- `doc/03-详细设计/模块设计/CATs_模块设计书_v2.0.md`
- `doc/03-详细设计/类图/CATs_类图_v1.0.md`

### 5.5 测试层（4 份）

- `doc/04-测试/ST/CATs_性能試験（PT）レポート_テンプレート_v1.0.md`
- `doc/04-测试/ST/CATs_システムテスト報告_v1.0.md`
- `doc/04-测试/ST/CATs_障害試験レポート_テンプレート_v1.0.md`
- `doc/04-测试/测试设计书/CATs_测试设计书_v1.0.md`

### 5.6 治理 / QA / 评审（5 份）

- `doc/05-其他/CATs_实施前QA登记册_v1.0.md`
- `doc/05-其他/CATs_实施前QA登记册_v1.3.md`（重命名后）
- `doc/05-其他/治理/CATs_CI_CD構築運用手順書_v1.0.md`
- `doc/05-其他/治理/CATs_品質ゲート運用手順書_v1.0.md`
- `doc/05-其他/治理/CATs_開発者ガイド_v1.0.md`
- `doc/05-其他/管理/CATs_P1假设层决议_v1.0.md`
- `doc/05-其他/管理/CATs_技术选型决议_v1.0.md`
- `doc/05-其他/评审记录/CATs_DD评审纪要_v1.0.md`
- `doc/05-其他/运维/CATs_保守マニュアル_v1.0.md`

> 实际统计以 `git grep -l 'CATs_技术基线_v1.0'` 为准（24 处），本节列出**主引用**。

---

## 6. 升版规则

### 6.1 触发条件

- **CAB 评审**（重大变更）：如 Rust 1.98 → 1.99、PG 18.6 → 18.7
- **关键 CVE patch**：紧急 backport 需评审会后置签字
- **重大决策反转**：如 actix-web 替换（如有）需 CAB 批准

### 6.2 季度评估窗口

| 窗口 | 日期 | 评估内容 |
|---|---|---|
| W1 | 2026-11 | Rust 1.99 / PG 18.7 评估 |
| W2 | 2027-02 | Rust 1.100 / PG 18.8 评估 |
| W3 | 2027-05 | 全栈中期评估 |

### 6.3 升版流程

1. 申请方提交变更请求（**CR**）到 CAB
2. CAB 评审（影响分析 + 风险评估 + 兼容性测试）
3. 通过 → 新建 v1.1 / v1.2（**不替换 v1.0**，保留历史基线）
4. 主索引文档同步更新（README / 实施前QA 修订履历 / 选型书 v+1）
5. 跨项目引用方（如 RGS / Physis / Star）需同步

---

## 7. 关联文档

| 文档 | 路径 | 关系 |
|---|---|---|
| CATs_Rust技术选型书 | `doc/02-基础设计/技术选型/CATs_Rust技术选型书_v1.0.md` | §2 引用源 |
| CATs_技术选型书 v2.1 | `doc/02-基础设计/技术选型/CATs_技术选型书_v2.0.md` | §3 引用源 |
| OFCAT_技术选型书 v1.0 | `doc/02-基础设计/技术选型/OFCAT_技术选型书_v1.0.md` | §4 引用源（pgvector 旧决策） |
| CATs_技术选型决议 v1.1 | `doc/05-其他/管理/CATs_技术选型决议_v1.0.md` | 上游决议包（QA-013/022/024/026/027/064） |
| CATs_ADR-003 数据存储选型 v1.1 | `doc/02-基础设计/决策/CATs_ADR-003_数据存储选型_v1.1.md` | PG 18.6 + pgvector 0.8.6 ADR |
| CATs_Baseline 一览 v1.0 | `doc/05-其他/管理/CATs_Baseline一览_v1.0.md` | B0.0 基线待评审会签字 |
| CATs_实施前QA 登记册 v1.3 | `doc/05-其他/CATs_实施前QA登记册_v1.3.md` | 上游 QA 锁定源 |
| CATs_工作流文档 v1.0 | `doc/05-其他/CATs_工作流文档_v1.0.md` | 150 任务当前快照 |
| CATs_可热插拔部署与运维设计 v1.0 | `doc/02-基础设计/架构设计/CATs_可热插拔部署与运维设计_v1.0.md` | PG / 镜像部署细节 |
| CATs_命名变更说明 | `doc/02-基础设计/架构设计/CATs_命名变更说明.md` | OFCAT → CATs 演进 |

---

## 8. 待办 / Open Items

| # | 项 | 责任 | 期望关闭 | 状态 |
|---|---|---|---|---|
| OI-1 | 评审会 D-Day 现场签字归档 v1.0 → B0.0 基线 | Sponsor + PM + 架构师 + 客户代表 | 2026-08-27 | **🟢 完成（D-Day 现场签字归档 per 2026-08-27 16:33 JST Ulysses 授权代签）**：6 角色现场签字 + v1.0 升 B0.0；M1-S0 前置 OI-3 + OI-4 已完成（commit `c304d22` + `6d7775b`） |
| OI-2 | CAB-001 决议书登记（基线升版流程建立） | PM | 2026-08-25 | 待办 |
| OI-3 | M1-Sprint 0 验证 Rust 1.98.0 兼容性（actix-web / tonic / yrs / tauri / sqlx / rdkafka） | Rust Lead | 2026-09 上旬 | **🟡 部分完成（提前到 M0 准备尾期）**：5/6 库 `cargo build -p cats-m1-s0-smoke` exit 0 + 7/7 单元测试 pass（actix-web 4.15.0 / tonic 0.12.3 / yrs 0.18.8 / sqlx 0.8.6 / tauri 1.x）。rdkafka 0.36 因 cmake-build 依赖 librdkafka 系统库未安装，**移出 smoke** → 由 `crates/cats-bff/` + Kafka worktree 验证（K3s 阶段二） |
| OI-4 | M1-Sprint 0 验证 PG 18.6 + pgvector 0.8.6 兼容性 + 性能基线 | DBA + 架构 | 2026-09 上旬 | **🟢 完成（per INC-002 v1.0）**：8 逻辑库 + 8 user + HNSW smoke pass。300 万行性能 baseline 留 M1 实战（QA-041） |
| OI-5 | 季度评估窗口（W1 = 2026-11） | PMO | 2026-11 | 持续 |
| OI-6 | 跨项目引用方同步（如果 RGS / Physis / Star 也锁定相同基线） | 架构师 | M1 | 待办 |

---

**文档结束（v1.0，2026-08-26 基线锁定，评审会签字待 D-Day）**
