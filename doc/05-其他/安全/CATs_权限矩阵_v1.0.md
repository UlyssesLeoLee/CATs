# CATs 权限矩阵 v1.0

**5 角色 × 16 服务 × 8 逻辑库 × 30+ Kafka topic × 5 平台系统的 RBAC 三元组矩阵**

> **文档编号**: CATs-SEC-013
> **版本**: v1.0（2026-08-27 评审前草稿）
> **创建日**: 2026-08-27
> **修订日**: 2026-08-27
> **作者**: 架构师 + SRE Lead（worker 代签 per DEC-008 文档代签规则）
> **状态**: 草稿待评审（D-Day 升 B0.0）
> **密级**: 仅社内
> **上游文档**:
> - [CATs_安全要件定义书 v1.0](../../安全/CATs_安全要件定义书_v1.0.md)
> - [CATs_微服务架构设计书 v1.1 §4.1 / §5.1 / §5.2 / §6](../../02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md)
> - [CATs_实施前QA 登记册 v1.3 §2.3 OI-1](../CATs_实施前QA登记册_v1.3.md)
> - [CATs_Baseline 一览 v1.0 §6](../../管理/CATs_Baseline一览_v1.0.md)
> **下游文档**:
> - CATs_类图 v1.0 §5 域模型聚合（RBAC 实体）
> - CATs_接口设计书 v2.0 §3 gRPC AuthCheck
> - M1-Sprint 1 auth-service 实施任务

### 审批栏

| 角色 | 姓名 | 审批 | 签字 | 日期 |
|---|---|---|---|---|
| 起草 | 架构师 | ☑ | Mavis 代签 | 2026-08-27 |
| 评审 | SRE Lead | ☐ | — | — |
| 评审 | DBA | ☐ | — | — |
| 评审 | QA 负责人 | ☐ | — | — |
| 批准 | Sponsor | ☐ | — | D-Day 现场 |
| 批准 | 客户代表 | ☐ | — | D-Day 现场 |

### 修订履历

| 版本 | 日期 | 修订人 | 修订内容 |
|---|---|---|---|
| **v1.0** | **2026-08-27** | **架构师 + SRE Lead（worker 代签 per DEC-008）** | **首版定稿**：5 角色 × 16 服务 × 8 库 × 30+ topic × 5 平台 RBAC 三元组矩阵（per 实施前QA v1.3 §2.3 OI-1 + 安全要件 §3 + 微服务架构设计 §4-§6） |

---

## 1. 目的

本矩阵定义 CATs 系统的**完整 RBAC 权限模型**——5 类角色对 16 个微服务 API + 8 个逻辑库表 + 30+ Kafka topic + 5 个平台系统的**访问权限三元组**（角色 × 资源 × 操作）。

目标:

- **避免兼任**: 架构师兼任架构域 Lead / SRE 兼任 admin 域 Lead（per 2026-08-21 RGS 5 域独立 Lead 决议 + 5 域分配原则）
- **数据边界**: 服务 / 数据库 / 消息 / 平台 4 维度权限独立（per 微服务架构设计 §5.2 账号隔离原则）
- **审计可证**: 每条权限决策可追溯到 OI / ADR / 评审纪要（per 安全要件 §3 + 等保 2.0 三级）

## 2. 角色定义

### 2.1 super-admin (1 个)

- **持有方**: Sponsor（一人公司情况 = Ulysses 本人）
- **作用域**: 跨域超管, 包括 K3s / CNPG / Kafka 集群 root + GitOps 全部权限
- **MFA 强制**: 必须 (per 安全要件 §3.2)
- **审计**: 所有操作 100% 强制审计（per 实施前QA v1.3 §2.3 OI-1 + 等保 2.0 三级）
- **不能由 agent 代签**（即使 2026-08-26 08:40 JST 代签新规则也豁免此角色——Mavis 不代表 Sponsor）

### 2.2 SRE Lead (1 个)

- **持有方**: 平台域 Lead（独立任命, 不兼任任何业务域 Lead）
- **作用域**: K3s 集群 / CNPG / Kafka / Harbor / ArgoCD / Prometheus / Grafana 全部平台系统
- **不涉及**: 任何业务域（identity / project / media / translate）的数据操作
- **审计**: 平台操作全量审计

### 2.3 domain-lead (5 个)

- **持有方**: 5 业务域各 1 Lead，**不可兼任**（per 2026-08-21 RGS 5 域独立 Lead 决议 + 5 域分配原则）:
  - **identity-domain-lead** (auth / user / audit / notification)
  - **project-domain-lead** (project / task / worker / report)
  - **media-domain-lead** (ingestion / asr / ocr / subtitle / office-converter / render-writer)
  - **translate-domain-lead** (translation-core / llm-gateway)
  - **platform-domain-lead** (cats-bff + Envoy Gateway + 平台组件运维，不与 SRE Lead 兼任)
- **作用域**: 仅本域服务的代码 / 部署 / 配置变更; 跨域只读
- **MFA**: 推荐
- **审计**: 域内全量 + 跨域访问增量审计

### 2.4 service-user (16 个, 1 服务 1 user)

- **持有方**: 各 service 在 K8s pod 内以独立 user 身份运行
- **作用域**: 仅本服务的 SQL + Kafka topic 范围
- **MFA**: N/A (机器身份)
- **审计**: 服务间调用全量审计

### 2.5 audit-reader (1 个)

- **持有方**: 合规 / 审计 / DBA (合规读权限)
- **作用域**: 只读所有 8 逻辑库 + Kafka audit topic + 平台系统监控数据
- **MFA**: 必须
- **审计**: 读操作全量审计 (per QA-048 审计不可篡改)
- **不涉及**: 任何写操作 / 部署 / 配置

## 3. 资源清单

### 3.1 16 微服务 (per 微服务架构设计 §4.1)

| 编号 | 服务 | 库 | 阶段 | Domain |
|---|---|---|---|---|
| S01 | auth-service | auth_db | MVP | identity |
| S02 | user-service | user_db | MVP | identity |
| S03 | project-service | project_db | MVP | project |
| S04 | task-service | task_db | MVP | project |
| S05 | file-service | file_db | MVP | project |
| S06 | notification-service | notification_db | MVP | project |
| S07 | report-service | report_db | MVP | project |
| S08 | audit-service | audit_db | MVP | identity |
| S09 | worker-service | (无, 消费 task_db) | MVP | project |
| S10 | translation-core | project_db (TM/术语) | MVP | translate |
| S11 | ingestion-service | task_db | 阶段二 | media |
| S12 | asr-service | (无, 写回 file-service) | 阶段二 | media |
| S13 | ocr-service | (无) | 阶段二 | media |
| S14 | subtitle-service | (无) | 阶段二 | media |
| S15 | office-converter-service | (无) | 阶段二 | media |
| S16 | render-writer-service | (无) | 阶段二 | media |
| S17 | cats-bff | (无) | MVP | platform |

### 3.2 8 逻辑库 (per 微服务架构设计 §5.1 + Baseline §5.1)

| 编号 | 库 | owner | 关联服务 |
|---|---|---|---|
| D01 | auth_db | postgres | S01 auth |
| D02 | user_db | postgres | S02 user |
| D03 | project_db | postgres | S03 project, S10 translation-core |
| D04 | task_db | postgres | S04 task, S11 ingestion |
| D05 | file_db | postgres | S05 file |
| D06 | notification_db | postgres | S06 notification |
| D07 | report_db | postgres | S07 report |
| D08 | audit_db | postgres | S08 audit |

### 3.3 Kafka topic (per 微服务架构设计 §6.2 完整 11 类)

| 编号 | Topic | Producer | Consumer |
|---|---|---|---|
| T01 | user.events | S02 | S08, S06 |
| T02 | project.events | S03 | S10, S06, S07 |
| T03 | task.events | S04 | S06, S07, S08 |
| T04 | task.media.asr.requested | S04 | S12 |
| T05 | task.media.asr.completed | S12 | S04 |
| T06 | task.media.ocr.requested | S04 | S13 |
| T07 | task.media.ocr.completed | S13 | S04 |
| T08 | task.media.subtitle.requested | S04 | S14 |
| T09 | task.media.subtitle.completed | S14 | S04 |
| T10 | task.media.office.requested | S04 | S15 |
| T11 | task.media.office.completed | S15 | S04 |
| T12 | task.media.render.requested | S04 | S16 |
| T13 | task.media.render.completed | S16 | S04 |
| T14 | audit.events | S01-S17 全员 | S08 |
| T15 | notification.events | S01-S17 全员 | S06 |
| T16-30 | retry.1/.2/.3 + .dlq | 11 类各 4 | S01-S17 |

### 3.4 5 平台系统

| 编号 | 平台 | 用途 |
|---|---|---|
| P01 | K3s 集群 | 容器编排 |
| P02 | CloudNativePG (CNPG) | PG 18.6 + pgvector 0.8.6 运维 |
| P03 | Kafka (KRaft) | 事件总线 |
| P04 | Harbor | 镜像仓库 |
| P05 | ArgoCD | GitOps 持续部署 |

## 4. 操作类型

| 缩写 | 全称 | 含义 |
|---|---|---|
| C | Create | 新建资源 |
| R | Read | 读资源 |
| U | Update | 修改资源 |
| D | Delete | 删除资源 (审计强化 + 二级审批) |
| X | Execute | 触发操作 (部署 / 重启 / 触发任务) |
| A | Admin | 后台运维 (配置 / 监控 / 日志) |

## 5. RBAC 三元组矩阵

### 5.1 角色 × 16 微服务 API (REST/gRPC 端点)

权限标记: Y=允许, N=禁止, *M=需 MFA 二级验证, *A=全量审计

| 角色 | S01 auth | S02 user | S03 project | S04 task | S05 file | S06 notif | S07 report | S08 audit |
|---|---|---|---|---|---|---|---|---|
| super-admin | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A |
| SRE Lead | N | N | N | N | N | N | N | R*A |
| identity-lead | Y*M | Y*M | R | R | N | R | R | Y |
| project-lead | R | R | Y*M | Y*M | R | R | Y | R |
| media-lead | N | N | R | Y | R | N | N | R |
| translate-lead | N | N | Y | Y | N | N | R | R |
| platform-lead | R | R | R | R | R | R | R | R |
| service-user | 仅本服务 S01 Y* | 仅本服务 S02 Y* | ... | ... | ... | ... | ... | 仅本服务 S08 R* |
| audit-reader | R*A | R*A | R*A | R*A | R*A | R*A | R*A | R*A |

(注: service-user 严格 = 仅本服务的 SXX Y*; 其他服务 SYY=N, 不在表中重复列)

| 角色 | S09 worker | S10 trans | S11 ingest | S12 asr | S13 ocr | S14 sub | S15 office | S16 render | S17 bff |
|---|---|---|---|---|---|---|---|---|---|
| super-admin | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A |
| SRE Lead | N | N | N | N | N | N | N | N | N |
| identity-lead | N | N | N | N | N | N | N | N | N |
| project-lead | Y | R | Y | R | R | R | R | R | R |
| media-lead | N | N | Y*M | Y*M | Y*M | Y*M | Y*M | Y*M | R |
| translate-lead | N | Y*M | R | N | N | R | N | R | R |
| platform-lead | R | R | R | R | R | R | R | R | Y*M |
| service-user | 仅 S09 Y* | 仅 S10 Y* | ... | ... | ... | ... | ... | ... | 仅 S17 Y* |
| audit-reader | R*A | R*A | R*A | R*A | R*A | R*A | R*A | R*A | R*A |

### 5.2 角色 × 8 逻辑库

操作标记: S=SELECT, I=INSERT, U=UPDATE, D=DELETE (审计强化 + 二次确认), A=DDL/ADMIN

| 角色 | D01 auth | D02 user | D03 project | D04 task | D05 file | D06 notif | D07 report | D08 audit |
|---|---|---|---|---|---|---|---|---|
| super-admin | SIUD*A | SIUD*A | SIUD*A | SIUD*A | SIUD*A | SIUD*A | SIUD*A | SIUD*A |
| SRE Lead | A | A | A | A | A | A | A | S*A |
| identity-lead | S*IU | S*IU | S | S | N | S | S | SIU |
| project-lead | S | S | SIU*M | SIU*M | S | S | SIU*M | S |
| media-lead | N | N | S | SIU | S | N | N | S |
| translate-lead | N | N | SIU | SIU | N | N | S | S |
| platform-lead | S | S | S | S | S | S | S | S |
| svc_auth (S01) | SIU | N | N | N | N | N | N | N |
| svc_user (S02) | N | SIU | N | N | N | N | N | N |
| svc_project (S03,S10) | N | N | SIU | N | N | N | N | N |
| svc_task (S04,S11) | N | N | N | SIU | N | N | N | N |
| svc_file (S05) | N | N | N | N | SIU | N | N | N |
| svc_notify (S06) | N | N | N | N | N | SIU | N | N |
| svc_report (S07) | N | N | N | N | N | N | SIU | N |
| svc_audit (S08) | N | N | N | N | N | N | N | SIU |
| audit-reader | S*A | S*A | S*A | S*A | S*A | S*A | S*A | S*A |

### 5.3 角色 × 30 Kafka topic

操作标记: P=PRODUCE, C=CONSUME, A=ADMIN (创建 topic / 修改 retention / 配 ACL)

| 角色 | T01-T03 user/project/task | T04-T13 media (10) | T14 audit | T15 notif | T16-T30 retry+dlq (15) |
|---|---|---|---|---|---|
| super-admin | PCA*A | PCA*A | PCA*A | PCA*A | PCA*A |
| SRE Lead | A | A | A | A | A |
| identity-lead | PC on T01,T03 | N | PC | PC | N |
| project-lead | PC on T01-T03 | PC on T04-T13 (T04-T13 全) | PC | PC | PC |
| media-lead | N | PC on T04-T13 | PC | N | PC on T04-T13 retry+dlq |
| translate-lead | PC on T02 | PC on T08,T09,T12,T13 | PC | N | PC on T08-T13 retry+dlq |
| platform-lead | PC | PC | PC | PC | PC |
| svc_xxx (16 服务) | 仅本服务范围 | 仅本服务范围 | P (全员) | P (全员) | 仅本服务范围 |
| audit-reader | C*A | C*A | C*A | C*A | C*A |

(注: 11 类媒体 topic 各有 retry.1/2/3 + dlq = 4 个派生 = 11×4=44, 但 §6.2 标 11 类主 topic + 派生; 简化标 15 个)

### 5.4 角色 × 5 平台系统

操作标记: A=ADMIN (完全控制), M=MODIFY (改配置), R=READ (查), N=NONE

| 角色 | P01 K3s | P02 CNPG | P03 Kafka | P04 Harbor | P05 ArgoCD |
|---|---|---|---|---|---|
| super-admin | AMRN*A | AMRN*A | AMRN*A | AMRN*A | AMRN*A |
| SRE Lead | AMR | AMR | AMR | AMR | AMR |
| identity-lead | N | R (read-only) | N | N | R (看本域服务 deploy) |
| project-lead | N | R | N | R (本域镜像) | R (本域) |
| media-lead | N | R | N | R (本域镜像) | R (本域) |
| translate-lead | N | R | N | R (本域镜像) | R (本域) |
| platform-lead | R (read) | R | R | R | MR (改本域) |
| svc_xxx | N | 仅本服务 DB | 仅本服务 topic | N | N |
| audit-reader | R*A | R*A | R*A | R*A | R*A |

## 6. 跨服务鉴权 (Service-to-Service Auth)

### 6.1 gRPC AuthCheck (per 接口设计书 §3 AuthCheck service)

- **位置**: 每个 service 在接受其他 service 调用前, 调 auth-service 的 `gRPC AuthCheck(Claims)`
- **流程**:
  1. 客户端 service 持短期 JWT (15 分钟 TTL)
  2. JWT 携带 `sub` (user_id) + `domain` + `service` claims
  3. 接收 service 调 auth-service 验证 JWT 签名 + Claims 完整性
  4. auth-service 查 Valkey 缓存 (TTL 60s) 决定是否走 DB 校验
  5. 返回 Allow / Deny / QuotaExceeded 三态

### 6.2 mTLS (per 接口设计书 §3.7)

- 内部 service-to-service gRPC 走 mTLS (CA: per 实施前QA QA-061)
- 证书从 K3s cert-manager 内部 CA 签发
- 每 30 天自动轮转 (per QA-006)

### 6.3 短期 JWT 透传

- 用户登录 → auth-service 发 access_token (1h) + refresh_token (24h)
- BFF / 其他 service 不重发 token, 透传用户原 token
- Token 失效 → 用 refresh_token 静默续期 (per 接口设计书 §3.9 OAuth refresh flow)

## 7. 实施步骤 (M1-Sprint 1 落地 per OI-1)

| 步骤 | 任务 | 估时 | 责任 | 完成判据 |
|---|---|---|---|---|
| 1 | auth-service 增 AuthCheck gRPC service | 2d | identity-lead + Rust Lead | gRPC unit test pass |
| 2 | 8 逻辑库按 §5.2 grant | 0.5d | DBA | sqlx migration 跑通 |
| 3 | 30 Kafka topic ACL 配置 | 1d | platform-lead + SRE Lead | kafka-acl.sh dry-run 通过 |
| 4 | RBAC roles 表 + Casbin 模型加载 | 1d | identity-lead | cat 决策矩阵可读 |
| 5 | audit-reader 角色 / 凭据 | 0.5d | DBA + 合规 | audit-reader 可连 8 库只读 |
| 6 | 端到端鉴权测试 (per 服务 gRPC) | 1d | QA | authz-test suite 全 pass |
| 7 | 评审会 OI-1 签字归档 (D-Day) | 0d | Sponsor + 客户代表 | RBAC 矩阵 v1.0 → v1.1 评审通过 |

## 8. 已知缺口 + 关联文档

### 8.1 已知缺口 (M1 实战后补)

1. **CASL/Casbin 策略引擎**: M1-Sprint 1 决定用 Casbin (per 安全要件 §3 推荐) 还是自实现 — 暂占位
2. **多租户隔离**: 现状 1 租户 (局域网 SaaS); 未来多 org 需在 Claims 加 `org_id` 并在 §5.2 增 `org_id` 谓词
3. **跨域访问 audit log 增量**: §5.2/5.3 中标 R*A 的, 需 S08 audit-service 落库 (per OI-048 不可篡改)

### 8.2 关联文档

- [CATs_安全要件定义书 v1.0 §3](../../安全/CATs_安全要件定义书_v1.0.md) - 安全等级 + RBAC 原则
- [CATs_微服务架构设计书 v1.1 §4.1 / §5.1 / §5.2 / §6](../../02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md) - 资源清单源
- [CATs_实施前QA 登记册 v1.3 §2.3 OI-1](../CATs_实施前QA登记册_v1.3.md) - RBAC 决议
- [CATs_类图 v1.0 §5 域模型聚合](../../03-详细设计/类图/CATs_类图_v1.0.md) - RBAC 实体建模
- [CATs_接口设计书 v2.0 §3.7 / §3.9](../../03-详细设计/接口设计/CATs_接口设计书_v2.0.md) - gRPC AuthCheck + OAuth flow
- [CATs_Baseline 一览 v1.0 §6](../../管理/CATs_Baseline一览_v1.0.md) - 待基线化清单
- [CATs_数据库设计书 v2.0 §6.2](../../03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md) - 8 逻辑库 schema
- [CATs_技术基线 v1.0 §3.3 / §8 OI-3 + OI-4 完成](../../02-基础设计/技术选型/CATs_技术基线_v1.0.md) - M1-S0 已具备

---

**文档结束 (v1.0, 2026-08-27 草稿待 D-Day 评审)**
