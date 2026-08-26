# CATs 可热插拔部署与运维设计书

**系统名称:** CATs — 全媒体 AI 辅助翻译 SaaS 平台

---

## 文档管理信息

| 项目 | 内容 |
|---|---|
| 文档编号 | CATs-ARCH-OPS-001 |
| 文档名 | 可热插拔部署与运维设计书（Hot-Pluggable Deployment & Ops Design） |
| 版本 | 第 1.1 版（基线升级：Rust 1.98.0 + PostgreSQL 18.6 + pgvector 0.8.6） |
| 创建日 | 2026-08-19 |
| 更新 | v1.0 → v1.1（2026-08-26 锁定 Rust 1.98.0 + PG 18.6 + pgvector 0.8.6，见 `CATs_技术基线_v1.0`） |
| 作者 | 架构师 + Rust Lead + DBA（worker 代签 per DEC-008） |
| 状态 | 评审前草稿 |
| 密级 | 仅社内 |
| 上下游文档 | 上游：[CATs 微服务架构设计书 v1.1](./CATs_微服务架构设计书_v1.0.md)、[`CATs_技术基线_v1.0`](../技术选型/CATs_技术基线_v1.0.md)、[CATs 技术选型书 v2.0](../技术选型/CATs_技术选型书_v2.0.md)、[CATs 命名变更说明](./CATs_命名变更说明.md)<br>下游：[CATs 接口设计书 v2.0](../../03-详细设计/接口设计/CATs_接口设计书_v2.0.md)、[CATs 数据库设计书 v2.0](../../03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md)、[CATs 模块设计书 v2.0](../../03-详细设计/模块设计/CATs_模块设计书_v2.0.md) |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|---|---|---|---|
| 1.0 | 2026-08-19 | 架构师 | 初版。补齐架构设计书中横向整合机制：① 原子化部署语义；② 中心事件管理详细规范；③ App 集群管理；④ 可热插拔架构（服务接口/能力宣告/插件接口/配置热更新/特性开关）；⑤ 管理员运维界面设计；⑥ 单功能独立升级流程；⑦ API + 存储过程设计规范 |
| **1.1** | **2026-08-26** | **架构师 + Rust Lead + DBA（worker 代签 per DEC-008）** | **基线升级：统一引用 `CATs_技术基线_v1.0`（Rust 1.98.0 + PostgreSQL 18.6 + pgvector 0.8.6）** |

### 审批栏

| 角色 | 姓名 | 审批日 | 签字 |
|---|---|---|---|
| 起草 |  |  |  |
| 评审 |  |  |  |
| 批准 |  |  |  |

---

## 0. 阅读指南

本书是 CATs 架构设计书的**横向补充**——架构设计书已定义了"按微服务拆分、K3s + Kafka + Debezium 事件驱动"等纵向结构，本书补齐**横向整合机制**：

- **原子化部署**：每个服务/功能的部署是**单一不可分割**的单元，可独立回滚
- **中心事件管理**：所有跨服务事件流经单一中心（含 DLQ、回放、追踪）
- **可热插拔**：服务/插件可运行时增删而不影响其他
- **单功能独立升级**：跨服务的"功能"作为发布单元，灰度 + 回滚
- **管理员运维界面**：Web 化、可视化、RBAC 化的操作面板
- **API + 存储规范**：适应上述所有机制的契约层

配套 Excel `CATs_可热插拔部署与运维设计_v1.0.xlsx` 包含 9 个工作表（部署单元/事件 topic/插件接口/Feature Bundle/RBAC/能力宣告/特性开关/SLO/风险登记册）。

---

## 1. 概述

### 1.1 背景与目标

**背景**：现有架构设计书已定义 16 个微服务（8 核心 + worker-service + translation-core + 6 媒体处理）、K3s 容器编排、Kafka 事件总线、Debezium CDC、Outbox 模式等技术选型。但对于以下**横向整合**问题，规范不足：

| 缺口 | 现有文档 | 缺失 |
|---|---|---|
| 部署单元语义 | 仅说"每个服务独立部署" | 缺：原子性定义、镜像规范、GitOps 流程、回滚契约 |
| 中心事件管理 | 仅说"Kafka + 6 类事件" | 缺：DLQ 处理、事件回放、跨服务追踪、保留策略 |
| 可热插拔 | 仅说"媒体处理可插拔" | 缺：服务能力宣告、配置热更新、特性开关、可选服务降级 |
| 单功能独立升级 | 仅说"按服务部署" | 缺：Feature Bundle 概念、跨服务灰度、Canary 流程、Expand-Contract 迁移 |
| 运维界面 | 完全未涉及 | 缺：管理员可见的 Web 控制台 |
| API/存储规范 | 散落在接口/数据库设计书 | 缺：针对热插拔的 API 兼容性与存储兼容性约束 |

**目标**：补齐这些横向规范，使 CATs 满足以下能力：

- 每个 service 或功能**可独立原子部署**——失败时仅回滚该单元
- 所有跨服务事件流经**中心事件管理**——有统一的 DLQ/回放/监控
- 任意 service 或插件**可热插拔**——不依赖、不重启其他
- 任意"功能"（跨服务组合）**可独立升级**——Canary + 灰度 + 一键回滚
- **管理员运维界面** Web 化、RBAC 化、可视化
- **API + 存储**规范完全适应上述机制

### 1.2 范围

**在范围内：**

- 原子化部署的契约层（镜像命名、GitOps 流程、回滚策略）
- 中心事件管理的运营机制（DLQ、回放、追踪、保留）
- 可热插拔的服务/插件设计模式
- 管理员运维界面的模块划分与 RBAC
- 单功能独立升级的发布单元（Feature Bundle）与流程
- API 设计规范（针对热插拔的版本化、兼容性、错误码、幂等性、追踪）
- 存储过程设计规范（针对热插拔的 schema 隔离、Outbox、Expand-Contract 迁移）

**对象外：**

- 具体业务功能实现（属于详细设计/模块设计）
- 模型选择与训练（属于 AI 团队职责）
- 商业 LLM 选型（见技术选型书 ADR）
- 移动端（不在当前交付范围）

### 1.3 术语定义

| 术语 | 定义 |
|---|---|
| 原子化部署（Atomic Deployment） | 一个部署单元的更新要么**完全成功**要么**完全回滚**，无中间状态。 |
| 部署单元（Deployment Unit） | 不可分割的部署单位。本书定义三级：**镜像**（OCI）/**服务**（Deployment 资源集）/**功能**（Feature Bundle 跨服务组合）。 |
| 中心事件（Central Event） | 所有跨服务事件流经单一 Kafka 集群（架构设计书 §6）。 |
| DLQ（Dead Letter Queue） | 消费失败的消息暂存队列。中心化管理而非各服务私有。 |
| 事件回放（Event Replay） | 从 Kafka 起始 offset 重放历史事件到消费者，用于灾备恢复或新服务初始化。 |
| 能力宣告（Capability Advertisement） | 服务通过 `/internal/v1/capabilities` 端点声明自身支持的能力，供其他服务/运维/插件发现。 |
| 特性开关（Feature Flag） | 运行时控制功能启用的开关，独立于代码部署。 |
| 插件（Plugin） | 通过 trait/interface 注入的可替换实现（如 render-writer 的 5 种 Renderer）。 |
| Feature Bundle | 跨多个服务协同发布的"功能"——一组服务的特定版本构成一个可独立发布/回滚的单元。 |
| Canary | 新版本先对少量流量/用户发布，逐步扩量。 |
| Expand-Contract 迁移 | 兼容性数据库迁移三步法：① 加新列（Expand）→② 部署应用切换读写→③ 删除旧列（Contract）。 |
| 灰度（Gray Release） | 按用户/项目/域名等维度分流部分流量到新版本。 |
| RBAC | Role-Based Access Control，基于角色的访问控制。 |
| PDB（PodDisruptionBudget） | K8s 资源，限制自愿中断时存活的最小 Pod 数。 |
| HPA（HorizontalPodAutoscaler） | K8s 资源，根据 CPU/内存/自定义指标自动扩缩容。 |

### 1.4 关联文档

- 架构设计书：[CATs 微服务架构设计书 v1.0](./CATs_微服务架构设计书_v1.0.md)
- 技术选型：[CATs 技术选型书 v2.0](../技术选型/CATs_技术选型书_v2.0.md)
- 命名变更：[CATs 命名变更说明](./CATs_命名变更说明.md)
- 接口设计：[CATs 接口设计书 v2.0](../../03-详细设计/接口设计/CATs_接口设计书_v2.0.md)
- 数据库设计：[CATs 数据库设计书 v2.0](../../03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md)
- 模块设计：[CATs 模块设计书 v2.0](../../03-详细设计/模块设计/CATs_模块设计书_v2.0.md)
- 测试设计：[CATs 测试设计书 v1.0](../../04-测试/测试设计书/CATs_测试设计书_v1.0.md)

### 1.5 引用标准

| 标准 | 用途 |
|---|---|
| [CNCF K8s Operator 白皮书](https://www.cncf.io/) | K8s 控制器模式参考 |
| [12-Factor App](https://12factor.net/) | 微服务/云原生应用设计原则 |
| [OpenFeature](https://openfeature.dev/) | 特性开关标准规范 |
| [CloudEvents v1.0](https://cloudevents.io/) | 事件 schema 规范（CNCF 沙箱） |
| [AsyncAPI 2.x](https://www.asyncapi.com/) | 事件/消息 API 契约规范 |
| [Semantic Versioning 2.0](https://semver.org/) | 镜像/API 版本号规则 |
| [Conventional Commits](https://www.conventionalcommits.org/) | 提交信息规范 |
| [JIS X 0129](https://www.jisc.go.jp/) | 配套测试设计（见测试设计书 §1.4） |

---

## 2. 设计原则

| 编号 | 原则 | 说明 |
|---|---|---|
| P-01 | **单一权威** | 每个业务数据只有一处权威存储（PostgreSQL），其他都是缓存/视图/派生。 |
| P-02 | **原子性优先** | 一个部署单元的更新要么完全成功要么完全回滚；一个业务事务要么完全提交要么完全回滚。 |
| P-03 | **可热插拔** | 服务/插件可通过注册中心动态加入/移除，运行时不影响其他。 |
| P-04 | **可独立升级** | 任意 Feature Bundle 可独立部署、灰度、回滚，不与其他耦合。 |
| P-05 | **中心事件** | 跨服务事件流经单一 Kafka 集群，便于统一监控、追溯、回放。 |
| P-06 | **向后兼容** | API/Schema 变更默认向后兼容；破坏性变更必须发布新版本（/v2/）。 |
| P-07 | **声明式部署** | 所有部署通过 GitOps（Argo CD）声明式定义，集群状态 = Git 状态。 |
| P-08 | **可观测** | 任何部署、事件、调用都有 Trace ID 贯穿；操作有审计日志。 |
| P-09 | **最小特权** | 运维操作按 RBAC 最小特权授予；变更需审批。 |
| P-10 | **失败安全** | 敏感内容处理链路失败时**中止**而非降级（架构设计书 §1.2 原则 3）。 |
| P-11 | **渐进发布** | 新版本默认走 Canary（5% → 25% → 50% → 100%），验证通过后再扩量。 |

---

## 3. 原子化部署

### 3.1 部署单元定义

CATs 的部署单元分为**三级**：

| 级别 | 单元 | 原子性边界 | 适用场景 |
|---|---|---|---|
| **L1: 镜像** | OCI 镜像 | 单个 OCI 镜像的构建/推送/拉取 | 持续集成产物 |
| **L2: 服务** | K8s Deployment 资源集（Pod + Service + ConfigMap + Secret + PDB + HPA + NetworkPolicy） | 单个服务的一次部署/回滚 | 通用服务升级 |
| **L3: 功能** | Feature Bundle（跨多个服务的特定版本组合） | 一个业务功能的一次发布/回滚 | 跨服务的"功能"灰度 |

```
┌─────────────────────────────────────────────────┐
│ L3: Feature Bundle  (例: PDF 文档翻译功能)        │
│ ┌──────────┐ ┌──────────┐ ┌──────────┐          │
│ │L2: office│ │L2:       │ │L2: render│          │
│ │converter │ │translation│ │ writer   │          │
│ │  v1.3.2  │ │ core     │ │  v1.2.0  │          │
│ │          │ │ v2.0.1   │ │          │          │
│ └────┬─────┘ └────┬─────┘ └────┬─────┘          │
│      │            │            │                │
│ ┌────▼─────┐ ┌────▼─────┐ ┌────▼─────┐          │
│ │L1: OCI   │ │L1: OCI   │ │L1: OCI   │          │
│ │镜像     │ │镜像     │ │镜像     │          │
│ │sha256..│ │sha256..│ │sha256..│          │
│ └──────────┘ └──────────┘ └──────────┘          │
└─────────────────────────────────────────────────┘
```

### 3.2 镜像命名规范

```
{service}-{version}-{feature_set}-{git_sha}-{build_num}

例: office-converter-1.3.2-stable-a1b2c3d4-42
    translation-core-2.0.1-canary-d5e6f7a8-7
    render-writer-1.2.0-stable-f9g0h1i2-15
```

| 字段 | 规则 | 示例 |
|---|---|---|
| `{service}` | 小写连字符，与 K8s 资源名一致 | `office-converter` |
| `{version}` | SemVer 2.0（major.minor.patch），major 不轻易变更 | `1.3.2` |
| `{feature_set}` | `stable` / `canary` / `beta` / 特定 feature 名（多租户实验时） | `stable` |
| `{git_sha}` | 7 字符短哈希 | `a1b2c3d` |
| `{build_num}` | CI 构建序号（单调递增） | `42` |

镜像推送策略：
- `latest` 标签 = 最后一个 stable 版本（仅供开发使用，**禁止生产使用**）
- `{version}` 标签 = 该 stable 版本（可回滚）
- `{feature_set}-{git_sha}` 标签 = 不可变引用（CI 唯一标识）

### 3.3 部署流程（GitOps）

```
┌─────────────┐    git push     ┌─────────────┐
│  Developer  │ ───────────────▶│  Git Repo   │
└─────────────┘                  └──────┬──────┘
                                        │ webhook
                                        ▼
┌─────────────┐  build+test   ┌─────────────┐
│ CI Pipeline │ ◀─────────────│   Argo CD   │
│ (Gitea/     │  sync         │  (Sync)     │
│  Jenkins)   │               └─────────────┘
└──────┬──────┘
       │ push
       ▼
┌─────────────┐
│   Harbor    │  (镜像仓库)
└─────────────┘
       │ pull
       ▼
┌─────────────────────────────────────┐
│  K8s 集群                           │
│  ┌──────────────────────────────┐  │
│  │ Deployment (new)             │  │
│  │   replicas: N                │  │
│  │   image: foo:v1.3.2-stable   │  │
│  └──────────────────────────────┘  │
│           │                         │
│           ▼ RollingUpdate          │
│  ┌──────────────────────────────┐  │
│  │ old Pod ──▶ new Pod          │  │
│  │   readiness: ✓               │  │
│  └──────────────────────────────┘  │
└─────────────────────────────────────┘
```

**部署契约**：

1. **声明式**：所有部署状态由 Git 仓库的 Helm values + Application manifest 定义
2. **自动同步**：dev/STG 环境自动同步；PRE/PROD 需审批
3. **可回滚**：`git revert` + Argo CD 自动检测 + 同步（**回滚时长 < 5min**，架构设计书 §1.1 目标）
4. **可暂停**：Argo CD 提供 `suspend` 操作，紧急时暂停所有同步
5. **审计**：每次同步记录到 `audit.events`（架构设计书 §4.2）

### 3.4 镜像不可变原则

> 同一镜像 tag **不可被覆盖**。

实现：
- Harbor 启用"不可变标签"策略
- CI 中 `docker push` 失败重试时不重新 tag，而是生成新 build_num
- 生产引用必须使用 `{git_sha}` 或 `{version}` 标签，不使用 `latest`

### 3.5 回滚契约

| 触发 | 方式 | 时长 | 数据影响 |
|---|---|---|---|
| 主动回滚 | Argo CD `Rollback` 按钮 | < 5min | 仅 Pod 替换，无 schema 影响 |
| 自动回滚（Helm 钩子） | `maxUnavailable` + 健康检查失败 N 次 | < 10min | 同上 |
| 紧急回滚（Canary 失败） | Argo CD 暂停 + 100% 切回旧版 | < 3min | 同上 |
| 数据库迁移回滚 | 见 §10.4 Expand-Contract 反向 | < 30min | 需预先准备 reverse 脚本 |

---

## 4. 中心事件管理

### 4.1 事件总线架构

CATs 的中心事件总线 = **单一 Kafka 集群（KRaft 模式）+ Outbox 模式 + Debezium CDC + Schema Registry**。

```
                          ┌──────────────────────────────┐
                          │  Schema Registry             │
                          │  (主题 schema 版本化)        │
                          └──────────────────────────────┘
                                      ▲    │
                                      │    ▼
┌──────────────┐  Outbox ──▶ Debezium │    │ Validation
│  Service A   │ ─────────▶ CDC ──────┤    │
│ (PostgreSQL) │                      │    │
└──────────────┘                      ▼    ▼
┌──────────────┐  Outbox ──▶ Debezium ┌─────────────────┐
│  Service B   │ ─────────▶ CDC ─────▶│  Kafka Cluster  │
│ (PostgreSQL) │                      │  (3 brokers)     │
└──────────────┘                      │                 │
┌──────────────┐  Outbox ──▶ Debezium │  topic-per-     │
│  Service C   │ ─────────▶ CDC ─────▶│  domain         │
│ (PostgreSQL) │                      └────────┬────────┘
└──────────────┘                               │
                                                │
              ┌─────────────────────────────────┼─────────────────────────────────┐
              │                                 │                                 │
              ▼                                 ▼                                 ▼
     ┌────────────────┐              ┌────────────────┐              ┌────────────────┐
     │ Domain: task   │              │ Domain: file   │              │ Domain: ...    │
     │   topic.media.*│              │   events       │              │                │
     │   .requested   │              │                │              │                │
     │   .completed   │              │                │              │                │
     └────────┬───────┘              └────────┬───────┘              └────────┬───────┘
              │                                 │                                 │
              ▼                                 ▼                                 ▼
     ┌────────────────┐              ┌────────────────┐              ┌────────────────┐
     │ Consumer A     │              │ Consumer B     │              │ Consumer N     │
     │ (asr-service)  │              │ (ingestion-    │              │ (notification- │
     │                │              │  service)      │              │  service)      │
     └────────────────┘              └────────────────┘              └────────────────┘
              │                                 │                                 │
              ▼                                 ▼                                 ▼
        On success:                  On success:                    On success:
        commit offset                 commit offset                  commit offset

        On failure:                  On failure:                    On failure:
        retry 1 (10s)                retry 1 (10s)                  retry 1 (10s)
        retry 2 (1min)               retry 2 (1min)                 retry 2 (1min)
        retry 3 (10min)              retry 3 (10min)                retry 3 (10min)
        DLQ: {domain}.dlq            DLQ: {domain}.dlq              DLQ: {domain}.dlq
```

### 4.2 事件 Schema 规范

按 **CloudEvents v1.0** 规范 + 自定义扩展：

```json
{
  "specversion": "1.0",
  "id": "evt_01HZX...",                // 唯一事件 ID
  "source": "/cats/task-service",       // 事件源
  "type": "cats.task.completed.v1",     // 类型（含版本）
  "datacontenttype": "application/json",
  "subject": "tsk_01HZX...",            // 资源 ID
  "time": "2026-08-19T09:00:00Z",
  "schema_version": 1,                  // payload schema 版本
  "traceparent": "00-...",              // OTel Trace Context
  "dataref": "file://...",              // 可选：引用大数据
  "data": {
    // 业务 payload（向后兼容）
  }
}
```

**主题命名**：`{domain}.{entity}.{event_type}.v{n}`

| 域 | 实体 | 事件类型 | 示例 |
|---|---|---|---|
| `task` | `task` | `created/updated/canceled/completed/failed` | `task.task.created.v1` |
| `task` | `media` | `asr.requested/asr.completed/ocr.requested/...` | `task.media.asr.requested.v1` |
| `file` | `file` | `uploaded/version_added/deleted` | `file.file.uploaded.v1` |
| `user` | `user` | `created/updated` | `user.user.created.v1` |
| `user` | `org` | `member_added/subscription_changed` | `user.org.member_added.v1` |
| `project` | `project` | `created/updated/glossary_updated/tm_updated/compliance_policy_changed` | `project.project.glossary_updated.v1` |
| `audit` | `*` | `*` | `audit.*.*.v1` |
| `notification` | `*` | `*` | `notification.*.*.v1` |
| `*.dlq` | — | 死信 | `task.dlq.v1` |

**schema 演进规则**（参考 CloudEvents）：

- ✅ **向后兼容**：新增可选字段、新增事件类型、新增枚举值
- ❌ **破坏性变更**：删除字段、改字段类型、改字段语义、改枚举值
- 破坏性变更 → 发布新版本（`type=v2`），老类型保留 2 个发布周期供消费者迁移

### 4.3 DLQ 管理

DLQ 在每个域独立：`{domain}.dlq.v1`

**消息结构**（DLQ 特殊头）：

```json
{
  "id": "evt_01HZX...",
  "type": "cats.task.media.asr.requested.v1",
  "traceparent": "00-...",
  "original_topic": "task.media.asr.requested.v1",
  "original_partition": 3,
  "original_offset": 12345,
  "consumer": "asr-service",
  "error_code": "ASR_INFERENCE_FAILED",
  "error_message": "GPU OOM after 3 retries",
  "retry_count": 3,
  "first_failed_at": "2026-08-19T09:00:00Z",
  "last_failed_at": "2026-08-19T09:00:30Z",
  "payload": { /* 原始事件 */ }
}
```

**DLQ 操作**（通过 §7 运维界面）：
- 查看 DLQ 消息
- 按错误码/时间范围过滤
- 重放单条/批量到原 topic
- 永久删除（归档后）
- 自动告警：DLQ 消息数 > 阈值 → Slack/钉钉告警

### 4.4 事件回放

**场景**：
- 新服务初始化（消费历史事件填充数据）
- 灾备恢复（DR 演练后从 Kafka 恢复数据库）
- 修复消费者 bug 后重放

**回放 API**（运维界面）：

| 参数 | 类型 | 说明 |
|---|---|---|
| `topic` | string | 目标 topic |
| `start_offset` | int/-1 | 起始 offset（-1 = 最早，0 = 最新） |
| `end_offset` | int/-1 | 结束 offset |
| `partition` | int/-1 | 分区（-1 = 全部） |
| `consumer_group` | string | 目标消费者组 |
| `filter` | object | 可选：按 event_type 过滤 |

**回放模式**：
- **完全重放**：消费者组 offset 强制设到 start_offset
- **旁路重放**：创建临时消费者组，不影响生产
- **选择性重放**：仅重放特定 event_id / 特定时间范围

**安全约束**：
- 回放需要 platform_admin 角色 + 审批
- 回放期间消费者告警
- 回放后做对账（与目标系统状态一致）

### 4.5 事件追踪

每个事件携带 OTel Trace Context，跨服务追踪：

```
Event: task.task.created.v1
trace_id: 4bf92f3577b34da6a3ce929d0e0e4736
span_id: 00f067aa0ba902b7
parent_span_id: 00f067aa0ba902a0  (来自 HTTP API)

   task-service (created event)
   ├── span: create_task
   │   ├── span: db_insert (PostgreSQL)
   │   └── span: outbox_write (PostgreSQL)
   ├── debzium (CDC)
   │   └── span: kafka_publish
   ├── asr-service (consume)
   │   ├── span: kafka_consume
   │   ├── span: asr_inference
   │   └── span: db_insert (asr_transcripts)
   └── render-writer (consume)
       └── ...
```

通过 Tempo 可查询 `trace_id=4bf92f...` 看到完整链路。

### 4.6 事件保留策略

| Topic 类型 | 保留期 | 压缩 | 说明 |
|---|---|---|---|
| 业务事件（task/file/user/project） | 7 天 | 启用（log cleanup） | 短窗口：消费后可压缩 |
| DLQ | 30 天 | 不压缩 | 长保留：留出排障时间 |
| 审计事件 | 365 天 | 不压缩 | 合规要求 |
| 通知事件 | 1 天 | 不压缩 | 短保留：实时性 |

---

## 5. App 集群管理

### 5.1 节点池（Node Pool）

| 池 | 标签 | 用途 | 污点 |
|---|---|---|---|
| `pool=general` | 通用业务微服务 | auth/user/project/task/file/notification/report/audit/worker/translation-core | 无 |
| `pool=media-cpu` | ffmpeg/PaddleOCR/LibreOffice（CPU 密集） | ingestion/ocr/office-converter/subtitle | `media-cpu=true:NoSchedule` |
| `pool=media-gpu` | faster-whisper/OCR GPU 加速 | asr（需要时） | `nvidia.com/gpu=true:NoSchedule` |
| `pool=data` | PostgreSQL/Kafka/Valkey/Harbor 有状态 | 平台组件 | `data=true:NoSchedule` |

媒体处理服务调度示例（`render-writer-service`）：

```yaml
spec:
  template:
    spec:
      tolerations:
        - key: media-cpu
          operator: Equal
          value: "true"
          effect: NoSchedule
      nodeSelector:
        pool: media-cpu
```

### 5.2 弹性伸缩（HPA）

| 服务 | 最小副本 | 最大副本 | 触发指标 |
|---|---|---|---|
| auth-service | 2 | 10 | CPU > 70% |
| user-service | 2 | 6 | CPU > 70% |
| project-service | 2 | 8 | CPU > 70% |
| task-service | 2 | 10 | CPU > 70% |
| file-service | 2 | 6 | CPU > 70% |
| notification-service | 2 | 6 | CPU > 70% |
| report-service | 1 | 4 | CPU > 70% |
| audit-service | 1 | 4 | CPU > 70% |
| translation-core | 3 | 30 | **Kafka consumer lag**（业务关键） |
| ingestion-service | 1 | 5 | Kafka consumer lag |
| asr-service | 2 | 10 (GPU 节点) | Kafka consumer lag |
| ocr-service | 2 | 10 | Kafka consumer lag |
| subtitle-service | 1 | 5 | Kafka consumer lag |
| office-converter-service | 2 | 8 | Kafka consumer lag |
| render-writer-service | 1 | 5 | Kafka consumer lag |
| worker-service | 1 | 3 | Cron schedule |
| Tauri 客户端 | N/A | N/A | 桌面应用 |

**自定义指标**（Kafka consumer lag）需 K8s Prometheus Adapter + KEDA。

### 5.3 Pod 中断预算（PDB）

| 服务 | minAvailable | 说明 |
|---|---|---|
| auth-service | 1 | 至少 1 副本可用 |
| translation-core | 2 | 高频调用，至少 2 副本 |
| task-service | 1 | 核心编排 |
| 其他业务服务 | 1 | 至少 1 副本可用 |
| 媒体处理服务 | 1 | 至少 1 副本可用（可短暂延迟） |

### 5.4 集群拓扑

3 控制面节点（HA）+ N 工作节点（按角色池分组），详见架构设计书 §2 拓扑图。

---

## 6. 可热插拔架构

### 6.1 服务接口版本化

详见接口设计书 §1.5，此处补充**热插拔**特定约束：

- **URL 版本化**：`/v1/...`、`/v2/...`——破坏性变更必须升 v
- **gRPC package 版本化**：`cats.task.v1` / `cats.task.v2`——同上
- **Event type 版本化**：`cats.task.completed.v1` / `cats.task.completed.v2`——同上
- **共存期**：旧版本至少保留 **2 个发布周期**（约 6 个月）

### 6.2 服务能力宣告

每个服务通过内部端点声明自身能力，供其他服务/插件/运维 UI 发现。

**端点**：`GET /internal/v1/capabilities`

**响应**：

```json
{
  "service": "translation-core",
  "version": "2.0.1",
  "build": {
    "git_sha": "a1b2c3d4",
    "build_num": 42,
    "built_at": "2026-08-18T09:00:00Z"
  },
  "runtime": {
    "language": "python",
    "framework": "fastapi",
    "grpc_enabled": true,
    "rest_enabled": true
  },
  "capabilities": {
    "translates": {
      "supported_languages": ["ja", "en", "zh-CN", "zh-TW", "ko"],
      "max_segment_length": 5000,
      "streaming": true,
      "modes": ["default", "high_quality"]
    },
    "tm": {
      "exact_match": true,
      "fuzzy_match": true,
      "semantic_match": true,
      "min_score": 75
    },
    "glossary": {
      "enforcement": "post-translation",
      "fallback": "warning"
    },
    "tag_protection": {
      "placeholders": ["{var}", "%s", "{{var}}"],
      "html_tags": ["<b>", "<i>", "<u>"]
    },
    "compliance": {
      "local_model_required": false,
      "fallback_on_local_unavailable": "block"
    }
  },
  "dependencies": {
    "project-service": ">=1.5.0",
    "ai-gateway": ">=2.0.0"
  },
  "health": {
    "status": "healthy",
    "uptime_seconds": 3600,
    "last_restart_at": "2026-08-18T08:00:00Z"
  }
}
```

**消费者**：
- 运维 UI（§7）显示每个服务的能力
- 客户端按能力选择使用模式（如不支持 streaming 则降级到非流式）
- 插件系统按能力动态注入实现
- 灰度发布时按能力差异选择流量

### 6.3 服务发现

| 场景 | 机制 | 说明 |
|---|---|---|
| K8s 内部 | K8s DNS（`{service}.{namespace}.svc.cluster.local`） | 默认 |
| 外部 → 内部 | Envoy Gateway + JWT | 通过 `https://api.cats.internal/v1/...` |
| 灰度/Canary | Envoy Gateway `HTTPRoute` 权重路由 | 灰度发布期使用 |
| 服务间可选依赖 | 服务发现 + 能力宣告 | 缺失时降级 |

### 6.4 配置热更新

| 配置类型 | 存储 | 更新方式 | 生效时间 |
|---|---|---|---|
| 普通配置 | K8s ConfigMap + Reloader | `kubectl apply` | < 30s（监听文件变化） |
| 敏感配置 | K8s Secret + External Secrets | 通过 SealedSecrets | < 1min |
| 特性开关 | 专门 feature-flag 服务 | API 调用 | < 5s |
| 运行时参数 | Valkey（缓存层） | API 调用 | 即时 |
| 业务参数（如术语库） | PostgreSQL + 缓存 | API 调用 | 即时（缓存 TTL） |

**Reloader**：监听 ConfigMap/Secret 变化，自动滚动重启 Pod。

### 6.5 特性开关

按 **OpenFeature** 规范实现独立的 feature-flag 服务。

**API**：

```
GET  /v1/flags/{flag_key}?context={user_id}&context={org_id}
PUT  /v1/flags/{flag_key}   # platform_admin
```

**Flag 分类**：

| 类别 | 示例 | 默认 | 评估时机 |
|---|---|---|---|
| 发布类（Release） | `enable_pdf_relayout_v2` | false | 用户请求时 |
| 灰度类（Rollout） | `pdf_relayout_v2_rollout_pct` | 10 | 用户请求时（带 % 评估） |
| 实验类（Experiment） | `tm_match_algorithm` | "v1" | 启动时 |
| 运维类（Ops） | `disable_asr_for_maintenance` | false | 立即生效 |
| 合规类（Compliance） | `force_local_model_for_org_X` | false | 用户请求时 |

**Flag 评估上下文**：
- `user_id` / `org_id` / `project_id`
- `domain`（域名）
- `user_agent` / `client_version`
- `request_id`（追踪用）

**实现**：在每个服务内嵌入 OpenFeature SDK（轻量客户端），不依赖网络调用 flag 服务（首次启动拉取全量 flag 到内存，定时刷新）。

### 6.6 插件接口设计

按 Rust trait / Python Protocol 定义可插拔点。已有先例：模块设计书 §1.2 `render-writer-service` 的 `ports::Renderer` trait。

**插件类型**：

| 插件 | Trait/Protocol | 实现 | 注入方式 |
|---|---|---|---|
| 渲染器（Renderer） | `Renderer::render(&self, job: RenderJob) -> Result<RenderOutput>` | 5 种（subtitle_burn_in / pdf_relayout / office_passthrough / gif_reencode / webp_reencode） | main.rs 按 render_kind 注入 |
| OCR 引擎 | `OcrEngine::recognize(&self, image) -> Result<TextRegions>` | PaddleOCR / Tesseract | DI |
| ASR 模型 | `AsrModel::transcribe(&self, audio) -> Result<Transcript>` | faster-whisper-medium / faster-whisper-large | DI |
| 翻译模型提供者 | `ModelProvider::translate(&self, segment) -> Result<Translation>` | OpenAI/Claude/Gemini/DeepSeek/本地 Qwen | LiteLLM 路由 |
| 存储后端 | `Storage::put(&self, key, data)` | MinIO / NFS | DI |
| 通知渠道 | `NotificationChannel::send(&self, msg)` | WS / 邮件 / 桌面 | DI |

**插件发现**：
- 编译时：通过 `inventory` crate / `entry_points` 机制注册
- 运行时：通过 capability 端点 + feature flag

### 6.7 可选服务降级

| 可选服务 | 缺失时行为 |
|---|---|
| asr-service | 视频/音频翻译走 OCR + 字幕烧录降级路径；返回 503 提示用户 |
| ocr-service | 图片/扫描件走纯文本路径；返回 503 |
| subtitle-service | 视频不生成字幕文件，直接用 ASR 结果 |
| office-converter-service | Office 文档翻译不可用；返回 503 |
| render-writer-service | 仅返回翻译文本，不产出文件 |
| worker-service | 对账/批量导入手动执行 |
| ai-gateway | 翻译核心 fail-closed（敏感项目）或返回错误（普通项目） |
| Valkey | 服务降级为直连 PostgreSQL（性能下降） |
| Kafka | Outbox 累积不消费；本地有队列但跨服务协同暂停 |

降级策略在 §6.2 capabilities 端点中显式声明，其他服务/客户端可据此决策。

---

## 7. 管理员运维界面

### 7.1 设计目标

- **Web 化**：浏览器访问，无客户端安装
- **RBAC**：细粒度角色权限
- **可视化**：图表/拓扑/实时数据
- **可审计**：所有操作留痕
- **可审批**：高风险操作需审批

### 7.2 模块清单

```
                    ┌─────────────────────────────────┐
                    │  顶部：全局搜索 / 用户菜单 / 通知   │
                    └────────────┬────────────────────┘
                                 │
       ┌──────────────┬──────────┼──────────┬──────────────┐
       ▼              ▼          ▼          ▼              ▼
   ┌────────┐   ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐
   │服务总览│   │部署管理│  │事件总线│  │特性开关│  │数据库  │
   │        │   │        │  │        │  │        │  │        │
   │服务状态│   │升级/回滚│  │Topic   │  │Flag    │  │迁移    │
   │健康检查│   │暂停/继续│  │消费 Lag│  │灰度%   │  │Schema  │
   │版本    │   │Canary  │  │DLQ     │  │A/B     │  │备份    │
   │流量    │   │审批    │  │回放    │  │审计    │  │        │
   └────────┘   └────────┘  └────────┘  └────────┘  └────────┘
       │              │          │          │              │
       └──────────────┴──────────┼──────────┴──────────────┘
                                 ▼
                    ┌─────────────────────────────────┐
                    │  底部：审计 / 通知 / 帮助         │
                    └─────────────────────────────────┘
```

### 7.3 关键模块详细设计

#### 7.3.1 服务总览

**视图**：
- 16 个服务的健康卡片（按命名空间分组）
- 每卡显示：服务名 / 版本 / 副本数（X/Y ready）/ 错误率 / p95 延迟 / 流量 QPS
- 状态颜色：绿（healthy）/ 黄（degraded）/ 红（down）
- 点击进入服务详情页

**功能**：
- 查看服务 capabilities（§6.2 端点）
- 查看服务日志（聚合 Loki）
- 查看服务 Trace（跳转到 Tempo）
- 查看服务 Metrics（跳转到 Grafana）
- 重启/缩容/扩容（需权限 + 审批）

#### 7.3.2 部署管理

**视图**：
- 当前版本（Git SHA + 版本号 + 部署时间 + 部署人）
- 历史版本列表（可回滚）
- 当前进行中的部署进度
- Feature Bundle 视图（一个功能包含哪些服务/版本）

**功能**：
- **升级**：
  - 选择目标版本（Git Tag / OCI Tag）
  - 选择范围：单服务 / Feature Bundle
  - 选择策略：立即 / Canary（5%-25%-50%-100%）/ 灰度（按用户/项目）
  - 提交 → 审批（如生产环境）→ 执行
- **回滚**：
  - 选择目标版本（历史版本）
  - 范围：单服务 / Feature Bundle
  - 一键执行（< 5min，架构设计书 §1.1 目标）
- **暂停 / 继续**：
  - 暂停：Argo CD `suspend`（紧急时停止所有同步）
  - 继续：恢复同步
- **Canary 进度监控**：
  - 实时显示各阶段流量比例
  - 错误率/p95 延迟对比新旧版本
  - 自动/手动 promote（推进到下一阶段）或 abort（回滚）

#### 7.3.3 事件总线监视

**视图**：
- Topic 列表（每个域）
- 每个 Topic 显示：分区数 / 副本数 / in-sync 副本数 / 总消息数 / 入速率 / 出速率 / 存储大小
- Consumer Group 列表：每个组的 lag / offset / 消费速率
- DLQ 总览：各域 DLQ 消息数 + 最近错误

**功能**：
- 查看 Topic 详情（分区级指标）
- 查看 Consumer 详情（lag 趋势）
- **DLQ 管理**：
  - 列出 DLQ 消息
  - 按错误码/时间过滤
  - 查看消息详情（含 trace_id 跳转）
  - **重放**：选择单条/批量 → 选择目标消费者组 → 确认
  - **归档删除**：永久删除（仅 platform_admin）
- **事件回放**（§4.4）：
  - 选择 topic / offset 范围
  - 选择消费者组
  - 选择重放模式（完全 / 旁路 / 选择性）
  - 提交审批
- **Schema 管理**：
  - 查看各 schema 版本
  - 比较 schema 差异
  - 注册新版本

#### 7.3.4 特性开关管理

**视图**：
- Flag 列表（按类别）
- 每个 Flag 显示：key / 类别 / 默认值 / 当前评估规则 / 影响范围

**功能**：
- 查看 Flag 详情
- 启/停 Flag
- 修改评估规则（布尔/百分比/分桶）
- 灰度切换（按维度）
- A/B 实验配置
- 操作历史

#### 7.3.5 数据库迁移

**视图**：
- 各逻辑库当前 schema 版本
- 迁移历史
- 待执行迁移

**功能**：
- 查看迁移详情
- **执行 Forward**（需 platform_admin + 审批）
- **执行 Rollback**（需预先准备 reverse 脚本）
- 迁移前后对比（schema diff）
- 备份/恢复（链接到 Velero / pgBackRest）

#### 7.3.6 日志/指标查询

**集成**：
- 日志：Loki + Grafana Explore（嵌入或跳转）
- 指标：Prometheus + Grafana
- Trace：Tempo
- 告警：Alertmanager（按 SLO 触发）

#### 7.3.7 审计日志

**视图**：
- 操作列表：谁 / 何时 / 操作了什么 / 结果
- 过滤：用户 / 时间 / 操作类型 / 服务 / 项目

**不可篡改**：审计日志不可由非 platform_admin 删除。

#### 7.3.8 DR 演练

**视图**：
- 当前 DR 状态
- 历史演练记录

**功能**：
- **触发演练**（dry-run）：模拟故障，验证恢复
- **回滚演练**：数据恢复到指定时间点
- **验证清单**：恢复时长 / 数据完整性 / 服务可用性

### 7.4 RBAC

| 角色 | 权限 |
|---|---|
| `viewer` | 只读：查看服务状态、Metrics、日志、Trace |
| `operator` | viewer + 重启/扩缩容/暂停同步/查看 DLQ |
| `deployer` | operator + 升级/回滚/Canary（仅 STG/PRE） |
| `org_admin` | viewer + 切换 org 维度的 Feature Flag + 查看 org 用量 |
| `platform_admin` | 全部权限（含生产部署审批/DLQ 删除/Schema 变更/DR 演练） |
| `auditor` | 只读：所有操作 + 审计日志（不可被其他角色改动） |

RACI 见 `CATs_可热插拔部署与运维设计_v1.0.xlsx` 「RACI」工作表。

### 7.5 审批工作流

| 操作 | 是否需审批 | 审批人 |
|---|---|---|
| STG/PRE 升级 | 否（自动） | — |
| PROD 升级 | 是 | 至少 1 名 platform_admin |
| PROD Canary 推进到 50%/100% | 是 | 至少 2 名 platform_admin |
| 任何回滚 | 是 | 至少 1 名 platform_admin |
| DLQ 永久删除 | 是 | 至少 2 名 platform_admin |
| Schema 变更 | 是 | 至少 2 名 platform_admin（含 DBA） |
| DR 演练 | 是 | 至少 1 名 platform_admin + 1 名 SRE |
| 特性开关影响范围 > 10% 用户 | 是 | 1 名 platform_admin |
| 紧急暂停（Suspend） | 否（事后审计） | 事后补审批 |

审批通过 Argo CD Notifications 触发（Slack/钉钉/邮件），审批人在 UI 内完成。

### 7.6 安全

- HTTPS（内部 CA）
- JWT 鉴权（auth-service 统一签发）
- 操作记录到 `audit.events`
- 关键操作双因素认证（2FA，如生产部署）
- 越权操作直接拒绝（中间件 + BFF 双重）

---

## 8. 单功能独立升级

### 8.1 Feature Bundle 概念

**Feature Bundle** = 一组服务的特定版本组合，**构成一个业务功能**。

**示例**：

| Feature Bundle | 包含服务（特定版本） | 业务能力 |
|---|---|---|
| `pdf-translation-v2` | `ingestion v1.5.0` + `ocr v2.0.0` + `translation-core v2.1.0` + `render-writer v1.3.0` | PDF 文档翻译 V2（增强版面保留） |
| `video-subtitle-burn-in` | `ingestion v1.5.0` + `asr v1.2.0` + `subtitle v1.1.0` + `render-writer v1.3.0` | 视频字幕烧录 |
| `compliance-local-model` | `translation-core v2.1.0` + `ai-gateway v1.5.0` | 合规本地模型路由 |

**Feature Bundle 标识**（在 Git 仓库）：

```
manifests/
  features/
    pdf-translation-v2.yaml
    video-subtitle-burn-in.yaml
    compliance-local-model.yaml
```

`pdf-translation-v2.yaml` 示例：

```yaml
apiVersion: feature.cats/v1
kind: FeatureBundle
metadata:
  name: pdf-translation-v2
  version: "1.0.0"
spec:
  description: "PDF 文档翻译 V2（增强版面保留）"
  services:
    - name: ingestion
      version: "1.5.0"
    - name: ocr
      version: "2.0.0"
    - name: translation-core
      version: "2.1.0"
    - name: render-writer
      version: "1.3.0"
  rollout:
    canary_pct: [5, 25, 50, 100]
    step_interval_minutes: 30
    abort_criteria:
      error_rate_increase_pct: 0.5
      p95_latency_increase_pct: 50
  rollback:
    auto: true
    target: previous_version
  dependencies:
    - feature: tm-matching-v3
      min_version: "1.0.0"
```

### 8.2 升级流程

```
1. Developer 提交代码 → Git PR
        │
        ▼
2. CI 构建镜像 → push 到 Harbor
        │
        ▼
3. CI 生成/更新 Feature Bundle manifest → Git push
        │
        ▼
4. CI 验证：
   - 单元测试 / 契约测试 / 集成测试通过
   - 镜像签名验证
   - Schema 兼容性检查
   - 依赖版本检查
        │
        ▼
5. 自动部署到 STG（无需审批）
        │
        ▼
6. STG 验证（自动化）：
   - E2E 场景
   - 性能基线
   - 合规验证（敏感内容）
        │
        ▼
7. 提交 PRE 部署（需 platform_admin 审批）
        │
        ▼
8. PRE 部署（自动）
        │
        ▼
9. PRE 验证（手工 + 自动化）
        │
        ▼
10. 提交 PROD 部署（需 ≥ 1 platform_admin 审批）
        │
        ▼
11. PROD Canary（按 Feature Bundle rollout 规则）：
    5% (30min) → 25% (30min) → 50% (1h) → 100%
        │
        ▼
12. 每个 Canary 阶段监控：
    - 错误率 ↑ > 0.5% → 自动 abort
    - p95 延迟 ↑ > 50% → 自动 abort
    - 关键业务指标异常 → 告警（可手动 abort）
        │
        ▼
13. 全部通过 → 完成
    或 自动 abort → 回滚到 previous_version
```

### 8.3 Canary 部署

通过 **Envoy Gateway `HTTPRoute` 权重路由**实现：

```yaml
apiVersion: gateway.networking.k8s.io/v1
kind: HTTPRoute
metadata:
  name: translation-core-canary
spec:
  hostnames:
    - api.cats.internal
  rules:
    - matches:
        - path:
            type: PathPrefix
            value: /v1/translation-core
      backendRefs:
        - name: translation-core-stable
          port: 8080
          weight: 95  # 95% 流量到 stable
        - name: translation-core-canary
          port: 8080
          weight: 5   # 5% 流量到 canary
```

运维 UI 通过修改 HTTPRoute weight 推进 Canary 阶段。

### 8.4 数据库迁移（Expand-Contract）

**核心原则**：**永远不要做不可逆的破坏性迁移**，必须分三步：

```
① Expand（扩展）
   - 加新列/表，不动旧列
   - 应用同时写新+旧列
   - 兼容旧版本消费者

② Migrate（迁移）
   - 后台脚本把旧列数据迁移到新列
   - 应用读新列
   - 旧版本消费者仍可读旧列

③ Contract（收缩）
   - 删除旧列/表
   - 此时新版本消费者已就位
   - 必须等待所有旧版本下线
```

**示例**：将 `translation_memory.target_text TEXT` 拆为 `target_text_zh TEXT` + `target_text_en TEXT`

| 步骤 | 操作 | 部署要求 |
|---|---|---|
| ① Expand | `ALTER TABLE translation_memory ADD COLUMN target_text_zh TEXT;` | 任意版本可读 |
| ② Migrate | 后台 job 填充 target_text_zh | 部署 v2.0.0 应用读新列 |
| ③ Contract | `ALTER TABLE translation_memory DROP COLUMN target_text;` | 旧版本已全部下线 |

详见 `CATs_数据库设计书_v1.0` §15 迁移策略。

### 8.5 回滚策略

| 回滚触发 | 自动/手动 | 范围 | 时长 |
|---|---|---|---|
| Canary 阶段错误率 ↑ > 0.5% | 自动 | 当前 Feature Bundle | < 3min |
| Canary 阶段延迟恶化 > 50% | 自动 | 当前 Feature Bundle | < 3min |
| 生产关键告警 | 手动（platform_admin） | 全部或部分 | < 5min |
| 用户报告严重问题 | 手动 | 特定 Feature Bundle | < 10min |
| 安全漏洞 | 紧急（任何 platform_admin） | 受影响服务 | < 30min |

**回滚契约**：
- 仅回滚 Pod（不涉及 schema 变更）→ 任意时候可回滚
- 涉及 schema 变更的回滚：
  - Expand 阶段（①）→ 任意回滚
  - Migrate 阶段（②）→ 需先停止应用，再回滚 schema
  - Contract 阶段（③）→ 不可回滚 schema（已删除），需通过 forward-only 修复

---

## 9. API 设计规范

### 9.1 命名约定

| 项 | 规则 | 示例 |
|---|---|---|
| REST URL | 全小写、连字符、复数资源 | `/v1/translation-memories/{id}` |
| Query 参数 | 全小写、snake_case | `?source_lang=ja&target_lang=zh-CN` |
| Header | 全小写、连字符 | `authorization`、`x-cats-user-id` |
| JSON 字段 | snake_case | `{"user_id": "...", "target_lang": "..."}` |
| 事件 type | `cats.{domain}.{entity}.{event_type}.v{n}` | `cats.task.task.completed.v1` |
| gRPC package | `cats.{service}.v{n}` | `cats.translation.v1` |
| 环境变量 | SCREAMING_SNAKE | `CATS_DB_URL` |
| 日志字段 | snake_case | `{"trace_id": "...", "user_id": "..."}` |

### 9.2 版本管理

详见接口设计书 §1.5。

补充热插拔相关：
- **单一版本**：`/v1/...` 内可新增可选字段，非破坏性
- **双版本共存**：发布 `/v2/...` 后，`/v1/...` 至少保留 2 个发布周期
- **事件双 type**：发布 `task.completed.v2` 后，老消费者仍按 `task.completed.v1` 接收

### 9.3 兼容性原则

**向后兼容变更**（默认）：
- ✅ 新增可选字段
- ✅ 新增 enum 值
- ✅ 新增 endpoint
- ✅ 放宽请求约束（如允许更长的字符串）
- ✅ 严格化响应（新增字段，旧消费者忽略）

**破坏性变更**（需升 v）：
- ❌ 删除字段
- ❌ 改字段类型
- ❌ 改字段语义
- ❌ 改 endpoint 语义
- ❌ 减少 enum 值

### 9.4 错误码

详见接口设计书 §1.4 统一错误码（11 种）。

补充：
- 每个错误响应必须含 `trace_id`，便于追踪
- 错误响应含 `details` 字段（可选）描述具体错误

### 9.5 幂等性

- 所有 POST 写操作支持 `Idempotency-Key` Header（UUID）
- 服务端 24h 内对相同 Key 返回首次结果
- 服务端存储（Valkey）`idem:{key_hash}` 存首次结果

### 9.6 健康检查

每个服务必须实现 3 个端点（k8s liveness/readiness/startup）：

| 端点 | 用途 | 检查内容 | 失败行为 |
|---|---|---|---|
| `GET /health/live` | 存活 | 进程是否运行 | 重启 Pod |
| `GET /health/ready` | 就绪 | 依赖（DB/Kafka）是否就绪 | 从 Service Endpoints 移除 |
| `GET /health/startup` | 启动 | 启动是否完成 | 阻止流量 |

### 9.7 能力宣告

见 §6.2。每个服务必须实现 `GET /internal/v1/capabilities`。

### 9.8 追踪

所有请求/响应/事件必须含 OTel Trace Context：

- HTTP Header：`traceparent: 00-{trace_id}-{span_id}-{flags}`
- Kafka Header：`traceparent`（消息传递时透传）
- gRPC Metadata：`traceparent`

---

## 10. 存储过程设计规范

### 10.1 数据库隔离

详见数据库设计书 §1-§2。每服务独立逻辑库（`auth_db` / `user_db` / `project_db` / `task_db` / ...），跨服务**不建物理外键**，仅逻辑外键。

补充热插拔相关：
- 媒体处理服务**无独立库**（架构设计书 §4.1），全部经 `task-service` API 间接读写
- 逻辑外键需应用层保证一致性
- 跨服务数据访问通过事件而非直连

### 10.2 模式管理

每个逻辑库有独立的 migration 仓库（`migrations/` 目录）。

| 服务实现语言 | 迁移工具 | 版本号策略 |
|---|---|---|
| Rust | `sqlx-migrate` | 单调递增的 8 位时间戳（`20260819000001`） |
| Python | `alembic` | 同上 |
| Node.js/TS | `Prisma Migrate` | 同上 |

**模式版本管理**：
- 所有 migration 文件按版本号排序
- 每个 migration 含 `forward.sql`（执行）和 `rollback.sql`（回滚）脚本
- CI 中跑 `sqlx migrate run --dry-run` 验证（数据库设计书 §3）

### 10.3 Outbox 模式

详见架构设计书 §7 与数据库设计书 §4 各逻辑库的 outbox_event 表。

**强制规则**：
- 业务写 + Outbox 写必须在**同一事务**
- 写 Outbox 表前不写 Kafka（避免双写不一致）
- Debezium 监听 outbox 表 → 发布到 Kafka
- 事务结束前 Outbox 行已可见，事务结束后 CDC 转发

**代码模板**（以 `task-service` 为例）：

```rust
async fn create_task(&self, input: CreateTaskInput) -> Result<Task> {
    let mut tx = self.db.begin().await?;
    let task = sqlx::query!("INSERT INTO tasks ...").fetch_one(&mut tx).await?;
    sqlx::query!("INSERT INTO task_events_outbox (event_type, aggregate_id, payload, schema_version) VALUES ($1, $2, $3, $4)",
        "task.created", &task.id, json!({...}), 1
    ).execute(&mut tx).await?;
    tx.commit().await?;  // 业务表 + Outbox 同事务提交
    Ok(task.into())
}
```

### 10.4 兼容性迁移（Expand-Contract）

见 §8.4 完整说明。

**强制规则**：
- 所有破坏性 schema 变更必须分三步
- 迁移期间 schema 版本号必须递增
- 任何 schema 变更必须先经 DBA/platform_admin 审批
- 迁移脚本必须有 reverse 脚本（即便 reverse 是空的）

### 10.5 跨服务数据访问

**禁止**：
- ❌ 服务 A 直连服务 B 的数据库
- ❌ 跨服务物理外键
- ❌ 跨服务事务（XA/Saga 等）

**允许**：
- ✅ 服务 B 暴露 API，服务 A 通过 API 访问
- ✅ 服务 A 消费服务 B 的事件做派生数据
- ✅ 跨服务数据通过 Kafka 事件传播

**例外**：
- `report-service` 可跨库只读（`svc_report_ro` 账号，仅 `task_db`/`project_db`）
- `audit-service` 可消费各服务事件（仅 `audit.events`）

### 10.6 历史数据归档

| 库 | 表 | 归档策略 |
|---|---|---|
| task_db | `tasks`（completed > 90 天） | 归档到 `tasks_archive` |
| task_db | `asr_transcripts`（> 180 天） | 归档到 `asr_transcripts_archive` |
| notification_db | `notifications`（read + > 30 天） | 物理删除 |
| audit_db | `audit_events`（> 365 天） | 归档到冷存储 |

归档通过 worker-service 的 Cron 任务执行。

### 10.7 备份恢复

- **PITR**（Point-In-Time Recovery）：CloudNativePG 启用，每 5 分钟一次 WAL 归档
- **全量备份**：每日凌晨 02:00，`pgBackRest` 备份到对象存储
- **保留**：每日备份保留 30 天，周备份保留 12 月
- **DR 演练**：每季度 1 次

---

## 11. 风险与缓解

| 编号 | 风险 | 概率 | 影响 | 缓解措施 |
|---|---|---|---|---|
| OPR-01 | 镜像不可变未严格执行，CI 重试覆盖了 tag | 中 | 高 | Harbor 强制不可变标签；CI 重试不重 tag；生产禁止 latest 标签 |
| OPR-02 | 灰度回滚时长 > 5min | 低 | 高 | 预先用 `k6` 压测回滚流程；Argo CD 自动化；演练 |
| OPR-03 | Kafka 中心化后，Kafka 故障影响所有跨服务协同 | 中 | 高 | Kafka 3 broker HA + RF=3；监控 broker 健康；DR 演练 |
| OPR-04 | DLQ 累积未及时处理 | 中 | 中 | DLQ 告警阈值 + 运维 UI DLQ 管理；SLA：DLQ 消息 24h 内处理 |
| OPR-05 | 事件 schema 演进破坏消费者 | 中 | 高 | Schema Registry + CI 兼容性校验；破坏性变更强制升 v；新版本共期 ≥ 2 发布周期 |
| OPR-06 | 能力宣告不一致（服务说支持但实际不支持） | 中 | 中 | capabilities 端点 CI 验证（与代码断言一致）；金丝雀用例覆盖 |
| OPR-07 | 特性开关误操作影响生产 | 中 | 高 | 影响范围 > 10% 用户需审批；开关变更全审计；可一键回滚 |
| OPR-08 | 插件注入失败 | 中 | 中 | 启动时强校验必需插件；DI 失败时启动失败；插件热加载有 fallback |
| OPR-09 | 可选服务缺失时调用方未降级 | 中 | 中 | 启动时依赖检查 + 降级路径测试；运维 UI 标记状态 |
| OPR-10 | 运维 UI 越权 | 中 | 高 | RBAC 强制 + 中间件 + BFF 双重；越权操作直接拒绝；审计 |
| OPR-11 | Feature Bundle 依赖冲突 | 中 | 中 | manifest 声明依赖 + CI 校验；版本约束 |
| OPR-12 | 数据库迁移回滚失败 | 低 | 高 | Expand-Contract 强制；每步前先备份；reverse 脚本验证 |
| OPR-13 | Canary 自动 abort 误判 | 中 | 中 | 多指标综合判定（错误率 + 延迟 + 业务指标）；保守阈值；可手动覆盖 |
| OPR-14 | 审计日志被篡改 | 低 | 高 | 审计事件 append-only（PostgreSQL trigger 禁止 UPDATE/DELETE）；定时异地归档 |
| OPR-15 | 运维 UI 单点故障 | 中 | 中 | 3 副本 + 跨 AZ；只读操作可通过 CLI 替代；紧急情况手动 kubectl |

---

## 12. 实施路线图

### 12.1 阶段一（M1 MVP 闭环，2026-09 ~ 2026-10）

| 项目 | 内容 |
|---|---|
| 原子化部署 | 镜像命名规范落地；GitOps 流程（Argo CD）；STG 自动同步 |
| 中心事件 | 单一 Kafka 集群 + DLQ 基础；事件 schema v1 注册 |
| App 集群 | K3s 3 控制面 + 工作节点池标签；HPA（基于 CPU） |
| 可热插拔 | 服务能力宣告端点（最小集）；媒体处理插件 trait 实现 |
| 运维界面 | Next.js 后台新增模块：服务总览 + 部署管理（仅 STG）+ 审计日志 |
| 单功能升级 | 单服务级别（无 Feature Bundle）；STG 自动，PROD 审批 |
| API 规范 | 接口设计书 §1.5 全部落地；capabilities 端点最小集 |
| 存储规范 | Outbox + Expand-Contract 落地；migration 工具选型完成 |

### 12.2 阶段二（M2 数据底座，2026-11 ~ 2027-01）

| 项目 | 内容 |
|---|---|
| 原子化部署 | Feature Bundle manifest 格式；PROD Canary 自动化 |
| 中心事件 | 事件回放 API；DLQ 运维 UI 完善；Schema Registry 落地 |
| App 集群 | HPA 基于 Kafka consumer lag（KEDA）；PDB 全部服务 |
| 可热插拔 | OpenFeature 落地；运行时配置热更新（ConfigMap + Reloader） |
| 运维界面 | 事件总线监视模块；特性开关管理模块；数据库迁移模块 |
| 单功能升级 | 跨服务 Feature Bundle 试点（PDF 翻译 V2） |
| API 规范 | 完整 capabilities 端点；动态服务发现 |
| 存储规范 | 历史数据归档 job；备份恢复流程落地 |

### 12.3 阶段三（M3-M5 文档/工作流/增强能力，2027-02 ~ 2027-Q4）

| 项目 | 内容 |
|---|---|
| 原子化部署 | 跨集群多环境；Federation 能力评估 |
| 中心事件 | 事件版本化 2 套并存；事件流分析工具 |
| App 集群 | Cluster Autoscaler 弹性节点；跨节点池调度优化 |
| 可热插拔 | 第三方插件注册机制；插件市场（内部） |
| 运维界面 | 完整运维 UI 全部模块上线；DR 演练模块 |
| 单功能升级 | 完整 Feature Bundle 灰度 + 自动 abort + 一键回滚 |
| API 规范 | gRPC streaming 增强；multipart upload |
| 存储规范 | PITR 落地；DR 演练每季度 |

---

> **本书的下一步**: 评审通过后，① 与现有架构设计书/接口/数据库/模块设计书交叉引用补全；② Next.js 后台启动运维 UI 模块开发（M1 仅服务总览 + 审计）；③ 服务镜像命名规范在 CI 脚本中落地。
