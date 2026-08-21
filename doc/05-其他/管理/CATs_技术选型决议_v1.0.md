# CATs 技术选型决议 v1.0

> **文档编号**：CATs-PMO-008  
> **フェーズ**：管理 フェーズ 决策类  
> **关联任务**：QA-013 / QA-022 / QA-024 / QA-026 / QA-027 / QA-064  
> **版本**：v1.0  
> **创建日**：2026-08-20  
> **作者**：PMO + 架构师 + Rust Lead + DBA  
> **状态**：评审会前草稿

---

## 文档管理信息

### 审批栏

| 角色 | 审批 | 日期 |
|------|------|------|
| PM | ☐ | — |
| 架构师 | ☐ | — |
| Rust Lead | ☐ | — |
| DBA | ☐ | — |
| Sponsor | ☐ | — |

### 修订履历

| 版本 | 日期 | 修订人 | 修订内容 |
|------|------|--------|----------|
| **v1.0** | **2026-08-20** | **PMO** | **6 项技术选型决议（用户调整 3 项）** |

---

## 1. 目的

一次性决议 6 项 P1 关键技术选型，使 M1-S0 起的编码工作具备明确基线，作为：

- Cargo workspace 依赖基线
- CI 镜像 / Runner 版本基线
- K8s Operator 选型基线
- 文档版本对齐（Rust 选型书 / 数据库设计书 / 架构书 / 命名变更说明）

---

## 2. 决议总览

| # | QA | 主题 | **决议** | 决策方 | 优先级 | 状态 |
|---|----|------|----------|--------|--------|------|
| 1 | QA-013 | 阶段编号 | **A. 三阶段（M1/M2/M3）为准，需求 §10 M1-M5 作为子里程碑** | 架构 + 产品 | P1 | ✅ Decided |
| 2 | QA-022 | Web 框架 | **B. actix-web**（用户指定） | 架构 | P1 | ✅ Decided |
| 3 | QA-024 | MSRV | **C. Rust 最新 stable**（用户指定，跟随最新） | Rust Lead | P1 | ✅ Decided |
| 4 | QA-026 | PostgreSQL | **B. PostgreSQL 最新**（用户指定） | DBA | P2 | ✅ Decided |
| 5 | QA-027 | PG Operator | **A. CloudNativePG (CNPG)** | DBA + 架构 | P1 | ✅ Decided |
| 6 | QA-064 | Docker 基础镜像 | **A. distroless** | Rust Lead | P1 | ✅ Decided |

---

## 3. 各项详细

### 3.1 QA-013 阶段编号

**当前状况**：架构 §18 三阶段 vs 需求 §10 M1-M5 两套阶段划分。

**决议**：**以架构 §18 三阶段（M1/M2/M3）为准，需求 §10 的 M1-M5 作为子里程碑**。

**理由**：
- 架构 §18 是宏观（容量/性能阶段），更稳定
- 需求 §10 M1-M5 是功能 milestone，应作为子里程碑
- PMO 文档（系统化計画書 / WBS / 150 任务）已统一用 M1-M3
- 避免后续文档编号混乱

**同步文档**：
- `CATs_系统化计划书_v1.0.md` §6 里程碑（已用 M1-M3）
- `CATs_WBS_v1.0.xlsx` 16 フェーズ（已用 M1-M3）
- `CATs_微服务架构设计书_v1.0.md` §18 加注说明
- `CATs_需求规格说明书_v2.0.md` §10 M1-M5 作为子里程碑映射到 M1-M3

---

### 3.2 QA-022 Web 框架 → actix-web

**当前状况**：默认 axum。

**决议**：**actix-web**（用户指定）。

**理由**：
- actix-web 性能业界顶级（TechEmpower 基准领先）
- 自有 actor model runtime（`actix-rt`），不强制 Tokio
- 生态成熟，4.0+ 稳定
- 团队对 actix-web 性能信任
- 代价：与 Tokio 生态集成需要 `tokio::task::spawn_blocking` 等桥接（QA-021 决策的 Tokio 1.x 不变，仅跨框架集成时用桥接）

**影响**：
- 所有 15 服务 + BFF 使用 actix-web
- cats-grpc 库：actix-web + tonic 共存（用 `actix_web::web::block` 桥接）
- 不再用 axum 0.7+（撤销 QA-021 决议中的"Web 框架统一 Axum 0.7+"部分）
- CI 模板更新为 actix-web

**同步文档**：
- `CATs_Rust技术选型书_v1.0.md` §3 ADR-R-06：axum → actix-web
- `CATs_微服务架构设计书_v1.0.md` §3 软件方式 / §6 服务描述
- `CATs_类图_v1.0.md` §5.3 BFF Server（actix-web）
- `CATs_開発者ガイド_v1.0.md` §3 上手（actix-web 例子）

---

### 3.3 QA-024 MSRV → Rust 最新 stable

**当前状况**：当前 1.75+ 假设。

**决议**：**Rust 最新 stable**（跟随官方发布，跟随项目开发期升级）。

**理由**：
- 不锁定 MSRV，灵活性最大
- Rust 团队每 6 周 release，新特性可用
- 工具链随 rustup 升级无障碍
- 风险：新版本可能引入 breaking change，但 Cargo.lock + CI 测试可兜底

**具体策略**：
- CI Runner 始终用当时最新 stable
- 项目不写 `rust-version` 字段（或写 `rust-version = "1.79"` 作为最低支持线）
- Cargo.lock 提交并锁定 transitive deps
- 季度评估一次：是否锁定到某 stable（避免 breaking 风险）

**影响**：
- 所有 crate 用最新稳定版
- 团队开发机跟随 rustup 升级
- CI self-hosted runner 用 rust-toolchain.toml 固定

**同步文档**：
- `CATs_Rust技术选型书_v1.0.md` §2 / §3 ADR-R-01：版本策略
- `CATs_開発者ガイド_v1.0.md` §3 必装工具
- `CATs_CI_CD_構築運用手順書_v1.0.md` §4.2 基础工具

---

### 3.4 QA-026 PostgreSQL → 最新

**当前状况**：当前 16.x。

**决议**：**PostgreSQL 最新 stable**（跟随官方发布）。

**理由**：
- 跟随最新可用最新特性（pgvector 0.7+、逻辑复制增强等）
- 但要求：**M1-S0 时确认 pgvector 对所选 PG 版本兼容性**
- 上线期（2027-Q3）会再做一次评估，可能锁定到 LTS
- 风险：新版本生态未完全 ready → 缓解：QA-041 Benchmark 强制验证

**具体策略**：
- M1-S0 用当时最新 stable（如 17.x）
- 所有 migrations / 测试用此版本
- CNPG operator 必须支持
- pgvector 扩展必须能装

**影响**：
- 数据库设计书 §1 / §2：PG 版本描述
- CI 用 `postgres:最新` 镜像
- K3s 上 CNPG 配置 PG 最新版本
- 备份策略适配新版本特性

**同步文档**：
- `CATs_数据库设计书_v2.0.md` §1 / §2 PG 版本
- `CATs_Rust技术选型书_v1.0.md` §2 技术选型
- `CATs_可热插拔部署与运维设计_v1.0.md` §5 PG 配置

---

### 3.5 QA-027 PG Operator: CloudNativePG (CNPG)

**当前状况**：默认 CNPG。

**决议**：**CloudNativePG (CNPG)**（保持推荐）。

**理由**：
- K3s 官方推荐 PG Operator
- 原生 Kubernetes Operator 模式
- Patroni 需 etcd，复杂度高
- EDB 商业版成本不必要
- 集成 pgvector 良好

**影响**：
- K3s 上部署 CNPG operator
- 备份用 barman-cloud（CNPG 内置）
- 高可用用 streaming replication
- 监控用 postgres_exporter

**同步文档**：
- `CATs_微服务架构设计书_v1.0.md` §5.6 PG
- `CATs_可热插拔部署与运维设计_v1.0.md` §5
- `CATs_数据库设计书_v2.0.md` §1 部署

---

### 3.6 QA-064 基础镜像: distroless

**当前状况**：默认 distroless。

**决议**：**distroless**（保持推荐）。

**理由**：
- 最小攻击面（无 shell、无包管理器）
- 符合等保 2.0 三级（QA-071）
- 镜像体积小（~20MB）
- 缺点：调试困难 → 缓解：`kubectl debug` + ephemeral containers

**影响**：
- 所有 15 服务 + BFF 用 distroless/cc-debian12
- 前端 Node.js 服务用 distroless/nodejs20-debian12
- LLM 推理用 debian:12-slim（需要 CUDA / 额外库）
- 调试时切到 distroless:debug

**同步文档**：
- `CATs_Rust技术选型书_v1.0.md` §14.2 基础镜像
- `CATs_CI_CD_構築運用手順書_v1.0.md` §5 容器构建
- `CATs_开发者ガイド_v1.0.md` §3 调试技巧

---

## 4. 影响范围

### 4.1 直接受影响文档（本次同步）

| 文档 | 同步项 |
|------|--------|
| `CATs_实施前QA登记册_v1.0.md` | QA-013/022/024/026/027/064 状态 → Decided |
| `CATs_Rust技术选型书_v1.0.md` | actix-web + 最新 Rust |
| `CATs_数据库设计书_v2.0.md` | 最新 PostgreSQL |
| `CATs_微服务架构设计书_v1.0.md` | actix-web 框架 + CNPG |
| `CATs_类图_v1.0.md` | actix-web BFF |
| `CATs_開発者ガイド_v1.0.md` | actix-web 上手 + 最新 Rust |
| `CATs_CI_CD_構築運用手順書_v1.0.md` | 镜像 + Rust toolchain |
| `CATs_可热插拔部署与运维设计_v1.0.md` | PG 部署 + 镜像 |
| `CATs_命名变更说明.md` | 加注 actix-web 替换 axum |

### 4.2 间接影响（待 P1/P2 阶段处理）

- Cargo workspace 依赖基线（QA-024 actix-web）
- CI Runner 镜像版本（QA-024）
- K8s manifests（QA-027 CNPG）
- 容器镜像 Dockerfile（QA-064 distroless）

---

## 5. 与 150 任务 / 文档关联

| 任务 / 文档 | 关联 |
|-------------|------|
| 131 プロジェクト計画 | 时间基线（M1-M3） |
| 132 WBS | 阶段结构（已 M1-M3） |
| 144 ベースライン管理 | 本次 B0.1 增量基线 |
| QA-021 异步运行时 | Tokio 1.x 不变，actix-web 用桥接 |
| QA-071 等保 | distroless 满足 |

---

## 6. 关联文档

| 文档 | 路径 |
|------|------|
| CATs_实施前 QA 登记册 v1.0 | `05-其他\` |
| CATs_Rust 技术选型书 v1.0 | `02-基础设计\技术选型\` |
| CATs_数据库设计书 v2.0 | `03-详细设计\数据库设计\` |
| CATs_微服务架构设计书 v1.0 | `02-基础设计\架构设计\` |
| CATs_Baseline 一览 v1.0 | `05-其他\管理\` |
| CATs_类图 v1.0 | `03-详细设计\类图\` |

---

## 7. 待办 / Open Items

| # | 项 | 责任 | 期望关闭 |
|---|----|------|----------|
| OI-1 | M1-S0 时验证 pgvector 对最新 PG 兼容性 | DBA | M1-S0 |
| OI-2 | 评估是否锁定 Rust MSRV（季度评估） | Rust Lead | 季度 |
| OI-3 | actix-web 桥接 Tokio 工具类 | Rust Lead | M1-S0 |
| OI-4 | CNPG 部署 + barman-cloud 配置 | DBA + SRE | M1-S0 |
| OI-5 | distroless 镜像 CI 集成 | SRE | M1-S0 |

---

**文档结束**
