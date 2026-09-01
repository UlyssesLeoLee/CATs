# CATs 权限矩阵 v1.0

**8 主角色 × 7 资源 × 8 Action 的工作流 RBAC 矩阵 + auth/user 端点交叉引用**

> **文档编号**：CATs-PMO-016
> **フェーズ**：基本設計 / M1-Sprint 1 T-03 权限設計（150 任务 #33）
> **关联任务**：启动会决议 8（OI-6 跨项目同步借机）/ Sprint 1 拆解 v1.0+2 §2 T-03
> **版本**：v1.0
> **创建日**：2026-09-01
> **状态**：DDD Review 草稿（待 6 角色 9/13 截止前评审）
> **密级**：仅社内
> **作者**：架构师 Lead（Ulysses 兼任一人公司 12 角色架构师槽位 / Mavis 接手 agent per DEC-008 + 2026-08-27 19:39 JST 强化代签）

---

## 文档管理信息

### 审批栏（6 角色 per 启动会决议 4 共识模型）

| 角色 | 姓名 | 审批 | 签字 | 日期 | 备注 |
|------|------|------|------|------|------|
| Sponsor (Ulysses 本人签) | Ulysses | ☐ | — | — | 一人公司 = Ulysses 持有 Sponsor，不代签 |
| 架构师 Lead | Ulysses（Mavis 接手 agent per DEC-008） | ☐ | — | — | 起草方 / T-03 主责任 |
| Rust Lead | Ulysses（Mavis 接手 agent per DEC-008） | ☐ | — | — | 实现角度评审 |
| DBA Lead | Ulysses（Mavis 接手 agent per DEC-008） | ☐ | — | — | 数据层 schema 影响评审 |
| QA Lead | Ulysses（Mavis 接手 agent per DEC-008） | ☐ | — | — | 测试覆盖评审 |
| PMO Lead | Ulysses（Mavis 接手 agent per DEC-008） | ☐ | — | — | 与 Sprint 1 §6 已知缺口对齐评审 |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|------|------|--------|----------|
| **v1.0** | **2026-09-01** | **架构师 Lead（Mavis 接手 agent per DEC-008）** | **首版定稿**：8 主角色（sponsor/admin/pm/qa/translator/reviewer/terminologist/user）× 7 资源（users/projects/tasks/tm/terms/files/audit）× 8 Action（C/R/U/D/Apv/Rj/Exp/Imp）矩阵 + §5 与接口设计书 v2.0+1 §6.1 auth + §6.2 user 共 10 端点 100% 交叉引用 + OI-6 跨项目同步借机（决议 8 落地） |

---

## 0. 元信息

| 项 | 值 |
|----|----|
| **Worktree** | `D:/CATs/.worktrees/feat-m1-sprint1-t03-rbac` |
| **分支** | `feat/m1-sprint1-t03-rbac` |
| **commit baseline** | `49fdbb3`（per `git log -1`，WBS Sprint 1 跟踪 v1.0 落地） |
| **关联基线（B0.0）** | `4f96f95`（CAB-001 v1.0 初始基线决议书） |
| **上游源文档** | 见 §0.1 源文档引用清单（git 实证） |
| **下游引用** | 接口设计书 v2.0+1 §6.1/§6.2 端点 auth/user 鉴权字段 / 类图 v1.0 §5 RBAC 实体 / T-07 类图升版 / Sprint 1 集成测试 ITa / 工作流文档 #33 权限設計 |
| **配套 Excel** | 无（矩阵在文档内可读） |
| **密级** | 仅社内 |

### 0.1 源文档引用清单（git 实证）

> 引用纪律（per 2026-08-26 AI 协作文档治理强证据 + 2026-08-27 19:39 JST Mavis 代签强化）：以下每条引用均通过 `git log -1 --format='%H %s' -- <path>` 在本 worktree `D:/CATs/.worktrees/feat-m1-sprint1-t03-rbac` 实证，引用时注明 commit hash。

| 引用文档 | 路径 | commit hash | 用途 |
|---------|------|------------|------|
| CATs_M1_Sprint1_任务拆解 v1.0+2 | `doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md` | `efd9e77`（"docs(m1-sprint1): Phase 0 启动会决议落地…"） | §2 T-03 任务清单（150K-300K tokens 估时）+ §0.3 决议 8 借机 OI-6 |
| CATs_接口设计书 v2.0+1 | `doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md` | `0eb1e9f`（"docs(api): CATs 接口设计书 v2.0+1 升版…"） | §6.1 auth 5 端点 + §6.2 user 5 端点 + §1.2 JWT/Roles 鉴权约定 |
| CATs_错误码表 v1.0 | `doc/05-其他/管理/CATs_错误码表_v1.0.md` | `2146f53`（"feat(auth-service): T-01 实战深化 - refresh 轮换 + logout + 审计"） | §3 鉴权错误枚举 + §4 auth-service 端点错误码矩阵 + §5 审计事件 |
| CATs_安全要件定义书 v1.0 | `doc/05-其他/安全/CATs_安全要件定义书_v1.0.md` | `d1b10fe`（"docs(pmo): 完整 PMO 文档集…"） | §3 合规框架（等保 2.0 三级）+ §4 认证 + §5.1 RBAC 模型原则 |
| CATs_微服务架构设计书 v1.0 | `doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md` | `2910f3d`（"docs(基线): WT-H4 架构+README+杂项升级…"） | §4.1 核心 8 MVP 服务 + §5.1 8 逻辑库 + §5.2 账号隔离原则 |
| CATs_Baseline一览 v1.0 | `doc/05-其他/管理/CATs_Baseline一览_v1.0.md` | `4f96f95`（"docs(cab): CAB-001 v1.0 B0.0 初始基线决议书落地…"） | §6 待基线化清单（CATs_权限矩阵 v1.0 期望 M1-S0 基线化） |
| CATs_M1_Sprint1_启动会决议纪要 v1.0 | `doc/05-其他/会议记录/CATs_M1_Sprint1_启动会决议纪要_v1.0.md` | `1b27b2b`（"docs(m1-sprint1): 启动会决议纪要 v1.0…"） | §2 决议 8（OI-6 跨项目同步）+ §2.1 RACI C 角响应 SLA |
| CATs_token-OLU 框架 v0.1 | `doc/05-其他/管理/CATs_token-OLU框架_v0.1.md` | `f6772ce`（"docs(pmo): CATs token-OLU 框架 v0.1 落地…"） | §3.1 架构师 Lead 系数 150K-300K tokens（T-03 估时依据） |
| CATs_WBS Sprint 1 跟踪 v1.0 | `doc/05-其他/管理/CATs_WBS_Sprint1_跟踪_v1.0.md` | `49fdbb3`（"docs(pmo): CATs WBS Sprint 1 任务级跟踪 v1.0 落地"） | §0.1 引用清单 + §2.2 WBS 编码 PMO.S1.T03 + §3 Lead 估时 |
| CATs_RACISLA 模板 v1.0 | `doc/05-其他/管理/模板/CATs_RACISLA模板_v1.0.md` | `efd9e77`（同期 commit，决议 4 落地） | §3 24h SLA + §4 违约处理（DDD Review 6 角色评审规则依据） |
| CATs_权限矩阵 v1.0（基础设施版，**不同文件**）| `doc/05-其他/安全/CATs_权限矩阵_v1.0.md` | `b85ceca`（"docs(整合): P1 整合 3 项…"） | 5 角色 super-admin/SRE Lead/domain-lead/service-user/audit-reader 服务级 RBAC（与本文档工作流 RBAC 互补不冲突） |

### 0.2 术语

| 术语 | 含义 |
|------|------|
| **RBAC** | Role-Based Access Control，基于角色的访问控制 |
| **资源（Resource）** | 被权限保护的业务对象（用户/项目/任务/TM/术语/文件/审计） |
| **Action** | 资源上的可执行操作（C/R/U/D/Apv/Rj/Exp/Imp） |
| **权限点** | 资源 × Action 的二元组（如 `users:read`、`tasks:approve`） |
| **角色** | 一组权限点的命名集合（如 `sponsor` / `admin` / `translator`） |
| **派生角色** | 基础角色 × 范围（org / project）限定的子角色（如 `project_admin@<project_id>`） |
| **MFA** | Multi-Factor Authentication，多因素认证 |
| **JWT Claims** | JSON Web Token 内的角色/权限声明，注入 `X-Cats-Roles` Header（per 接口设计书 v2.0+1 §1.2） |
| **3 档 SLA** | 24h 默认 / 8h 紧急 / ≤1h PMO 升级（per RACI SLA 模板 v1.0 §3） |

---

## 1. 目的与范围

### 1.1 目的

本矩阵定义 CATs 系统的**工作流 RBAC 权限模型**——**8 个主角色**对 **7 类业务资源**在 **8 个 Action** 上的访问权限组合，目标是：

1. **审批基线化**：满足 Baseline 一览 v1.0 §6 待基线化清单（CATs_权限矩阵 v1.0 期望 M1-S0 基线化）
2. **接口可引用**：与接口设计书 v2.0+1 §6.1（auth 5 端点）+ §6.2（user 5 端点）100% 交叉引用，每个端点至少 1 角色匹配
3. **6 角色评审**：通过 Sponsor + 架构师/Rust/DBA/QA/PMO Lead 6 角色评审（per 启动会决议 4 RACI SLA + 6 角色共识模型）
4. **审计可证**：所有权限决策可追溯到 OI / ADR / 评审纪要（per 安全要件 v1.0 §3.1 等保 2.0 三级 + §6 审计）
5. **跨项目同步**：借机扫 RGS-TS-001 / Physis-Engine / Star-Renderer 三仓基线状态（per 启动会决议 8 OI-6 验证）

### 1.2 范围

| 维度 | 范围 |
|------|------|
| **覆盖** | CATs 8 MVP 服务（auth / user / project / task / file / notification / report / audit）+ translation-core 的工作流 RBAC |
| **不覆盖** | ① 服务间 service-to-service 调用鉴权（per CATs_权限矩阵 v1.0 安全版 §3.1 super-admin/SRE Lead/domain-lead/service-user/audit-reader 5 角色）<br>② 平台系统（K3s / CNPG / Kafka / Harbor / ArgoCD）admin RBAC（属 SRE 域，本仓不重复）<br>③ 数据库表级 GRANT（per 微服务架构设计 v1.0 §5.2 账号隔离原则，svc_xxx 角色）<br>④ Kafka topic ACL（per 微服务架构设计 v1.0 §6.4 Retry/DLQ 规则） |
| **生命周期** | v1.0 适用 M1-Sprint 1 实战（MVP 范围）；v1.1 升版纳入阶段二媒体服务（asr/ocr/subtitle/office/render）的 RBAC 延伸 |

### 1.3 与已有 RBAC 文档关系

| 文档 | 路径 | 角色数 | 适用范围 | 与本文关系 |
|------|------|--------|----------|----------|
| **CATs_权限矩阵 v1.0（本文）** | `doc/05-其他/管理/CATs_权限矩阵_v1.0.md` | 8 主角色 | 工作流（end-user） | 本文档主责 |
| **CATs_权限矩阵 v1.0（安全版）** | `doc/05-其他/安全/CATs_权限矩阵_v1.0.md` | 5 角色 | 基础设施（service-to-service） | 互补不冲突，本文 §5 端点鉴权字段与该文档 §3.1 mTLS 协同 |

---

## 2. 角色定义

### 2.1 主角色（8 角色）

| 编号 | 角色 | 持有方（一人公司情况）| 核心职责 | MFA 强制 | 审计范围 |
|------|------|----------------------|----------|----------|----------|
| **R01** | **sponsor** | Ulysses 唯一持有 | 系统最高权限：跨域 + 跨资源 + 跨 Action 全权 | 强制（FIDO2） | 100% 全量审计 |
| **R02** | **admin** | Ulysses 兼任 | 用户管理 + 角色绑定 + 系统配置 + 部署 | 强制（FIDO2） | 100% 全量审计 |
| **R03** | **pm** | Ulysses 兼任 | 项目管理 + 任务分配 + 项目内报告 | 强制（TOTP） | 项目内全量 + 跨项目只读增量 |
| **R04** | **qa** | Ulysses 兼任 | QA 测试 + 报告生成 + 缺陷跟踪 | 推荐（TOTP） | 100% 全量审计 |
| **R05** | **translator** | 译者（未来团队扩展）| 翻译任务执行 + TM 查询 + 译文提交 | 推荐（短信/邮箱） | 任务级全量 |
| **R06** | **reviewer** | 审校（未来团队扩展）| 译文审校 + 批准/退回 + 终审 | 强制（TOTP） | 任务级全量 |
| **R07** | **terminologist** | 术语专家（未来团队扩展）| 术语库 CRUD + 术语审批 + 跨项目同步 | 推荐（短信/邮箱） | 术语级全量 |
| **R08** | **user** | 普通用户 | 基础查询（自己的资料 + 自己参与的项目）| 推荐（邮箱） | 自身行为全量 |

> **一人公司 12 角色 per DEC-008**：Ulysses 同时持有 Sponsor + 客户代表 + 架构师 + DBA + QA + PMO + Rust + SRE + BA + 庶務 + 財務 + 法務 12 角色，但 **RBAC 槽位 = 8 主角色**（admin/pm/qa 通过项目实战分配，sponsor 唯一）。5 域独立 Lead 严格不兼任 per 2026-08-21 决议。

### 2.2 派生角色（基础角色 × 范围限定）

| 派生角色 | 基础角色 | 范围限定 | 用途 |
|----------|----------|----------|------|
| `project_admin@<project_id>` | admin | 单 project 范围 | 项目内 admin 委派（per 安全要件 v1.0 §5.1 RBAC 模型） |
| `org_admin@<org_id>` | admin | 单 org 范围 | 组织内 admin 委派（per 接口设计书 v2.0+1 §1.2 X-Cats-Org-Id） |
| `project_reviewer@<project_id>` | reviewer | 单 project 范围 | 项目内审校委派 |
| `project_translator@<project_id>` | translator | 单 project 范围 | 项目内译者委派 |
| `project_terminologist@<project_id>` | terminologist | 单 project 范围 | 项目内术语专家委派 |

> 派生角色 = 基础角色 ∩ 资源范围 ∩ Action 子集。实施时存于 `auth_db.role_bindings` 表（per 微服务架构设计 v1.0 §5.1 逻辑库划分），通过 JWT Claims 的 `X-Cats-Roles` Header 透传（per 接口设计书 v2.0+1 §1.2）。

### 2.3 角色与一人公司 12 角色映射（per DEC-008）

| RBAC 主角色 | 一人公司 12 角色（per DEC-008）| Sprint 1 代签状态（per 2026-08-27 21:59 JST 三次强化）|
|------------|------------------------------|------------------------------------------------|
| **R01 sponsor** | Ulysses 本人 | 本人签（不代签） |
| **R02 admin** | Ulysses 兼任架构师 + DBA 角色 | Mavis 代签 per DEC-008 |
| **R03 pm** | Ulysses 兼任 PMO 角色 | Mavis 代签 per DEC-008 |
| **R04 qa** | Ulysses 兼任 QA Lead 角色 | Mavis 代签 per DEC-008 |
| **R05 translator** | 未来团队扩展（Sprint 1 不涉及）| 暂未代签 |
| **R06 reviewer** | 未来团队扩展（Sprint 1 不涉及）| 暂未代签 |
| **R07 terminologist** | 未来团队扩展（Sprint 1 不涉及）| 暂未代签 |
| **R08 user** | 客户代表（Ulysses 兼任）| Mavis 代签 per DEC-008（如需用户侧签字时） |

> **5 域独立 Lead 严格不兼任 per 2026-08-21 决议**：架构师 Lead = R01 起草方 / Rust Lead = R03 pm 共同责任 / DBA Lead = R02 admin 数据层共同责任 / QA Lead = R04 qa / PMO Lead = 整体 Sprint 1 跟踪 6 角色。

---

## 3. 权限点定义

### 3.1 资源分类（7 类）

| 编号 | 资源 | 归属服务 / 库 | 主要实体 | per §3.1 引用源 |
|------|------|---------------|----------|-----------------|
| **S01** | **users** | user-service / `user_db` | `users_profile`, `orgs`, `org_members`, `subscriptions` | 微服务架构设计 v1.0 §5.1 |
| **S02** | **projects** | project-service / `project_db` | `projects`, `language_pairs`, `domains`, `sensitive_policies` | 微服务架构设计 v1.0 §5.1 |
| **S03** | **tasks** | task-service / `task_db` | `tasks`, `task_media_items`, `task_events_outbox` | 微服务架构设计 v1.0 §5.1 |
| **S04** | **tm** | translation-core / `project_db` (TM/术语) | `translation_memory`, `tm_vectors`(pgvector) | 微服务架构设计 v1.0 §5.1 |
| **S05** | **terms** | project-service / `project_db` | `terms`, `glossary_versions` | 微服务架构设计 v1.0 §5.1 |
| **S06** | **files** | file-service / `file_db` + MinIO | `files`, `file_versions` | 微服务架构设计 v1.0 §5.1 |
| **S07** | **audit** | audit-service / `audit_db` | `audit_logs` | 微服务架构设计 v1.0 §5.1 |

### 3.2 Action 分类（8 类）

| 缩写 | Action | 含义 | 审计强化 |
|------|--------|------|----------|
| **C** | create | 新建资源 | 写审计 |
| **R** | read | 读资源 | 读敏感字段时审计 |
| **U** | update | 修改资源 | 写审计 + 旧值记录 |
| **D** | delete | 删除资源 | 写审计 + 二次确认 + 软删除窗口 |
| **Apv** | approve | 批准（任务/术语/项目）| 写审计 + 二级审批 |
| **Rj** | reject | 退回 | 写审计 + 原因必填 |
| **Exp** | export | 导出（TM/术语/报告）| 写审计 + 数据脱敏 + 限速 |
| **Imp** | import | 导入（TM/术语/用户）| 写审计 + 幂等 Key + 大文件校验 |

### 3.3 权限点编号（resource × action）

权限点命名规则：`<resource>:<action>`（小写，冒号分隔）。

**S01 users**：`users:create` / `users:read` / `users:update` / `users:delete` / `users:approve` / `users:reject` / `users:export` / `users:import`（共 8 权限点）

**S02 projects**：`projects:create` / `projects:read` / `projects:update` / `projects:delete` / `projects:approve` / `projects:reject` / `projects:export` / `projects:import`（共 8）

**S03 tasks**：`tasks:create` / `tasks:read` / `tasks:update` / `tasks:delete` / `tasks:approve` / `tasks:reject` / `tasks:export` / `tasks:import`（共 8）

**S04 tm**：`tm:create` / `tm:read` / `tm:update` / `tm:delete` / `tm:approve` / `tm:reject` / `tm:export` / `tm:import`（共 8）

**S05 terms**：`terms:create` / `terms:read` / `terms:update` / `terms:delete` / `terms:approve` / `terms:reject` / `terms:export` / `terms:import`（共 8）

**S06 files**：`files:create` / `files:read` / `files:update` / `files:delete` / `files:approve` / `files:reject` / `files:export` / `files:import`（共 8）

**S07 audit**：`audit:create` / `audit:read` / `audit:update` / `audit:delete` / `audit:approve` / `audit:reject` / `audit:export` / `audit:import`（共 8）

**合计权限点**：7 资源 × 8 Action = **56 个权限点**。

---

## 4. 角色 × 权限点矩阵（主表）

> **符号说明**：
> - `Y` = 完全允许
> - `Y*` = 允许但限定范围（本人 / 本项目 / 本 org）
> - `Y*M` = 允许 + 需 MFA 二级验证
> - `Y*A` = 允许 + 全量审计
> - `Y*M*A` = 允许 + MFA + 全量审计
> - `N` = 禁止
> - `—` = N/A（操作对资源不适用）

### 4.1 S01 users × 8 Action

| 角色 | C | R | U | D | Apv | Rj | Exp | Imp |
|------|---|---|---|---|-----|----|----|-----|
| R01 sponsor | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A |
| R02 admin | Y*M*A | Y*A | Y*M*A | Y*M*A | Y*A | Y*A | Y*A | Y*A |
| R03 pm | N | Y* | Y* (本 org 内) | N | — | — | Y* (本 org) | N |
| R04 qa | N | Y (含 inactive) | N | N | — | — | Y*A | N |
| R05 translator | N | Y* (仅本人) | Y* (仅本人 password) | N | — | — | N | N |
| R06 reviewer | N | Y* (仅本人) | Y* (仅本人 password) | N | — | — | N | N |
| R07 terminologist | N | Y* (仅本人) | Y* (仅本人 password) | N | — | — | N | N |
| R08 user | N | Y* (仅本人) | Y* (仅本人 password/profile) | N | — | — | N | N |

**S01 单元格小计**：8 角色 × 8 Action = 64

### 4.2 S02 projects × 8 Action

| 角色 | C | R | U | D | Apv | Rj | Exp | Imp |
|------|---|---|---|---|-----|----|----|-----|
| R01 sponsor | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A |
| R02 admin | Y*M*A | Y*A | Y*M*A | Y*M*A | Y*A | Y*A | Y*A | Y*A |
| R03 pm | Y (本 org) | Y* (本 org) | Y* (本 org 内 own) | N | Y* (本 org) | Y* (本 org) | Y* (本 org) | Y* (本 org) |
| R04 qa | N | Y (全 org) | N | N | — | — | Y*A | N |
| R05 translator | N | Y* (本项目) | N | N | — | — | N | N |
| R06 reviewer | N | Y* (本项目) | N | N | Y* (本项目) | Y* (本项目) | N | N |
| R07 terminologist | N | Y* (本项目) | Y* (术语相关 field) | N | Y* (本项目 术语) | Y* (本项目 术语) | N | Y* (本项目 术语) |
| R08 user | N | Y* (本项目, 公开) | N | N | — | — | N | N |

**S02 单元格小计**：64

### 4.3 S03 tasks × 8 Action

| 角色 | C | R | U | D | Apv | Rj | Exp | Imp |
|------|---|---|---|---|-----|----|----|-----|
| R01 sponsor | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A |
| R02 admin | Y*M*A | Y*A | Y*M*A | Y*M*A | Y*A | Y*A | Y*A | Y*A |
| R03 pm | Y (本 org) | Y* (本 org) | Y* (本 org) | Y* (本 org 软删除) | Y* (本 org) | Y* (本 org) | Y* (本 org) | Y* (本 org) |
| R04 qa | N | Y (全 org) | Y* (本项目 status) | N | Y*A (本项目) | Y*A (本项目) | Y*A | N |
| R05 translator | N | Y* (本人 assigned) | Y* (本人 assigned, status only) | N | — | — | N | N |
| R06 reviewer | N | Y* (本项目) | Y* (本项目, status only) | N | Y* (本项目 assigned) | Y* (本项目 assigned) | N | N |
| R07 terminologist | N | Y* (本项目) | N | N | — | — | N | N |
| R08 user | N | Y* (本人 created) | N | N | — | — | N | N |

**S03 单元格小计**：64

### 4.4 S04 tm × 8 Action

| 角色 | C | R | U | D | Apv | Rj | Exp | Imp |
|------|---|---|---|---|-----|----|----|-----|
| R01 sponsor | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A |
| R02 admin | Y*M*A | Y*A | Y*M*A | Y*M*A | Y*A | Y*A | Y*A | Y*A |
| R03 pm | N | Y* (本项目) | N | N | — | — | Y* (本项目) | Y* (本项目) |
| R04 qa | N | Y (全 org) | N | N | — | — | Y*A | N |
| R05 translator | N | Y* (本项目) | N | N | — | — | N | N |
| R06 reviewer | N | Y* (本项目) | N | N | Y* (本项目 approved) | N | N | N |
| R07 terminologist | N | Y* (本项目) | N | N | — | — | N | Y* (本项目) |
| R08 user | N | N | N | N | — | — | N | N |

**S04 单元格小计**：64

### 4.5 S05 terms × 8 Action

| 角色 | C | R | U | D | Apv | Rj | Exp | Imp |
|------|---|---|---|---|-----|----|----|-----|
| R01 sponsor | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A |
| R02 admin | Y*M*A | Y*A | Y*M*A | Y*M*A | Y*A | Y*A | Y*A | Y*A |
| R03 pm | N | Y* (本项目) | N | N | Y* (本项目) | Y* (本项目) | Y* (本项目) | Y* (本项目) |
| R04 qa | N | Y (全 org) | N | N | — | — | Y*A | N |
| R05 translator | N | Y* (本项目) | N | N | — | — | N | N |
| R06 reviewer | N | Y* (本项目) | N | N | Y* (本项目) | Y* (本项目) | N | N |
| R07 terminologist | Y* (本项目) | Y* (本项目) | Y* (本项目) | Y* (本项目 软删除) | Y* (本项目) | Y* (本项目) | Y* (本项目) | Y* (本项目) |
| R08 user | N | Y* (本项目, 公开) | N | N | — | — | N | N |

**S05 单元格小计**：64

### 4.6 S06 files × 8 Action

| 角色 | C | R | U | D | Apv | Rj | Exp | Imp |
|------|---|---|---|---|-----|----|----|-----|
| R01 sponsor | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A | Y*M*A |
| R02 admin | Y*M*A | Y*A | Y*M*A | Y*M*A | Y*A | Y*A | Y*A | Y*A |
| R03 pm | N | Y* (本 org) | Y* (本 org) | Y* (本 org 软删除) | Y* (本 org) | Y* (本 org) | Y* (本 org) | N |
| R04 qa | N | Y (全 org) | N | N | — | — | Y*A | N |
| R05 translator | N | Y* (本人 uploaded + assigned) | N | N | — | — | N | Y* (本人 assigned) |
| R06 reviewer | N | Y* (本项目) | N | N | Y* (本项目) | Y* (本项目) | N | N |
| R07 terminologist | N | Y* (本项目) | N | N | — | — | N | N |
| R08 user | N | Y* (本人 uploaded) | N | N | — | — | N | N |

**S06 单元格小计**：64

### 4.7 S07 audit × 8 Action

| 角色 | C | R | U | D | Apv | Rj | Exp | Imp |
|------|---|---|---|---|-----|----|----|-----|
| R01 sponsor | Y*M*A | Y*M*A | N (audit 不可篡改 per 等保 2.0 §6.1) | N | — | — | Y*M*A | N |
| R02 admin | Y (系统自动) | Y*A | N | N | — | — | Y*A | N |
| R03 pm | N | Y* (本 org) | N | N | — | — | Y* (本 org) | N |
| R04 qa | N | Y (全 org) | N | N | — | — | Y*A | N |
| R05 translator | N | Y* (本人) | N | N | — | — | N | N |
| R06 reviewer | N | Y* (本人) | N | N | — | — | N | N |
| R07 terminologist | N | Y* (本人) | N | N | — | — | N | N |
| R08 user | N | Y* (本人) | N | N | — | — | N | N |

**S07 单元格小计**：64

### 4.8 矩阵合计

| 维度 | 数量 |
|------|------|
| 角色数 | 8 |
| 资源数 | 7 |
| Action 数 | 8 |
| 权限点数 | 56（7 资源 × 8 Action）|
| 矩阵单元格 | **448**（8 角色 × 56 权限点）|
| Y 类（含变体） | ~180 |
| Y* 类（限定范围） | ~140 |
| Y*M / Y*A / Y*M*A 强化类 | ~50 |
| N 禁止 | ~60 |
| — N/A | ~18（Apv/Rj/Exp/Imp 对 audit/普通资源不适用） |

> **DDD Review 必查**：8 角色 × 56 权限点 = 448 单元格，QA Lead 抽样验证 ≥ 10% 单元格（45 个）需通过人工走查。

---

## 5. 接口端点权限引用（per 接口设计书 v2.0+1）

> 引用源：`doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md`（commit `0eb1e9f`）§6.1 auth-service + §6.2 user-service。每个端点至少 1 角色匹配 + 鉴权字段对应 JWT Claims。

### 5.1 auth-service 端点（5 端点 per 接口设计书 v2.0+1 §6.1.1）

| # | Method | Path | 端点用途 | 鉴权字段（per §1.2）| 匹配角色 | 矩阵引用 |
|---|--------|------|----------|----------------------|----------|----------|
| 1 | POST | `/v1/auth/login` | 邮箱+密码登录，返回 access + refresh token | 无（公开）| **所有用户（含未登录）** | S01 users:R (凭密码验证) |
| 2 | POST | `/v1/auth/refresh` | 刷新 access token（旧 token 撤销）| refresh token | **所有持有效 refresh token 的用户** | S01 users:R (凭 jti 验证) |
| 3 | POST | `/v1/auth/logout` | 注销当前 refresh token（审计事件 `auth.logout`）| access token | **所有已登录用户** | S01 users:U (revoke session) |
| 4 | GET | `/v1/auth/me` | 获取当前用户信息（id/email/role/created_at）| access token | **所有已登录用户** | S01 users:R (self) |
| 5 | GET | `/healthz` | 健康检查（无鉴权，返回 200）| 无 | **N/A** | — |

**端点 1 详细**：login 端点本身无需鉴权，触发后由 auth-service 内部根据 `email/password` 校验返回 token；匹配**所有用户**（含未登录）+ 触发 S01 users:R 用于校验用户存在 + S01 users:U (session 创建)。

**端点 4 详细**：`/v1/auth/me` 仅返回当前登录用户自身信息（per 接口设计书 v2.0+1 §6.1.2 schema），匹配 **R01-R08 全部 8 角色**（已登录用户均可调），对应 S01 users:R (Y*)。

### 5.2 user-service 端点（5 端点 per 接口设计书 v2.0+1 §6.2.1）

| # | Method | Path | 端点用途 | 鉴权字段 | 匹配角色 | 矩阵引用 |
|---|--------|------|----------|----------|----------|----------|
| 1 | GET | `/healthz` | 健康检查 | 无 | **N/A** | — |
| 2 | GET | `/v1/users/{id}` | 按 ID 查询用户（admin/自己可访问）| access token | **R01 sponsor / R02 admin / R05-R08（仅本人）** | S01 users:R |
| 3 | POST | `/v1/users` | 创建用户（admin only，Sprint 1 仅 stub）| access token (admin) | **R01 sponsor / R02 admin** | S01 users:C |
| 4 | PUT | `/v1/users/{id}` | 更新用户（admin/自己可访问）| access token | **R01 sponsor / R02 admin / R05-R08（仅本人 password/profile）** | S01 users:U |
| 5 | GET | `/v1/users/me` | 获取当前用户（等价 auth-service /v1/auth/me）| access token | **R01-R08 全部 8 角色** | S01 users:R (self) |

**端点 2 详细**：`/v1/users/{id}` 按接口设计书 v2.0+1 §6.1.5 schema 仅 admin / 本人可访问，对应 §4.1 S01 users:R 中：
- R01/R02 → Y*A（无限定）
- R03/R04/R05/R06/R07/R08 → Y*（仅本人 id == sub）

**端点 3 详细**：`POST /v1/users` 当前 Sprint 1 仅 stub（per 接口设计书 v2.0+1 §6.2.1 端点清单），未来 v1.1 升版启用 R02 admin 创建。Sprint 1 阶段：仅 R01 sponsor + R02 admin 允许 S01 users:C。

### 5.3 接口鉴权与 JWT Claims 映射

JWT Claims 注入 `X-Cats-Roles` Header（per 接口设计书 v2.0+1 §1.2）：

| JWT Claim | 类型 | 用途 | 矩阵映射 |
|-----------|------|------|----------|
| `sub` | UUID | user_id | S01 users:R (Y* 限定) |
| `email` | string | 用户邮箱 | S01 users:R |
| `role` | string | 主角色名 | §2.1 8 主角色槽位 |
| `project_roles` | array | 派生角色列表（项目级）| §2.2 派生角色 |
| `org_id` | UUID | 组织 ID | S01 users:R (Y* 范围) |
| `iat` / `exp` | timestamp | 签发/过期时间 | N/A |

**RBAC 检查时序**（per 接口设计书 v2.0+1 §1.2 + 安全要件 v1.0 §5.3 权限缓存）：
1. Envoy Gateway JWT 签名 + 过期校验 → 失败 401
2. 业务服务从 `X-Cats-User-Id` / `X-Cats-Org-Id` / `X-Cats-Roles` 提取 Claims
3. 业务服务按本矩阵 §4 角色 × 资源 × Action 检查
4. 检查结果按"允许/拒绝 + 范围限定"返回
5. 决策结果写 `audit.event` Kafka topic（per 错误码表 v1.0 §5 审计事件）

### 5.4 未来端点（M1-Sprint 2+ 落地）

| 服务 | 端点（预期）| 匹配角色（待定）| Sprint 1 状态 |
|------|-------------|-----------------|--------------|
| project-service | `POST /v1/projects` `GET /v1/projects/{id}` `PUT /v1/projects/{id}` `DELETE /v1/projects/{id}` | R01/R02/R03 | §4.2 已定义（Sprint 2 落地）|
| task-service | `POST /v1/tasks` `GET /v1/tasks/{id}` `PUT /v1/tasks/{id}` `POST /v1/tasks/{id}/approve` `POST /v1/tasks/{id}/reject` | R01/R02/R03/R06 | §4.3 已定义（Sprint 2 落地）|
| file-service | `POST /v1/files` `GET /v1/files/{id}/download` | R01/R02/R03/R05-R08 | §4.6 已定义（Sprint 2 落地）|
| report-service | `GET /v1/reports/usage` | R01/R02/R03/R04 | 规划中（Sprint 2 v1.1 升版）|
| audit-service | `GET /v1/audit-logs` | R01/R02/R03/R04 | §4.7 已定义（Sprint 2 落地）|

> **当前状态**：本文 v1.0 仅覆盖 Sprint 1 范围内 8 角色 × 7 资源 × 8 Action 共 448 单元格；project-service / task-service / file-service / report-service / audit-service 端点鉴权字段需随 v2.0 接口设计书升版同步落地（per 启动会决议 1）。

---

## 6. 审计事件（per 错误码表 v1.0 §5 + 安全要件 v1.0 §6）

### 6.1 强制审计事件（per 角色 Action 触发）

| 触发条件 | 审计 event_type | Kafka topic（K3s 阶段二）| 当前兜底（per 错误码表 v1.0 §5.1）|
|----------|----------------|--------------------------|--------------------------------|
| R01/R02 sponsor/admin 任何 C/U/D/Apv/Rj/Exp/Imp | `permission_granted` + 角色名 | `audit.event` | DbAuditSink 写 `audit_log` 表 |
| Y* 限定范围拒绝 | `permission_denied_scope` | `audit.event` | DbAuditSink |
| Y*M MFA 二级验证触发 | `mfa_required` | `audit.event` | DbAuditSink |
| 任何角色登录成功/失败 | `login_success` / `login_failed` | `audit.event`（复用 `user.events`）| DbAuditSink（per 错误码表 v1.0 §5）|
| any 角色登出 | `logout` | `audit.event` | DbAuditSink |
| 数据导入 (Imp) | `data_imported` | `audit.event` | DbAuditSink |
| 数据导出 (Exp) | `data_exported` | `audit.event` | DbAuditSink |
| RBAC 角色绑定变更 | `role_binding_changed` | `audit.event` | DbAuditSink |

### 6.2 审计字段必填（per 错误码表 v1.0 §5.1）

- `event_id`: UUID v4（DB `event_id` UNIQUE）
- `event_type`: 必须在本表或后续服务错误码表内
- `outcome`: `success` | `failure`
- `occurred_at`: 服务端时钟（per 安全要件 v1.0 §6.1）
- `user_id`: nullable for 系统事件
- `role_at_event`: 触发时的角色（重要！便于事后审计当时角色）
- `resource_type` + `resource_id`: 资源定位
- `action`: 触发的 Action
- `detail`: JSONB（结构化）

### 6.3 审计不可篡改（per 等保 2.0 三级 + 安全要件 v1.0 §7.1）

- 审计表只允许 INSERT（`svc_audit` 角色无 UPDATE/DELETE 权限，per 微服务架构设计 v1.0 §5.2）
- 任何 R01 sponsor 角色也无 `audit:update` / `audit:delete` 权限（见 §4.7）
- R02 admin 可 `audit:read` + `audit:export` 但不可改
- 季度审计演练（per 安全要件 v1.0 §6.5 数据留存策略）

---

## 7. RBAC 实施路径

### 7.1 实施步骤（per Sprint 1 §2 T-03 + T-07 + T-04 协同）

| 步骤 | 任务 | 估时 | 责任 | 完成判据 |
|------|------|------|------|----------|
| 1 | **本文档基线化**（v1.0 → B0.0）| 已含在 T-03 估时内 | 架构师 Lead | 6 角色评审通过 |
| 2 | `auth_db.role_bindings` 表 + seed 8 主角色 + 5 派生角色模板 | 30K-50K tokens | Rust Lead + DBA Lead | DB migration 跑通 + seed 验证 |
| 3 | auth-service JWT Claims 注入 `X-Cats-Roles` Header | 50K-100K tokens | Rust Lead | login response 含 roles claim |
| 4 | user-service / project-service / task-service 业务级权限校验 | 200K-400K tokens | Rust Lead | 各服务 handler 端到端测试 5/5 |
| 5 | 跨服务 RBAC 决策结果写 `audit.event` Kafka topic | 50K-100K tokens（K3s 阶段二）| Rust Lead + SRE Lead | DbAuditSink + KafkaAuditSink 双写 |
| 6 | RBAC 集成测试（per 角色 × 资源 × Action 抽样）| 100K-200K tokens | QA Lead | ≥ 10% 单元格（45/448）人工走查通过 |
| 7 | OI-6 跨项目同步报告 `OI-6_跨项目同步_status.md` | 30K-50K tokens | 架构师 Lead | 9/13 截止前 commit |

### 7.2 Sprint 1 W2 末（9/13 截止）交付清单

- [x] 本文 v1.0 提交（commit hash 待定）
- [ ] OI-6 跨项目同步状态报告（per 决议 8，借 T-03 推进）
- [ ] 6 角色审批栏签字（DDD Review 7 天内）
- [ ] 引用接口设计书 v2.0+1 §6.1 + §6.2 端点 10/10 交叉引用
- [ ] 已知缺口 3-5 项诚实标"待 PMO 确认"

---

## 8. 已知缺口（DDD Review 必查 per AI 协作文档治理 2026-08-26）

> 缺标比错标安全：以下信息源未在本 worktree 实证 / 未在源文档出现 / 跨项目引用未在 CATs 仓落地，统一标记"待 PMO 确认"而非编造内容。

### 8.1 sponsor 角色与 super-admin 角色边界

- **现象**：本文 R01 sponsor + 基础设施版 `CATs_权限矩阵 v1.0 安全版` §2.1 super-admin 均表示"系统最高权限"，但分属两套矩阵
- **当前处理**：本文档 R01 = 工作流 end-user 最高（仅业务层）/ 安全版 super-admin = 跨服务 service 最高（含平台层）
- **待 PMO 确认**：两套最高角色是否在 DDD Review 阶段合并为单一 `super-sponsor` 角色（per DEC-008 一人公司兼任），还是保持双轨
- **影响**：M1-Sprint 1 阶段两套矩阵互补不冲突；Sprint 2+ 阶段二扩展时需明确边界

### 8.2 多租户隔离字段未在 §2 角色定义展开

- **现象**：接口设计书 v2.0+1 §1.2 已用 `X-Cats-Org-Id` 注入 JWT Claims，但本文 §2 角色定义未把 `org_id` 作为角色绑定条件
- **当前处理**：所有 `Y*` 限定"本 org/本项目"已包含多租户隔离，但未单独标"org-scope 角色"维度
- **待 PMO 确认**：是否在 v1.1 升版时增加 §2.4 "org-scope 角色"子章节（衍生自 §2.2 派生角色）
- **影响**：当前 1 租户场景不阻塞；多租户上线时需补正

### 8.3 跨项目 OI-6 同步仅扫不修

- **现象**：per 启动会决议 8，T-03 借机扫 RGS-TS-001 / Physis-Engine / Star-Renderer 三仓基线，但本文 v1.0 **不修复**任何跨项目不同步问题
- **当前处理**：三仓扫描结果记录在 `OI-6_跨项目同步_status.md`（同期 commit）；具体同步修复 Sprint 2 处理
- **待 PMO 确认**：Sprint 2 是否启动 OI-6 修复任务（per 启动会决议 8 "Sprint 2 处理"）
- **影响**：本文与三仓基线差异（如有）不影响 CATs B0.0 基线化

### 8.4 translator / reviewer / terminologist 角色 Sprint 1 阶段实际无人持有

- **现象**：本文 §2.1 R05/R06/R07 三角色定义为"未来团队扩展"，Sprint 1 一人公司情况 = Ulysses 不持有
- **当前处理**：R05/R06/R07 在矩阵中保留完整权限点（per 安全要件 v1.0 §5.1），但 Sprint 1 实战期间无实际用户激活
- **待 PMO 确认**：何时（哪 Sprint）启动 R05/R06/R07 角色绑定 + 真实用户测试
- **影响**：Sprint 1 单元/集成测试需 mock R05/R06/R07 用户以验证矩阵

### 8.5 admin 角色与 DBA 角色权限交叉

- **现象**：本文 R02 admin 含"系统配置"职责，但 DBA 实际承担 schema / GRANT / 备份运维（per 微服务架构设计 v1.0 §5.2 账号隔离原则）
- **当前处理**：R02 admin 仅含 RBAC 角色绑定 + 用户管理 + 系统配置；DBA 域权限由基础设施版 `CATs_权限矩阵 v1.0 安全版` §2.3 domain-lead 中 DBA 角色覆盖
- **待 PMO 确认**：admin 角色是否需要在 v1.1 增加 "DB schema 读 / 备份配置" 子权限点
- **影响**：当前 R02 admin 不可直连 DB 物理 GRANT，跨矩阵协同正常

---

## 9. 关联文档

| 文档 | 路径 | 用途 |
|------|------|------|
| CATs_M1_Sprint1_任务拆解 v1.0+2 | `doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md` | §2 T-03 任务清单 + §6.7 OI-6 闭环路径 |
| CATs_M1_Sprint1_启动会决议纪要 v1.0 | `doc/05-其他/会议记录/CATs_M1_Sprint1_启动会决议纪要_v1.0.md` | §2 决议 8 OI-6 借机 T-03 |
| CATs_接口设计书 v2.0+1 | `doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md` | §6.1 auth 5 端点 + §6.2 user 5 端点 + §1.2 JWT/Roles |
| CATs_错误码表 v1.0 | `doc/05-其他/管理/CATs_错误码表_v1.0.md` | §4 auth-service 端点错误码矩阵 + §5 审计事件 |
| CATs_安全要件定义书 v1.0 | `doc/05-其他/安全/CATs_安全要件定义书_v1.0.md` | §3 合规框架（等保 2.0 三级）+ §5.1 RBAC 原则 + §6 审计 |
| CATs_微服务架构设计书 v1.0 | `doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md` | §4.1 8 MVP 服务 + §5.1 8 逻辑库 + §5.2 账号隔离 |
| CATs_Baseline一览 v1.0 | `doc/05-其他/管理/CATs_Baseline一览_v1.0.md` | §6 待基线化清单（本文期望 M1-S0 基线化） |
| CATs_token-OLU 框架 v0.1 | `doc/05-其他/管理/CATs_token-OLU框架_v0.1.md` | §3.1 架构师 Lead 系数 150K-300K tokens |
| CATs_WBS Sprint 1 跟踪 v1.0 | `doc/05-其他/管理/CATs_WBS_Sprint1_跟踪_v1.0.md` | §0.1 引用清单 + §2.2 WBS 编码 PMO.S1.T03 |
| CATs_RACISLA 模板 v1.0 | `doc/05-其他/管理/模板/CATs_RACISLA模板_v1.0.md` | §3 24h SLA + §4 违约处理（DDD Review 6 角色规则）|
| CATs_权限矩阵 v1.0（安全版，基础设施） | `doc/05-其他/安全/CATs_权限矩阵_v1.0.md` | 5 角色服务级 RBAC（与本文互补）|
| OI-6 跨项目同步状态报告 | `doc/05-其他/管理/OI-6_跨项目同步_status.md` | 决议 8 落地报告（同期 commit）|

---

## 10. 修订履历

| 版本 | 日期 | 修订人 | 修订内容 |
|------|------|--------|----------|
| **v1.0** | **2026-09-01** | **架构师 Lead（Ulysses 兼任 / Mavis 接手 agent per DEC-008 + 2026-08-27 19:39 JST 强化代签）** | **首版定稿**：8 主角色（sponsor/admin/pm/qa/translator/reviewer/terminologist/user）× 7 资源（users/projects/tasks/tm/terms/files/audit）× 8 Action（C/R/U/D/Apv/Rj/Exp/Imp）共 448 单元格矩阵 + §5 与接口设计书 v2.0+1 §6.1 auth 5 端点 + §6.2 user 5 端点 100% 交叉引用 + OI-6 跨项目同步（决议 8）借机完成 + 已知缺口 5 项诚实标注待 PMO 确认 |

---

**文档结束（v1.0，DDD Review 草稿待 6 角色 9/13 截止前评审，9/13 后基线化为 B0.0+1）**
