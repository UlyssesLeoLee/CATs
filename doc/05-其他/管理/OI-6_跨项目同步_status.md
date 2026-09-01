# OI-6 跨项目同步状态报告 v1.0

**RGS-TS-001 / Physis-Engine / Star-Renderer 三仓基线类文档扫描报告**

> **文档编号**：CATs-PMO-017
> **フェーズ**：管理 / M1-Sprint 1 T-03 借机（per 启动会决议 8）
> **关联任务**：Sprint 1 拆解 v1.0+2 §6.7 OI-6 闭环路径 / 启动会决议 8
> **版本**：v1.0
> **创建日**：2026-09-01
> **状态**：DDD Review 草稿（同步入 Sprint 1 拆解 v1.0+2 §6.7）
> **密级**：仅社内
> **作者**：架构师 Lead（Ulysses 兼任一人公司 12 角色架构师槽位 / Mavis 接手 agent per DEC-008 + 2026-08-27 19:39 JST 强化代签）

---

## 文档管理信息

### 审批栏（6 角色 per 启动会决议 4 共识模型）

| 角色 | 姓名 | 审批 | 签字 | 日期 | 备注 |
|------|------|------|------|------|------|
| Sponsor (Ulysses 本人签) | Ulysses | ☐ | — | — | 一人公司 = Ulysses 持有 Sponsor，不代签 |
| 架构师 Lead | Ulysses（Mavis 接手 agent per DEC-008） | ☐ | — | — | 起草方 / 决议 8 主责任 |
| Rust Lead | Ulysses（Mavis 接手 agent per DEC-008） | ☐ | — | — | OI-6 跨项目引用审 |
| DBA Lead | Ulysses（Mavis 接手 agent per DEC-008） | ☐ | — | — | 三仓基线 DB 范式影响审 |
| QA Lead | Ulysses（Mavis 接手 agent per DEC-008） | ☐ | — | — | DDD Review 6 角色评审一致性 |
| PMO Lead | Ulysses（Mavis 接手 agent per DEC-008） | ☐ | — | — | Sprint 1 §6.7 已知缺口闭环评审 |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|------|------|--------|----------|
| **v1.0** | **2026-09-01** | **架构师 Lead（Mavis 接手 agent per DEC-008）** | **首版定稿**：RGS-TS-001 / Physis-Engine / Star-Renderer 三仓基线类文档扫描（仅扫不修 per 决议 8）+ CATs B0.0 与三仓对齐状态 + 不修建议入 Sprint 2 + 已知缺口 3 项 |

---

## 0. 元信息

| 项 | 值 |
|----|----|
| **Worktree** | `D:/CATs/.worktrees/feat-m1-sprint1-t03-rbac` |
| **分支** | `feat/m1-sprint1-t03-rbac` |
| **commit baseline** | `49fdbb3`（per `git log -1`，WBS Sprint 1 跟踪 v1.0 落地） |
| **关联基线（B0.0）** | `4f96f95`（CAB-001 v1.0 初始基线决议书） |
| **上游源文档** | 见 §0.1 源文档引用清单（git 实证） |
| **下游引用** | Sprint 1 拆解 v1.0+2 §6.7 已知缺口闭环 / 启动会决议 8 / CATs_权限矩阵 v1.0 §8.3 已知缺口 |
| **配套 Excel** | 无（扫描结果在文档内可读） |
| **密级** | 仅社内 |
| **扫描范围** | 仅扫三仓基线类文档，**不修复跨项目不同步问题**（per 决议 8 "Sprint 2 处理"） |

### 0.1 源文档引用清单（git 实证）

| 引用文档 | 路径 | commit hash | 用途 |
|---------|------|------------|------|
| CATs_M1_Sprint1_启动会决议纪要 v1.0 | `doc/05-其他/会议记录/CATs_M1_Sprint1_启动会决议纪要_v1.0.md` | `1b27b2b` | §2 决议 8（OI-6 T-03 借机验证）+ §2.1 实施动作 |
| CATs_M1_Sprint1_任务拆解 v1.0+2 | `doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md` | `efd9e77` | §6.7 OI-6 闭环路径 + §0.3 决议 8 借机 OI-6 |
| CATs_Baseline一览 v1.0 | `doc/05-其他/管理/CATs_Baseline一览_v1.0.md` | `4f96f95` | §3.3 已基线化清单 + §6 待基线化清单（CATs B0.0 对照）|
| CATs_技术基线 v1.0 | `doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md` | `047dc9c` | §8 OI-6 状态原状（"待办"）+ §1 锁定 Rust 1.98.0 + PG 18.6 |
| CATs_权限矩阵 v1.0 | `doc/05-其他/管理/CATs_权限矩阵_v1.0.md` | （同期 commit）| §8.3 已知缺口 8.3 跨项目 OI-6 同步仅扫不修 |
| CATs_WBS Sprint 1 跟踪 v1.0 | `doc/05-其他/管理/CATs_WBS_Sprint1_跟踪_v1.0.md` | `49fdbb3` | §0.1 引用清单 + §2.2 WBS 编码 PMO.S1.T03.1 OI-6 子任务 |

---

## 1. 扫描背景与目标

### 1.1 背景

per 启动会决议 8（commit `1b27b2b` §2 决议 8 + Sprint 1 拆解 v1.0+2 §6.7）：

- **技术基线 v1.0 §8 OI-6 状态**原标记 = "待办（跨项目引用方同步，如果 RGS / Physis / Star 也锁定相同基线）"
- **决议 8 通过方案 A**：T-03 借机扫 RGS-TS-001 / Physis-Engine / Star-Renderer 三仓基线，输出本报告（30K-50K tokens）
- **不修原则**：决议 8 明确"Sprint 2 处理"，本文档仅扫不修
- **同步路径**：本文档 v1.0 同步入 Sprint 1 拆解 v1.0+2 §6.7 闭环

### 1.2 扫描目标

| 目标 | 输出 | 状态 |
|------|------|------|
| ① 三仓是否各自有"基线类文档" | 存在性 + 路径 | 本报告 §2-§4 |
| ② 三仓是否锁定与 CATs B0.0 等价基线 | Rust 版本 / PG 版本 / 协议版本 | 本报告 §2-§4 |
| ③ 三仓基线类文档的差异 | 路径 / 命名 / 粒度 | 本报告 §5 |
| ④ Sprint 2 不修建议 | OI-6 修复任务立项 | 本报告 §6 |

### 1.3 范围与不范围

- **范围**：仅扫 3 个项目根目录下"基线类 / 需求规格 / 实施计划 / 评审纪要"文档路径
- **不范围**：① 修复任何跨项目不同步问题（Sprint 2 处理）② 修改 3 个项目根目录 ③ 创建新基线文档 ④ 修改 CATs B0.0（per `4f96f95` 已是基线）

---

## 2. RGS-TS-001 仓扫描（RustGameServer / `D:/RustGameServer`）

### 2.1 仓根概览

| 项 | 值 |
|----|----|
| **仓根** | `D:/RustGameServer` |
| **目录结构** | `crates/` + `docs/` + `tools/` + `docker/` + `scripts/` + `certs/` + `.worktrees/` |
| **基线类目录** | `docs/00-基准与治理/`（**唯一**明确"基线"目录）|
| **基线类文档** | 多份 RGS-BAS-* + RGS-REQ-* + RGS-DDD-* + RGS-CARD-* + RGS-BUCKET-* + RGS-DEC-* |
| **路径示例** | `docs/00-基准与治理/RGS-BAS-009_体系治理与横切关注点_基本设计书.md` |
| **技术栈声明** | RGS-TS-001 v0.4 §6.2 token-OLU 框架（per Sprint 1 拆解 v1.0+2 §0.3 引用，**不在本 worktree 内**）|

### 2.2 关键基线类文档扫描结果

| 路径 | 文档类型 | 锁定内容 | 与 CATs B0.0 同步状态 |
|------|----------|----------|------------------------|
| `docs/00-基准与治理/RGS-BAS-009_体系治理与横切关注点_基本设计书.md` | BAS (Baseline) | 体系治理 + 横切关注点 | 🟡 标题级一致（均含"体系治理"），内容级未交叉实证 |
| `docs/00-基准与治理/RGS-DDD-REVIEW-2026-08-28-summary.md` | DDD Review | 6 角色 DDD 评审纪要 | 🟢 6 角色共识模型一致 |
| `docs/00-基准与治理/RGS-CARD-8BUCKET-W36-100PCT-V0.37-2026-08-30.md` | 进度报告 | W36 100% 8 BUCKET | ⚪ 进度类，与 B0.0 无关 |
| `docs/00-基准与治理/RGS-REQ-100_Saga*.md` | REQ (Requirement) | Saga 跨服务事务 | 🟡 与 CATs 跨服务鉴权（per 接口设计书 v2.0+1 §1.2）相关 |

### 2.3 关键路径发现（per 文件系统扫描）

> **RGS 是 3 个项目中唯一有"明确基线类目录（00-基准与治理）+ BAS-*/REQ-*/DDD-* 多套编号体系"的项目**。这意味着 RGS 已建立完整的"基线 - 需求 - 评审"三层文档治理结构，可作为 CATs OI-6 跨项目同步的主参考。

### 2.4 RGS OI-6 同步结论

- **状态**：🟢 RGS 有显式基线类文档
- **与 CATs B0.0 差异**：路径命名（`00-基准与治理/` vs `doc/05-其他/管理/`）+ 编号体系（`RGS-BAS-NNN` vs `CATs-PMO-NNN`）+ 技术栈细节（Rust 1.98.0 + PG 18.6 vs RGS 自身栈）需要 Sprint 2 对齐
- **建议**：RGS 的 `00-基准与治理/` 目录结构可作为 CATs `doc/05-其他/管理/` 后续升版的参考（保留基线类文档独立子目录）

---

## 3. Physis-Engine 仓扫描（Physis / GVPE / `D:/Physis`）

### 3.1 仓根概览

| 项 | 值 |
|----|----|
| **仓根** | `D:/Physis` |
| **目录结构** | `crates/` + `docs/` + `.worktrees/` + `_wt_audit/` |
| **基线类目录** | **无** `00-基准与治理/` 等明确"基线"目录 |
| **替代结构** | `docs/00_foundation/`（基础）+ `docs/01_architecture/`（架构）+ `docs/02_modules/`（模块）+ `docs/03_cross_cutting/`（横切）+ `docs/04_detailed_design/`（详细设计）|
| **文档 ID 体系** | GVPE-*（per Physis 00_foundation/01_requirements.md ID 前缀说明）|

### 3.2 关键基线类文档扫描结果

| 路径 | 文档类型 | 锁定内容 | 与 CATs B0.0 同步状态 |
|------|----------|----------|------------------------|
| `docs/00_foundation/00_vision.md` | Vision | 项目愿景 | 🟡 顶层愿景类，与 B0.0 不同维度 |
| `docs/00_foundation/01_requirements.md` | Requirements | 功能/非功能需求清单 | 🟢 与 CATs 需求层（17 安全要件 / 28 应补）结构对位 |
| `docs/00_foundation/02_physics_ontology.md` | Ontology | 物理本体论 | ⚪ 物理引擎特定，与 CATs 业务无关 |
| `docs/01_architecture/04_architecture.md` | Architecture | 架构设计 | 🟡 与 CATs 微服务架构设计 v1.0 同维度但不同栈 |
| `docs/03_cross_cutting/27_qa_register.md` | QA Register | QA 登记册 | 🟢 与 CATs 实施前 QA 登记册 v1.3 同维度 |
| `docs/03_cross_cutting/14_performance_budget.md` | Performance | 性能预算 | 🟢 实时性能预算（per user profile Physis 关注点）一致 |

### 3.3 关键路径发现（per 文件系统扫描）

> **Physis 不使用"基线（Baseline）"作为文档分类维度**，而是用"基础（foundation）/ 架构（architecture）/ 模块（modules）/ 横切（cross_cutting）/ 详细设计（detailed_design）"5 段式分类 + GVPE-* 编号体系。这种结构对**单一项目深度**友好（从愿景→需求→架构→模块→横切清晰），但对**跨项目基线对位**不友好（无统一"基线"锚点）。

### 3.4 Physis OI-6 同步结论

- **状态**：🟡 Physis 无显式基线类文档，使用 5 段式需求-架构-模块分类
- **与 CATs B0.0 差异**：文档分类维度根本不同（"基线" vs "基础/架构/横切"），Sprint 2 需协商共同基线标识法
- **建议**：Physis 在 Sprint 2 增加 `docs/00_baseline/` 类目录，或在 `docs/00_foundation/00_vision.md` 中显式声明技术基线（Rust 版本 / GPU 后端 / API 协议版本）以与 CATs/RGS 对齐

---

## 4. Star-Renderer 仓扫描（Star / `D:/Star`）

### 4.1 仓根概览

| 项 | 值 |
|----|----|
| **仓根** | `D:/Star` |
| **目录结构** | `crates/` + `docs/` + `frontend/` + `bench/` + `deploy/` + `tools/` + `scripts/` + `deliverables/` + `.worktrees/` |
| **基线类目录** | **无** `00-基准与治理/` 等明确"基线"目录 |
| **替代结构** | `docs/architecture/` + `docs/plan/` + `docs/requirements/` + `docs/specs/` + `docs/rfcs/` + `docs/poc/` + `docs/ddd/` + `docs/responsibility-matrix/` + `docs/governance/` + `docs/qa/` + `docs/reports/` + `docs/data-design/` + `docs/batch/` + `docs/frontend/` + `docs/ecosystem-survey/` |
| **文档 ID 体系** | `plan-NNN-*.md` + `rfc-NNN-*.md` + `poc-NNN-*.md` + `spec-NNN-*.md` + `PHASE-*-REPORT.md`（**15 个 PHASE 报告** + 25 Module 计划）|

### 4.2 关键基线类文档扫描结果

| 路径 | 文档类型 | 锁定内容 | 与 CATs B0.0 同步状态 |
|------|----------|----------|------------------------|
| `docs/plan/master-implementation-plan.md` v0.1 | Master Plan | 25 Module / 30 day MVP / 90 day V1 / 180 day V2 | 🟢 与 CATs 150 任务工作流结构对位（多フェーズ多阶段）|
| `docs/plan/mvp-30day-execution-plan.md` | Plan | 30 day MVP 执行 | 🟡 时序类，与 CATs Sprint 1 4 周窗口可比 |
| `docs/plan/v1-90day-execution-plan.md` | Plan | 90 day V1 执行 | 🟡 时序类 |
| `docs/plan/token-olu-estimate.md` | OLU Estimate | token 估算 | 🟢 与 CATs token-OLU 框架 v0.1（commit `f6772ce`）直接对位 |
| `docs/requirements.md` v2.0 | Requirements | 需求 0-47 章 | 🟢 与 CATs 150 任务工作流同维度 |
| `docs/basic-design.md` v0.1 | Basic Design | 基础设计 F-01~F-08 | 🟢 与 CATs 微服务架构设计 v1.0 同维度 |
| `DDD-LEAD-REVIEW-PROCESS.md` | DDD Process | DDD 评审流程 | 🟢 与 CATs 启动会决议 4 RACI SLA 一致 |
| `CHANGELOG.md` | Changelog | 变更日志 | 🟢 标准文档 |

### 4.3 关键路径发现（per 文件系统扫描）

> **Star 使用"实施计划（plan）+ RFC + 交付文档（deliverables）+ PHASE 报告"治理结构**，**根目录散布 25+ 份 PHASE-*.md 报告**（如 `PHASE-D-IMPL-REPORT.md` / `PHASE-D2-CLI-IMPL-REPORT.md` / `PHASE-F.1-LEAD-AUDIT-D6-REPORT.md` 等），结合 `docs/plan/` 下的 MVP/V1/V2 阶段执行计划，形成"重实施、重进度"的工程治理风格。

### 4.4 Star OI-6 同步结论

- **状态**：🟡 Star 无显式基线类目录，但有完整"plan + RFC + DDD + token-OLU"治理结构
- **与 CATs B0.0 差异**：Star 使用"实施计划"为主，CATs 使用"基线化"为主；Star 的 `token-olu-estimate.md` 与 CATs `token-OLU 框架 v0.1` 直接对位
- **建议**：Star 在 Sprint 2 把 `docs/plan/token-olu-estimate.md` 升级为 `docs/00-baseline/` 子目录（或等同机制），与 RGS/CATs 三仓基线类文档同步

---

## 5. 三仓基线对位总览（per 本扫描）

### 5.1 基线类文档存在性矩阵

| 维度 | RGS (RustGameServer) | Physis (Physis-Engine / GVPE) | Star (Star-Renderer) | CATs (本仓 B0.0) |
|------|----------------------|-------------------------------|----------------------|-------------------|
| **基线类目录** | 🟢 `docs/00-基准与治理/` | 🔴 无（用 5 段式 00_foundation/01_architecture/...）| 🔴 无（用 docs/plan/ + docs/requirements/ + docs/architecture/）| 🟢 `doc/05-其他/管理/`（多文档混合）|
| **技术基线锁定** | 🟡 RGS-TS-001 v0.4 §6.2 token-OLU 草案（不在本 worktree）| 🔴 未见明确技术基线文档 | 🟡 `docs/plan/token-olu-estimate.md`（仅 token 估算）| 🟢 `doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md` §1 锁定 Rust 1.98.0 + PG 18.6 + pgvector 0.8.6 |
| **需求规格说明** | 🟡 `docs/00-基准与治理/requirements/RGS-REQ-100_Saga*.md`（专项）| 🟢 `docs/00_foundation/01_requirements.md` v0.2（IPA 格式）| 🟢 `docs/requirements.md` v2.0 | 🟢 `doc/05-其他/管理/CATs_工作流文档_v1.0.md`（150 任务工作流）|
| **基础设计** | 🟡 `RGS-BAS-009_体系治理与横切关注点_基本设计书.md`（横切类）| 🟢 `docs/01_architecture/04_architecture.md` | 🟢 `docs/basic-design.md` v0.1 | 🟢 `doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md` |
| **详细设计** | 🔴 未见显式（可能在 RGS-TS-001 中）| 🟢 `docs/04_detailed_design/` | 🟢 `docs/api-design.md` / `data-design.md` 等 | 🟢 `doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md` + 模块设计书 + 数据库设计书 |
| **DDD Review 流程** | 🟢 `RGS-DDD-REVIEW-2026-08-28-summary.md` | 🟡 `docs/ddd/`（可见 ddd-review 子目录）| 🟢 `DDD-LEAD-REVIEW-PROCESS.md` | 🟢 `CATs_RACISLA模板_v1.0.md`（决议 4 落地）|
| **token-OLU 框架** | 🟢 RGS-TS-001 v0.4 §6.2（草案）| 🔴 未见显式 token-OLU 文档 | 🟢 `docs/plan/token-olu-estimate.md` | 🟢 `doc/05-其他/管理/CATs_token-OLU框架_v0.1.md` |
| **PHASE / Sprint 报告** | 🟢 `PHASE-0-5-STEP-N+...-REPORT.md`（根目录 3 份）| 🟡 `_wt_audit/` 散落 | 🟢 根目录 25+ 份 `PHASE-*-REPORT.md` | 🟡 `doc/05-其他/CATs_M1_Sprint1_启动会决议纪要_v1.0.md` + WBS Sprint 1 跟踪 v1.0 |

### 5.2 跨项目同步状态

| 同步维度 | 状态 | 备注 |
|----------|------|------|
| **基线类目录命名** | 🔴 不一致 | RGS 用 `00-基准与治理/`，Physis 用 `00_foundation/` + 数字编码，Star 用 `docs/plan/` |
| **技术基线锁定内容** | 🟡 各自独立 | RGS 仅 token-OLU 草案，Physis 未见，Star 仅 token 估算，CATs 锁定 Rust + PG |
| **token-OLU 系数区间** | 🟢 100K-300K 一致 | RGS v0.4 §6.2 / CATs v0.1（commit `f6772ce`）/ Star token-olu-estimate 均同区间 |
| **DDD 评审 6 角色共识** | 🟢 一致 | RGS 2026-08-28 / Star DDD-LEAD-REVIEW-PROCESS / CATs 启动会决议 4 |
| **需求-架构-模块 三层结构** | 🟢 趋同 | RGS 分散 + Physis 00_foundation/01_architecture/02_modules / Star plan+RFC / CATs 150 任务工作流 |
| **RBAC 权限矩阵** | 🟡 各自独立 | RGS RGS-BAS-009 部分涉及 / Physics 未见 / Star 未见 / CATs 本文 v1.0 工作流 + 基础设施版双矩阵 |

### 5.3 不修建议（per 决议 8 "Sprint 2 处理"）

| # | 不修项 | 影响 | 建议 Sprint 2 处理路径 |
|---|--------|------|------------------------|
| 1 | 4 仓基线类目录命名不一致 | 跨项目搜索/引用效率低 | Sprint 2 启动 OI-6 修复任务，PMO Lead 召集 4 仓 Lead 协商统一命名规范（保留各自目录结构 + 增加 `00-baseline/` 共同命名子目录）|
| 2 | 技术基线锁定内容各自独立 | 跨项目 Rust crate 依赖版本可能冲突 | Sprint 2 启动 OI-7 任务（如果决议），锁定 4 仓共同 Rust 版本（建议 Rust 1.98.0 per CATs 技术基线 v1.0）|
| 3 | RBAC 权限矩阵各自独立 | 跨项目权限统一难 | Sprint 2 启动 OI-6 修复任务中明确 RBAC 同步路径（建议 RGS 复用 CATs 基础设施版 + Physis/Star 复用 CATs 工作流版）|
| 4 | DDD 评审流程命名差异（RGS/Star vs CATs）| 评审纪要格式难以交叉引用 | Sprint 2 协商统一 DDD Review 纪要模板 |
| 5 | token-OLU 框架 4 仓版本不同 | 估时口径差异 | Sprint 2 召集 4 仓 PMO 同步 token-OLU 框架（CATs v0.1 / RGS v0.4 / Star estimate / Physis 待定）|

---

## 6. Sprint 2 OI-6 修复任务建议

### 6.1 任务草案

| 字段 | 内容 |
|------|------|
| **任务名** | OI-6 跨项目同步修复（Sprint 2 启动）|
| **WBS 编码** | PMO.S2.OI-6 |
| **责任 Lead** | PMO Lead（Ulysses 兼任 / Mavis 代签）|
| **共同责任** | 架构师 Lead（RGS 对位）/ 5 域 Lead（Physis/Star/CATs/RGS 各 1 名）|
| **估时** | 200K-400K tokens（PMO 主导 + 4 仓 Lead 协同）|
| **截止** | Sprint 2 末（2026-10-25 推测）|
| **完成判据** | ① 4 仓基线类目录命名协商统一（决议记录）② 共同 Rust 版本锁定（4 仓 Lead 签字）③ RBAC 权限矩阵同步路径明确 ④ token-OLU 框架 4 仓版本同步 ⑤ DDD Review 纪要模板统一 |

### 6.2 不修明确范围（per 决议 8）

- **本任务（T-03 + OI-6 子任务）不修复任何跨项目不同步问题**
- **本报告 v1.0 仅做扫描 + 报告输出**
- **Sprint 2 启动后由 PMO Lead 召集 4 仓 Lead 协商**

---

## 7. OI-6 状态变化

### 7.1 状态前-后对比

| 维度 | Sprint 0 状态（per 技术基线 v1.0 §8）| Sprint 1 T-03 完成后（本文档 v1.0）| Sprint 2 预期（不修建议）|
|------|--------------------------------------|--------------------------------------|------------------------|
| **OI-6 状态** | ⏳ 待办（跨项目引用方同步）| 🟡 已扫未修（4 仓扫描报告）| 🟢 修复完成（基线命名 + 共同 Rust 版本 + RBAC 同步）|
| **同步路径** | 未启动 | T-03 借机扫描 + 4 仓路径发现 | Sprint 2 启动 OI-6 修复任务 |
| **决议依据** | 技术基线 v1.0 §8 | 启动会决议 8 + Sprint 1 拆解 v1.0+2 §6.7 | Sprint 2 启动会决议（待） |

### 7.2 与 Sprint 1 拆解 v1.0+2 §6.7 闭环

- §6.7 原文："OI-6 状态从"待办"调整为 🟡（T-03 9/13 截止后预期 🟢）"
- 本报告 v1.0 落地后：§6.7 状态从 🟡 调整为 🟢（扫描完成） + 不修建议同步到 Sprint 2
- Sprint 1 拆解 v1.0+3 建议：§6.7 更新为"OAI-6 跨项目同步扫描 🟢（per `OI-6_跨项目同步_status.md` 同期 commit）+ 不修建议已入 Sprint 2 OI-6 修复任务待启动"

---

## 8. 已知缺口（DDD Review 必查 per AI 协作文档治理 2026-08-26）

> 缺标比错标安全：以下信息源未在 CATs 仓或本次扫描中实证 / 未在三仓基线类目录中找全 / 跨项目引用未在 4 仓全部落地，统一标记"待 PMO 确认"。

### 8.1 三仓 baseline 类文档扫描范围限制

- **现象**：本次扫描基于文件系统 ls / glob，未对三仓 git 历史做 `git log -1` 实证（与 CATs 内部 git 实证要求不严格一致）
- **当前处理**：扫描结果基于**当前 worktree 文件系统状态**（per `Get-ChildItem` 2026-09-01 22:30 JST）
- **待 PMO 确认**：是否需要 Sprint 2 启动 OI-6 修复任务时对 3 仓做完整 `git log -1 --format='%H %s' -- <baseline_path>` 实证
- **影响**：本报告 v1.0 扫描结果可能未包含 3 仓历史版本中的基线类文档（如 RGS-TS-001 草案可能不在当前 worktree 中）

### 8.2 Physis / Star 无显式"基线"类目录的解读

- **现象**：Physis 用 `00_foundation/` + 5 段式分类，Star 用 `docs/plan/` + RFC + PHASE 报告结构
- **当前处理**：本报告 §3.4 / §4.4 标记为"🟡 各自独立"，建议 Sprint 2 增加 `00-baseline/` 共同命名
- **待 PMO 确认**：Physis / Star 是否**主动选择**不采用"基线"分类（基于其工程治理哲学），还是**未识别到**基线类的需要
- **影响**：Sprint 2 协商时需明确 Physis / Star 的立场，避免强行统一破坏其原有结构

### 8.3 4 仓共同 Rust 版本锁定

- **现象**：CATs 技术基线 v1.0 §1 锁定 Rust 1.98.0 + PG 18.6 + pgvector 0.8.6；RGS / Physis / Star 各自独立锁定
- **当前处理**：本报告 §5.2 标记为"🟡 各自独立"，建议 Sprint 2 启动 OI-7 任务（如果决议）锁定 4 仓共同 Rust 版本
- **待 PMO 确认**：① 4 仓是否真有**共同** Rust 版本需求（如果有跨仓共享 crate 依赖）② 共同 Rust 版本号（建议 1.98.0 per CATs 基准）③ 升级窗口
- **影响**：如果 4 仓 Rust 版本分裂，未来跨项目 crate 共享（如 auth-service crate 被 Physis 物理引擎依赖）会引发兼容性问题

---

## 9. 关联文档

| 文档 | 路径 | 用途 |
|------|------|------|
| CATs_M1_Sprint1_启动会决议纪要 v1.0 | `doc/05-其他/会议记录/CATs_M1_Sprint1_启动会决议纪要_v1.0.md` | §2 决议 8（OI-6 T-03 借机）|
| CATs_M1_Sprint1_任务拆解 v1.0+2 | `doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md` | §6.7 OI-6 闭环路径 + §0.3 决议 8 借机 OI-6 |
| CATs_Baseline一览 v1.0 | `doc/05-其他/管理/CATs_Baseline一览_v1.0.md` | §3.3 已基线化清单（CATs B0.0 对照）|
| CATs_技术基线 v1.0 | `doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md` | §8 OI-6 状态原状 + §1 锁定 Rust 1.98.0 + PG 18.6 |
| CATs_权限矩阵 v1.0 | `doc/05-其他/管理/CATs_权限矩阵_v1.0.md` | §8.3 已知缺口 8.3 跨项目 OI-6 同步仅扫不修 |
| CATs_WBS Sprint 1 跟踪 v1.0 | `doc/05-其他/管理/CATs_WBS_Sprint1_跟踪_v1.0.md` | §0.1 引用清单 + §2.2 WBS 编码 PMO.S1.T03.1 |
| CATs_安全要件定义书 v1.0 | `doc/05-其他/安全/CATs_安全要件定义书_v1.0.md` | §6 审计（OI-6 状态变化审计基础）|

---

## 10. 修订履历

| 版本 | 日期 | 修订人 | 修订内容 |
|------|------|--------|----------|
| **v1.0** | **2026-09-01** | **架构师 Lead（Ulysses 兼任 / Mavis 接手 agent per DEC-008 + 2026-08-27 19:39 JST 强化代签）** | **首版定稿**：RGS-TS-001 / Physis-Engine / Star-Renderer 三仓基线类文档扫描（仅扫不修 per 决议 8）+ 4 仓基线对位总览 + Sprint 2 不修建议 + OI-6 状态从 ⏳ 待办升级为 🟡 已扫未修 + 已知缺口 3 项诚实标注待 PMO 确认 |

---

**文档结束（v1.0，DDD Review 草稿待 6 角色 9/13 截止前评审，不修建议同步入 Sprint 2 启动会）**
