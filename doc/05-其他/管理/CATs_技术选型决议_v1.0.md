# CATs 技术选型决议 v1.1

> **文档编号**：CATs-PMO-008  
> **フェーズ**：管理 フェーズ 决策类  
> **关联任务**：QA-013 / QA-022 / QA-024 / QA-026 / QA-027 / QA-064  
> **版本**：v1.1(基线升级)  
> **创建日**：2026-08-20  
> **修订日**：2026-08-26  
> **作者**：架构师 + Rust Lead + DBA(worker 代签 per DEC-008)  
> **状态**：已锁定基线  
> **基线引用**：[CATs_技术基线_v1.0 §1](../02-基础设计/技术选型/CATs_技术基线_v1.0.md)

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
| v1.0 | 2026-08-20 | PMO | 6 项技术选型决议(用户调整 3 项) |
| **v1.1** | **2026-08-26** | **架构师 + Rust Lead + DBA(worker 代签 per DEC-008)** | **基线升级(WT-H2)**:QA-024 由"跟随 Rust 最新 stable" → **锁定 Rust 1.98.0**;QA-026 由"跟随 PostgreSQL 最新" → **锁定 PostgreSQL 18.6**;引用 `CATs_技术基线_v1.0 §1`;原 2026-08-20 决议作为历史保留(§3.3/§3.4 标题保留"→ 锁定 1.98.0/18.6"以示反转) |

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
| 3 | QA-024 | MSRV | **D. 锁定 Rust 1.98.0 (2026-08-20 release,2026-08-26 锁定)** | Rust Lead | P1 | ✅ Decided |
| 4 | QA-026 | PostgreSQL | **C. 锁定 PostgreSQL 18.6 + pgvector 0.8.6 (2026-08-26 锁定)** | DBA | P2 | ✅ Decided |
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

### 3.3 QA-024 MSRV → 锁定 Rust 1.98.0(2026-08-26 反转)

**当前状况**:v1.0 决议"跟随 Rust 最新 stable";2026-08-26 决议(WT-H2)反转为**锁定 1.98.0**。

**决议**:**锁定 Rust 1.98.0 (2026-08-20 release)**;基线引用 [CATs_技术基线_v1.0 §1](../02-基础设计/技术选型/CATs_技术基线_v1.0.md)。

**理由(2026-08-26 升级)**:
- 项目跨度 16+ 月,需要可复现构建,不允许 rustup 自动漂移
- 1.98.0 是 2026-08-20 release,2026-08-26 时为最新稳定
- 团队开发机 / CI / 发布镜像三方一致
- 季度评估窗口:2026-11 / 2027-02 / 2027-05 决定是否升 1.99 / 1.100
- 风险:失去"自动跟随新特性"灵活性 → 缓解:明确的季度升版窗口
- 风险:修复 backport 需等下个 release → 缓解:评估必要性,关键 CVE 走 patch 升级

**具体策略**:
- `rust-toolchain.toml` 固定 `channel = "1.98.0"`
- 项目 `Cargo.toml` 写 `rust-version = "1.98.0"`
- CI Runner 镜像:`rust:1.98.0-slim-bookworm`
- Cargo.lock 提交并锁定 transitive deps
- 季度评估一次:是否升到下个 stable

**影响**:
- 所有 crate 用 1.98.0
- 团队开发机固定 1.98.0(rustup 默认 channel 不漂移)
- CI self-hosted runner 用 rust-toolchain.toml 固定
- M1-S0 需做"Hello-Cargo 冒烟"验证各关键 crate 对 1.98.0 兼容性(actix-web / tonic / yrs / tauri / sqlx / rdkafka,详见 Rust 选型书 §19.4)

**同步文档**:
- `CATs_Rust技术选型书_v1.0.md` §3.1 ADR-R-01:版本策略(已升级 v1.1)
- `CATs_開発者ガイド_v1.0.md` §3 必装工具(已升级 v1.2)
- `CATs_CI_CD_構築運用手順書_v1.0.md` §4.2 基础工具(已升级 v1.1)

---

### 3.4 QA-026 PostgreSQL → 锁定 18.6(2026-08-26 反转)

**当前状况**:v1.0 决议"PostgreSQL 最新 stable";2026-08-26 决议(WT-H2)反转为**锁定 PostgreSQL 18.6**。

**决议**:**PostgreSQL 18.6**,CloudNativePG 1.30+ 管理;pgvector **0.8.6**;基线引用 [CATs_技术基线_v1.0 §1](../02-基础设计/技术选型/CATs_技术基线_v1.0.md) 与 [CATs_ADR-003_数据存储选型_v1.1](../02-基础设计/决策/CATs_ADR-003_数据存储选型_v1.1.md)。

**理由(2026-08-26 升级)**:
- 18.6 是 2026-08-26 时点最新稳定;跳过 18.5(已知被跳)
- pgvector 0.8.6 官方镜像 `pgvector/pgvector:pg18-trixie` 已验证兼容
- CloudNativePG 1.30+ 默认 `18.6-system-trixie` 镜像可用
- 风险:18.6 是次新 minor(18.5 被跳),生态适配窗口短 → 缓解:QA-041 Benchmark 强制验证
- 风险:6-12 月后 18.7/18.8 出来需再评估 → 缓解:Minor 升级窗口(2026-11 / 2027-02)写在保守手册

**具体策略**:
- 镜像:`postgres:18.6` / `pgvector/pgvector:pg18-trixie` / CNPG `18.6-system-trixie`
- 所有 migrations / 测试用 18.6
- pgvector 0.8.6 扩展已验证可装
- 18.5 不部署(已知跳过)

**影响**:
- 数据库设计书 §1 / §2:PG 版本描述(后续同步)
- CI 用 `postgres:18.6` 镜像
- K3s 上 CNPG 配置 PG 18.6
- 备份策略适配 18.x 特性

**同步文档**:
- `CATs_数据库设计书_v2.0.md` §1 / §2 PG 版本
- `CATs_Rust技术选型书_v1.0.md` §6 数据持久化(sqlx 0.8+ 兼容 PG 18)
- `CATs_可热插拔部署与运维设计_v1.0.md` §5 PG 配置
- `CATs_ADR-003_数据存储选型_v1.1.md` §1/§3(已独立升版)

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
