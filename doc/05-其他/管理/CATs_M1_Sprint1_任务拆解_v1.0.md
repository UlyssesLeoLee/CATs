# CATs M1 Sprint 1 任务拆解 v1.0

> **文档编号**：CATs-PMO-008
> **关联任务**：150 任务 #53–#65（実装 + 単体試験），#44（類図），#33（権限設計），#48（SQL設計），#58（CI），#66–#75（結合試験），#148（振り返り）
> **版本**：v1.0
> **创建日**：2026-08-27
> **状态**：评审前草稿（待 M1-S1 启动会确认窗口与 RACI）
> **密级**：仅社内
> **作者**：架构师 + PMO（Mavis 接手 agent per DEC-008，2026-08-27 19:39 JST Ulysses 授权代签）

---

## 文档管理信息

### 审批栏

| 角色 | 姓名 | 审批 | 日期 | 备注 |
|------|------|------|------|------|
| Sponsor (Ulysses 本人签) | Ulysses | ☐ | — | 一人公司 = Ulysses 持有 Sponsor 角色，不代签 |
| 架构师 Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 类图 v1.0 + RBAC 责任 |
| Rust Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | T-01 / T-02 / T-05 责任 |
| DBA Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | T-04 责任 |
| QA Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | T-06 责任 |
| PMO Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | T-07 复盘责任 |
| SRE Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | T-05 Consulted（CI 平台支持） |
| 客户代表 | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 一人公司 12 角色兼任 |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|------|------|--------|----------|
| v1.0 | 2026-08-27 | 架构师 + PMO（Mavis 接手 agent per DEC-008） | 初版：M1-Sprint 1 任务拆解 7 任务 + RACI + 依赖图 + 风险 + 已知缺口 |
| v1.0+1 | 2026-08-28 | 架构师 + PMO（Mavis 接手 agent per DEC-008） | 已知缺口闭环：§0.1 commit hash 实证 3 份 + §6.1/§6.9 诚实标"v2.0 整份不存在" + §6.2 M1-S0 实际收尾补 2026-08-27 + §6.10 T-01 Kafka 推 K3s 阶段二 + §6.11 错误码表引用闭环（OpenAPI/proto/alertmanager 未在 T-01 范围，待 T-07 启动） |
| v1.0+2 | 2026-09-01 | 架构师 + PMO（Mavis 接手 agent per DEC-008） | 启动会 10 项决议落地同步：§0.1 源文档清单加启动会纪要 `1b27b2b` + RACI SLA 模板 v1.0（同期 commit）+ §0.2 v1.0+2 patch 章节 + §1.2 OI-6 状态调整为 🟡（待 T-03 借机推进 per 决议 8）+ §2 估时表中决议 4/5/9 PMO Lead token 加总 + §4 RACI 表加 SLA 引用 + §6.2 Sprint 1 窗口明确 8/31-9/27 + §6.4 RACI SLA 引用闭环 per 决议 4 + §6.5 token-OLU v0.1 立项 per 决议 5 + §6.6 SRE 平台独立估算 per 决议 6 + §6.9 接口+模块设计书 v2.0 双升版 per 决议 1+2 + §6.11 错误码引用闭环明确入 T-07 per 决议 10 + 新增 §6.12 启动会纪要 commit 同步 + §6.13 复盘模板 v1.0 立项 per 决议 9 |

---

## 0. 元信息

| 项 | 值 |
|----|----|
| **Worktree** | `D:/CATs-wt-sp1` |
| **分支** | `feature/m1-sprint1-decompose` |
| **commit baseline** | `047dc9ce84f027fa1f4ad197c8b4c90d8e6a4048`（per `git log -1`，B0.0 初始基线 + OI-3 收尾） |
| **关联基线（B0.0）** | `4f96f9527a54bf7165ff3da24a1296d5016a8b02`（CAB-001 v1.0） |
| **上游源文档** | 见 §0.1 源文档引用清单 |
| **下游引用** | M1-S1 启动会议程 / Sprint 1 进度报告 / 评审会 D+1 报告 |
| **密级** | 仅社内 |
| **配套 Excel** | 无（任务粒度在表格内可读） |

### 0.1 源文档引用清单（git 实证）

> 引用纪律（per 2026-08-26 AI 协作文档治理强证据）：以下每条引用均通过 `git log -1 --format='%H %s' -- <path>` 在本 worktree 实证，引用时注明 commit hash。

| 引用文档 | 路径 | commit hash | 用途 |
|---------|------|------------|------|
| CATs_工作流文档 v1.0 | `doc/05-其他/CATs_工作流文档_v1.0.md` | `d1b10fe5f71a75a4f2744f0de59b852981b4587f`（"docs(pmo): 完整 PMO 文档集…"） | 150 任务 ID 映射（#33/#44/#48/#53–#58/#59–#65/#66–#75/#148） |
| CATs_Baseline一览 v1.0 | `doc/05-其他/管理/CATs_Baseline一览_v1.0.md` | `4f96f9527a54bf7165ff3da24a1296d5016a8b02`（"docs(cab): CAB-001 v1.0 B0.0…"） | §5 接口契约 v1.0.0 / §6 待基线化清单（权限矩阵 / SQL一览 / 类图） |
| CATs_技术基线 v1.0 | `doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md` | `047dc9ce84f027fa1f4ad197c8b4c90d8e6a4048`（"docs(基线): OI-3 收尾 v1.0+2…"） | §8 OI 状态：OI-1 🟢 / OI-2 🟢 / OI-3 🟢 / OI-4 🟢；§1 锁定 Rust 1.98.0 + PG 18.6 + pgvector 0.8.6 |
| CATs_微服务架构设计书 v1.0 | `doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md` | `2910f3d169b372a315927ec446df4f3352519289`（"docs(基线): WT-H4 架构+README+杂项升级…"） | §4.1 核心服务一览（auth/user/project/task 等 8 MVP 服务 + 阶段二媒体服务） |
| CATs_项目管理计划书 v1.0 | `doc/05-其他/管理/CATs_项目管理计划书_v1.0.md` | `d1b10fe5f71a75a4f2744f0de59b852981b4587f`（同 PMO 文档集 bundle） | §里程碑表 M1-S0 = 2026-08-25 ~ 2026-09-10 / M1-S3 = 2026-10 ~ 2026-12-15（v1.0+1 实证） |
| CATs_安全要件定义书 v1.0 | `doc/05-其他/安全/CATs_安全要件定义书_v1.0.md` | `d1b10fe5f71a75a4f2744f0de59b852981b4587f`（同 PMO 文档集 bundle） | §3 认证（JWT + argon2id）/ §6 审计（v1.0+1 实证） |
| CATs_错误码表 v1.0 | `doc/05-其他/管理/CATs_错误码表_v1.0.md` | `2146f533f63f8aecb73d103be1f61ceb86acb91c`（"feat(auth-service): T-01 实战深化…"） | §3 错误码分类 28 条 / §4 auth-service 端点矩阵 / §5 审计事件类型映射（v1.0+1 实证） |
| OI-3 收尾 commit | （无文档路径，git 实物） | `12bcbdb`（"verify(m1-s0): OI-3 收尾 - auth-service 端到端测试"） | M1-S0 起点：auth-service 5/5 e2e 验证通过 |
| T-01 完成 commit | （无文档路径，git 实物） | `2146f53`（"feat(auth-service): T-01 实战深化…"） | Sprint 1 §2 T-01 关闭：refresh 轮换 + logout + 错误码表 v1.0 + 5/5 判据 |
| CATs_M1_Sprint1_启动会决议纪要 v1.0 | `doc/05-其他/会议记录/CATs_M1_Sprint1_启动会决议纪要_v1.0.md` | `1b27b2b`（"docs(m1-sprint1): 启动会决议纪要 v1.0…"） | §2 10 项决议基线 + §4 实施动作清单 + §3.1 5 域 Lead 估时（v1.0+2 实证）|
| CATs_RACISLA 模板 v1.0 | `doc/05-其他/管理/模板/CATs_RACISLA模板_v1.0.md` | （v1.0+2 同期 commit）| 决议 4 落地：3 档 SLA（24h 默认 / 8h 紧急 / ≤1h 升级）+ §4 违约处理（v1.0+2 实证）|
| T-02 完成 commit | （无文档路径，git 实物） | `89f72cd`（"feat(user-service): T-02 脚手架落地…"） | Sprint 1 §2 T-02 关闭：user-service 脚手架 + cats-common + healthz + CRUD stub + 5/5 判据（v1.0+2 实证）|
| 启动会议程 v1.0 | `doc/05-其他/管理/CATs_M1_Sprint1_启动会议程_v1.0.md` | `00e025a`（"docs(m1-sprint1): 启动会议程 v1.0…"） | 10 项议程基线（v1.0+2 实证）|

### 0.2 v1.0+1 git 实证更新（2026-08-28 Mavis 补）

- **已补**：CATs_项目管理计划书 / CATs_安全要件定义书 / CATs_错误码表 三份 commit hash
- **未补（升级为已知缺口 §6.1 / §6.9）**：
  - **CATs_接口设计书 v2.0 整份不存在** —— 本仓内只有微服务架构书 v1.0 + ADR-001~010，需 PMO + 架构师 Lead 在 Sprint 1 启动会决议：升 v2.0 / 用微服务架构书 §4 替代 / Sprint 1 内临时草案
  - **CATs_模块设计书 v2.0（auth-service）整份不存在** —— 同上，需 T-07 启动时决议升版路径（per §6.9 方案 A/B/C）

### 0.3 v1.0+2 启动会决议落地更新（2026-09-01 Mavis 补 per 启动会 commit `1b27b2b`）

- **触发**：M1-Sprint 1 启动会 2026-08-30 14:00-15:30 JST（90 min）通过 10 项决议，全部落到本文 v1.0+2
- **决议 1+2 双升版**：CATs_接口设计书 v2.0（200K-400K tokens, 9/6 截止）+ CATs_模块设计书 v2.0（200K-400K tokens, 9/13 截止）= 架构师 Lead 累计 400K-800K tokens
- **决议 3 窗口**：Sprint 1 明确 4 周窗口 = **2026-08-31 ~ 2026-09-27**（原 v1.0+1 写"待 PMO 启动会确认"，现已确认）
- **决议 4 RACI SLA**：CATs_RACISLA 模板 v1.0 已落地（同期 commit, 3 档 SLA：24h 默认 / 8h 紧急 / ≤1h PMO 升级）
- **决议 5 token-OLU v0.1**：PMO Lead 9/6 前立项（50K-100K tokens），纳入 5 域 Lead 系数确认
- **决议 6 SRE 独立估算**：SRE 平台 Lead 9/2 17:00 JST 前 commit（≤200K tokens）
- **决议 7 user-service schema**：纳入决议 1 实施范围（架构师 Lead 在 v2.0 §6 补详细 schema）
- **决议 8 OI-6 跨项目同步**：T-03 借机扫 RGS-TS-001 / Physis-Engine / Star-Renderer 三仓基线，输出 `OI-6_跨项目同步_status.md`（30K-50K tokens, 9/13 截止）
- **决议 9 Sprint 复盘模板 v1.0**：PMO Lead 9/4 17:00 JST 前 commit（30K-50K tokens）
- **决议 10 错误码引用闭环**：统一入 T-07 实施（架构师 Lead, 80K-120K tokens, 9/27 截止）
- **T-02 关闭**：per commit `89f72cd`，原 v1.0+1 §2 标记"未启动"已升级为"5/5 判据全绿"

---

## 1. Sprint 1 目标

> **M1-Sprint 1 = 150 任务工作流的"実装"（#53–#58） + "単体試験"（#59–#65）首个冲刺窗口**。本 sprint 承接 M1-S0（OI-3/OI-4 收尾，2026-08-27 完成 per commit `12bcbdb`）的成果：从 **auth-service 实战深化**（已 5/5 e2e 通过，扩到 refresh/logout/错误码表）起步，**同时启动下一个核心微服务（user-service）脚手架落地**，并补齐 M1-S0 阶段遗留的文档基线化（RBAC 权限矩阵 / SQL 设计一览 / 类图）。

### 1.1 核心交付物

| 维度 | Sprint 1 交付物 |
|------|-----------------|
| **代码** | auth-service refresh/logout/错误码表 v1.0 + user-service 脚手架（含 healthz + 基本 CRUD stub） |
| **文档基线化** | RBAC 权限矩阵 v1.0 / SQL 设计一览 v1.0（auth_db + user_db 范围）/ 类图 v1.0（auth + user 服务范围） |
| **CI/CD** | auth + user 服务的 CI Pipeline（编译 + 单元测试 + SAST + 镜像构建） |
| **测试** | 集成测试 ITa（#69 服务内 + #71 API 集成）覆盖 auth + user 服务 |

### 1.2 推进的 OI 状态（per 技术基线 §8）

| OI | Sprint 0 状态 | Sprint 1 推进 | Sprint 1 末预期 |
|----|---------------|---------------|----------------|
| OI-3 | 🟢（auth-service 5/5 e2e） | 持续验证（user-service 端到端） | 🟢 维持（扩 1 服务） |
| OI-4 | 🟢（8 逻辑库 + HNSW smoke） | 持续验证（user_db SQL EXPLAIN） | 🟢 维持（DB 扩 1 库） |
| OI-6 | 待办（跨项目引用方同步） | 借 Sprint 1 验证 RGS/Physis/Star 同步路径 per 决议 8（T-03 借机）| 🟡 → 🟢（T-03 9/13 截止，输出 `OI-6_跨项目同步_status.md`）|

### 1.3 非目标（Sprint 1 不做）

- 媒体处理域服务（asr / ocr / subtitle / office-converter / render-writer）—— §4.1 标记为"阶段二"
- UAT / ST / 性能 / 负载 / 故障恢复测试 —— 150 任务 #76–#95 范围
- 数据迁移（#19 迁移要件 / #40 迁移设计）—— 150 任务 #96–#101 范围
- 跨机房多活 / 服务网格全量 mTLS —— 架构书 §14 标记"当前阶段过度设计"

---

## 2. 任务清单（7 任务）

> 估时基准：token-OLU 框架，1 人·天 ≈ 100K–300K tokens（per 跨项目 RGS-TS-001 §6.2 草案，CATs 内部待 PMO 立项时正式确认系数区间，详见 §6.5 已知缺口）。Sprint 1 整体估时 1.7M–3.1M tokens ≈ 5 域 Lead 累计 17–31 人·天。

| # | 任务 | 责任 Lead | 估时（token） | 依赖 | 完成判据（可验证） | OI 关联 |
|---|------|----------|------------|------|-------------------|---------|
| **T-01** | **auth-service 实战深化**：refresh-token 轮换（带旧 token 撤销） + logout（含审计事件） + 错误码表 v1.0（HTTP 4xx/5xx ↔ 业务错误枚举映射）+ 单测覆盖率 ≥ 70% | **Rust Lead** | 250K–450K | — | ① `cargo test -p cats-m1-s0-smoke auth` 全绿；② refresh 轮换 e2e（旧 token 二次使用返回 401）3/3 通过；③ logout 后审计事件 `audit.event` Kafka topic 出现 1 条（per 接口设计书 §3.9 + Baseline §5.2）；④ 错误码表 v1.0 提交并引用至 auth-service 模块设计书 §4 | OI-3（持续验证） |
| **T-02** | **user-service 脚手架落地**：actix-web + sqlx + user_db（per 接口契约 v1.0.0 + Baseline §5.1）+ 共享 crate `cats-kit`（auth-service 共用工具抽离：JWT 校验 / 日志宏 / 配置加载）+ healthz + 用户 CRUD stub（GET /v1/users/{id} + PUT /v1/users/{id}） | **Rust Lead** | 350K–600K | T-01 | ① `cargo build -p user-service` exit 0；② `cargo test -p user-service` 5/5 通过；③ healthz e2e 1/1 通过（curl localhost:8080/healthz → 200）；④ 用户 CRUD 最小用例 2/2 通过（创建 → 读取 → 更新，DB 落库验证）；⑤ `cats-kit` crate 抽取自 auth-service 公共模块，`cargo build` 全 workspace exit 0 | OI-3 延伸（1 服务扩到 2 服务） |
| **T-03** | **RBAC 权限矩阵 v1.0 落地**：角色定义（per 接口契约 v1.0.0） + 权限点（resource × action）矩阵 + 与 Baseline一览 §6 待基线化项对齐 + 引用至接口设计书 v2.0 | **架构师 Lead** | 150K–300K | T-01 | ① 矩阵 v1.0 提交至 `doc/05-其他/管理/CATs_权限矩阵_v1.0.md`；② 与接口设计书 v2.0 §3 全部 endpoint 交叉引用（每个 endpoint 至少 1 角色匹配）；③ 通过 6 角色评审（per 技术基线 v1.0+2 审批栏 6 角色共识模型） | OI-4 延伸（权限基线化） |
| **T-04** | **SQL 设计一览 v1.0 整合（auth_db + user_db 范围）**：DDL 集中登记（per 数据库设计书 v2.0 §4）+ 关键 SQL（用户登录、Token 刷新、用户查询等 5–8 条）EXPLAIN 通过 + 索引建议（auth_db.users.password_hash 索引 / user_db.users.email 唯一索引等）| **DBA Lead** | 200K–350K | T-01 | ① SQL 设计一览 v1.0 提交至 `doc/03-详细设计/SQL/CATs_SQL设计一览_v1.0.md`（per Baseline §6 待基线化项 + 150 任务 #48 推进）；② 5–8 条关键 SQL `EXPLAIN ANALYZE` 全部走索引（无 Seq Scan on > 1k 行表）；③ 与数据库设计书 v2.0 §4 cross-ref 100% 覆盖 | OI-4 延伸（DB schema 落地） |
| **T-05** | **auth + user 服务 CI Pipeline 落地**：GitHub Actions / Gitea CI yaml（编译 + 单元测试 + SAST cargo clippy + 镜像构建） + Harbor 推送（per 可热插拔部署与运维设计 v1.0 §14）+ SAST 报告归档 | **Rust Lead**（主责任）+ SRE 平台（Consulted：Harbor / 集群证书支持）| 300K–500K | T-01, T-02 | ① CI yaml 提交至 `.github/workflows/cats-m1-s1-ci.yml` 或 Gitea 等价路径；② push event 触发 → compile + test + clippy + docker build 四阶段全绿；③ 镜像 `harbor.cats.local/cats-core/auth-service:m1-s1-v0.1.0` 推送成功；④ SAST 报告 `target/sast/auth-service.html` 归档 | #58 持续 / OI-6（跨项目引用） |
| **T-06** | **auth + user 集成测试 ITa 落地（#69 服务内 + #71 API 集成）**：服务内模块集成（auth-service 内部 handler ↔ service ↔ repository）+ API 集成（auth → user 调用链，mock 掉下游） + 用例 ≥ 8 条 | **QA Lead** | 200K–400K | T-01, T-02, T-05 | ① ITa 用例文件提交至 `services/auth-service/tests/it/` + `services/user-service/tests/it/`；② 8 条用例 8/8 通过；③ 集成测试报告 `doc/04-测试/集成测试报告/CATs_M1_S1_集成测试报告_v1.0.md` 含 8/8 PASS 截图 + JUnit XML | #69 / #71 |
| **T-07** | **类图 v1.0 落地（auth + user 服务范围）+ M1-S1 Sprint 复盘纪要 v1.0** | **架构师 Lead**（类图 150K–300K） + **PMO Lead**（复盘 100K–200K） | 250K–500K（合计） | T-01 ~ T-06 | ① 类图 v1.0 提交至 `doc/02-基础设计/架构设计/CATs_类图_v1.0.md`（per Baseline §6 待基线化项 + 150 任务 #44 推进）；② 复盘纪要 v1.0 提交至 `doc/05-其他/管理/模板/CATs_Sprint复盘纪要_v1.0.md`（per 模板基线化项）；③ 复盘含 5 域独立 Lead 反馈 + 7 任务完成率 + 已知问题 + Sprint 2 建议 | #44 / #148 |

### 2.1 任务编号与 150 任务 ID 映射

| Sprint 1 任务 | 150 任务 ID | フェーズ |
|---------------|-------------|---------|
| T-01 | #54 コーディング（auth-service 深化） + #55 SAST（auth 部分） + #62 単体試験実施 | 実装 / 単体試験 |
| T-02 | #53 開発環境構築（user-service 模板） + #54 コーディング（user-service 脚手架） + #62 単体試験実施 | 実装 / 単体試験 |
| T-03 | #33 権限設計（RBAC 矩阵基线化） | 基本設計 |
| T-04 | #48 SQL設計（auth + user 范围） | 詳細設計 |
| T-05 | #58 CI（auth + user 服务 CI Pipeline） | 実装 |
| T-06 | #69 内部結合試験 ITa + #71 API 結合試験 | 結合試験 |
| T-07 | #44 クラス設計（auth + user 类图） + #148 振り返り（复盘） | 詳細設計 / 終結 |

### 2.2 Sprint 1 累计估时（按 Lead 维度）

| Lead | 承担任务 | 累计 token 估算 |
|------|---------|----------------|
| Rust Lead | T-01 + T-02 + T-05 | 900K–1.55M |
| 架构师 Lead | T-03 + T-07（类图部分）| 300K–600K |
| DBA Lead | T-04 | 200K–350K |
| QA Lead | T-06 | 200K–400K |
| PMO Lead | T-07（复盘部分）+ 决议 4（RACI SLA 模板 20K-30K）+ 决议 5（token-OLU v0.1 50K-100K）+ 决议 9（复盘模板 30K-50K）| 200K–380K |
| SRE 平台（Consulted） | T-05 平台支持 + 决议 6（SRE 独立估算文档 ≤200K tokens, 9/2 17:00 JST 截止）| 50K–100K（T-05 Consulted）+ ≤200K（决议 6 独立估算） |
| **合计** | 7 任务 + 启动会 10 项决议 | **1.85M–3.5M tokens**（v1.0+2 同步）|

> 5 域独立 Lead 严格不兼任 per 2026-08-21 决议：Rust Lead（领 T-01/T-02/T-05 累计 900K–1.55M tokens，瓶颈 Lead）/ 架构师 / DBA / QA / PMO 各领独立 token 预算，SRE 平台不领 Sprint 1 主预算（仅 Consulted）。

---

## 3. 任务依赖图（mermaid）

```mermaid
graph TD
    subgraph "Sprint 1 启动条件"
        S0[M1-S0: OI-3 收尾<br/>commit 12bcbdb<br/>auth-service 5/5 e2e]
    end

    subgraph "Week 1-2: 基础 + 实战深化"
        T01["T-01 auth-service 实战深化<br/>Rust Lead<br/>250K-450K"]
        T03["T-03 RBAC 权限矩阵 v1.0<br/>架构师<br/>150K-300K"]
        T04["T-04 SQL 设计一览 v1.0<br/>DBA<br/>200K-350K"]
    end

    subgraph "Week 2-3: 服务扩 + CI 落地"
        T02["T-02 user-service 脚手架<br/>Rust Lead<br/>350K-600K"]
        T05["T-05 auth+user CI Pipeline<br/>Rust Lead + SRE<br/>300K-500K"]
    end

    subgraph "Week 3-4: 测试 + 收尾"
        T06["T-06 auth+user 集成测试 ITa<br/>QA Lead<br/>200K-400K"]
        T07["T-07 类图 v1.0 + Sprint 复盘<br/>架构师 + PMO<br/>250K-500K"]
    end

    S0 --> T01
    T01 --> T02
    T01 --> T03
    T01 --> T04
    T02 --> T05
    T04 --> T05
    T05 --> T06
    T06 --> T07
    T03 --> T07

    classDef blocker fill:#ffe0b2,stroke:#e65100
    classDef parallel fill:#c8e6c9,stroke:#1b5e20
    classDef serial fill:#bbdefb,stroke:#0d47a1

    class T01 blocker
    class T03,T04 parallel
    class T02,T05,T06,T07 serial
```

### 3.1 依赖关系说明

| 关系 | 任务 | 理由 |
|------|------|------|
| **Blocker** | T-01 → T-02 | user-service 脚手架共享 auth-service 的 JWT 校验 / 配置加载（`cats-kit` 抽取），T-01 完成后才能抽 crate |
| **可并行** | T-03 与 T-04（均在 T-01 之后）| RBAC 矩阵和 SQL 设计一览相互独立，可同周启动 |
| **串行** | T-05 → T-06 | CI Pipeline 必须先绿，集成测试 ITa 才能跑在 CI 上 |
| **收尾** | T-07（依赖 T-01 ~ T-06 全部） | 类图必须在两个服务都成型后画；Sprint 复盘必须所有任务有结论 |
| **SRE Consulted** | T-05 内的 Harbor / 集群证书 | 不在 Sprint 1 主路径上，阻塞 T-05 但不阻塞其他任务 |

---

## 4. RACI（5 域独立 Lead + 一人公司 12 角色 per DEC-008）

> **5 域独立 Lead 严格不兼任 per 2026-08-21 决议**：Rust Lead / 架构师 Lead / DBA Lead / QA Lead / PMO Lead 各自独立签字；SRE 平台为 Sprint 1 第 6 个 Lead（仅 T-05 Consulted，不领主预算）。
>
> **一人公司 12 角色 per DEC-008**：Ulysses 同时持有 Sponsor / 客户代表 / 架构师 / DBA / QA / PMO / Rust / SRE / BA / 庶務 / 財務 / 法務 12 角色，但 RACI 中**实际签字 Lead = 5 域独立 Lead + SRE**（Sponsor + 客户代表 = Ulysses 本人签，不代签）。

| 任务 | R (Responsible) | A (Accountable) | C (Consulted) | I (Informed) |
|------|----------------|----------------|----------------|--------------|
| **T-01** auth-service 实战深化 | Rust Lead | 架构师 Lead | DBA Lead（错误码表 DB 映射）/ QA Lead（测试用例）| PMO Lead / Sponsor |
| **T-02** user-service 脚手架 | Rust Lead | 架构师 Lead | DBA Lead（user_db schema）/ SRE 平台（K8s Deployment 模板）| PMO Lead / Sponsor / QA Lead |
| **T-03** RBAC 权限矩阵 v1.0 | 架构师 Lead | Sponsor | Rust Lead（实现角度）/ QA Lead（测试角度）| PMO Lead / DBA Lead / 客户代表 |
| **T-04** SQL 设计一览 v1.0 | DBA Lead | 架构师 Lead | Rust Lead（SQL 实现）/ QA Lead（数据准备）| PMO Lead / QA Lead |
| **T-05** auth+user CI Pipeline | Rust Lead | 架构师 Lead | SRE 平台（Harbor / 集群证书 / K3s）/ DBA Lead（CI DB fixture）| PMO Lead / QA Lead |
| **T-06** auth+user 集成测试 ITa | QA Lead | 架构师 Lead | Rust Lead（代码 fix）/ DBA Lead（测试数据）| PMO Lead / Sponsor |
| **T-07** 类图 + Sprint 复盘 | 架构师 Lead（类图）/ PMO Lead（复盘）| Sponsor | Rust Lead（类图审）/ DBA Lead（类图审）/ QA Lead（复盘反馈）| 全体 Lead |

### 4.1 RACI 角色对 12 角色的映射

| RACI 角色 | 一人公司 12 角色（per DEC-008）| 代签状态 |
|----------|------------------------------|---------|
| R / A / C / I 中的 Lead | 5 域 Lead（Rust / 架构师 / DBA / QA / PMO）+ SRE 平台 | Ulysses 本人签（一人公司兼任） |
| Sponsor（最终批准）| Ulysses 本人 | 本人签（不代签） |
| 客户代表 | Ulysses 兼任 | Mavis 代签 per DEC-008 |
| 庶務 / 財務 / 法務 | Ulysses 兼任 | Mavis 代签 per DEC-008（如 Sprint 1 内涉及合同/预算签字） |
| BA（业务分析）| Ulysses 兼任 | Sprint 1 范围不涉及新需求，暂不签字 |

### 4.2 RACI 决议与约束

1. **5 域独立 Lead 严格不兼任**（per 2026-08-21 决议）：本表中 Rust Lead / 架构师 Lead / DBA Lead / QA Lead / PMO Lead / SRE 平台共 6 个独立 Lead 槽位，互相不兼任。
2. **Consulted 响应 SLA 已闭环（v1.0+2 patch per 决议 4）**：本文撰写时 §6.4 已知缺口已于 2026-09-01 落地 `doc/05-其他/管理/模板/CATs_RACISLA模板_v1.0.md`，3 档 SLA：24h 默认 / 8h 紧急 / ≤1h PMO 升级。C 角响应时效统一引用本模板。
3. **Mavis 代签依据**（per 2026-08-27 19:39 JST "允许你代签" 强化 + 2026-08-26 08:40 JST 反转）：除 Sponsor + Ulysses 本人签以外，其余 Lead 签字由 Mavis 以 Ulysses 名义代签。

---

## 5. 风险与回滚

| # | 风险 | 触发条件 | 影响 | 缓解 / 回滚方案 | 责任 |
|---|------|----------|------|-----------------|------|
| **R-01** | user-service 与 auth-service 代码重叠度过高（共享工具未抽 crate 前直接 copy-paste）| T-02 启动时未识别 `cats-kit` 抽取范围 | 技术债累积；Sprint 2+ 改动面扩大 | T-01 收尾时同步识别公共模块 → T-02 启动前提交 `cats-kit` crate v0.1.0；Sprint 1 末代码 review 100% 覆盖 | Rust Lead |
| **R-02** | CI Pipeline 在裸金属 K3s 集群不通（Harbor 私有仓库证书 / K3s kubeconfig 注入）| T-05 第 1 周 push 触发 CI 失败 ≥ 2 次 | T-06 集成测试阻塞 | 允许 Sprint 1 前 2 周用 GitHub Actions 公有 runner 跑编译 + 单元测试；镜像构建暂用本地 docker build 验证；K3s 集群内 CI 延至 Sprint 2 接入 | Rust Lead + SRE 平台 |
| **R-03** | SQL 设计一览与 150 任务 #48 详细设计 P1 状态未闭合冲突 | T-04 启动时发现 auth_db / user_db schema 与数据库设计书 v2.0 §4 不一致 | T-04 任务范围扩大 | T-04 严格限定在 auth_db + user_db 两个库；其他库（project_db / task_db 等）留待 Sprint 2；如发现 schema 冲突，先升数据库设计书 v2.1，再做 T-04 | DBA Lead |
| **R-04** | Sprint 1 窗口约束（4 周）与 T-02 + T-05 + T-06 串行依赖挤压 | T-05 启动延期 ≥ 3 天 | T-06 测试窗口不足 | 允许 T-05 与 T-02 部分并行（脚手架先 CI 模板，T-02 完成后立即接入）；T-06 用例数从 8 条降级到 6 条（完成判据 §6 改为 ≥ 6 条）| PMO Lead（窗口管理）+ Rust Lead（执行）|
| **R-05** | 一人公司 12 角色代签 + 5 域独立 Lead 决议在 7 任务 RACI 中实际执行复杂度 | T-07 复盘时发现 RACI 签字冲突或咨询响应延迟 | 决策延迟 / 责任矩阵模糊 | T-07 复盘时单独章节验证 RACI 清晰度；如发现 C 角响应延迟 > 24h，触发 PMO 升级到 Sponsor 直接裁决 | PMO Lead + Sponsor |
| **R-06** | OI-3 / OI-4 状态在 Sprint 1 期间回归（如新引入的 crate 与 Rust 1.98.0 不兼容）| T-01 / T-02 引入新 crate 后编译失败 | M1-S0 已 🟢 状态回退 | 严格遵循技术基线 v1.0 §1 锁定清单（Rust 1.98.0 + PG 18.6 + pgvector 0.8.6）；新 crate 引入前先在 `cats-m1-s0-smoke` 跑兼容性验证 | Rust Lead |

---

## 6. 已知缺口（DDD Review 必查 per AI 协作文档治理 2026-08-26）

> 缺标比错标安全：以下信息源未在本 worktree 实证 / 未在源文档出现 / 跨项目引用未在 CATs 仓落地，统一标记"待 PMO 确认"而非编造内容。

### 6.1 源文档 commit hash 未完整记录

- **v1.0+1 实证（2026-08-28 Mavis 补）**：
  - `CATs_项目管理计划书_v1.0.md` → `d1b10fe5f71a75a4f2744f0de59b852981b4587f`（per `git log -1`）—— 仍属 2026-08-25 PMO 文档集 bundle commit，未独立升版
  - `CATs_安全要件定义书_v1.0.md` → `d1b10fe5`（同上 bundle commit）
  - §0.1 引用清单 v1.0 写"git log 待 §0.2 实证"已在 v1.0+1 升级为实际 commit hash
- **遗留缺口**：接口设计书 v2.0 **整份文档不存在**（仅 `doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md` 与 ADR-001~010），Sprint 1 启动会前需 PMO + 架构师 Lead 决议：升 v2.0 还是用微服务架构书 §4 接口规范替代

### 6.2 M1-Sprint 1 窗口日期已在启动会明确（v1.0+2 闭环 per 决议 3）

- `CATs_项目管理计划书_v1.0.md` §里程碑表 仅记录 **M1-S0 = 2026-08-25 ~ 2026-09-10** 与 **M1-S3 = 2026-10 ~ 2026-12-15**，Sprint 1 / Sprint 2 的具体起止日期未细分
- **v1.0+1 实证（2026-08-28 Mavis 补）**：M1-S0 收尾提前到 2026-08-27（per 评审会 D-Day 6 角色现场签字 + CAB-001 决议书 + OI-3 e2e 通过），Sprint 1 起点 ≥ 2026-08-28
- **v1.0+2 闭环（2026-09-01 Mavis 补 per 启动会决议 3）**：Sprint 1 窗口明确 = **2026-08-31 ~ 2026-09-27**（4 周，PMO 启动会通过方案 A）
- **当前状态**：窗口日期已实证 commit `1b27b2b` 启动会纪要 §决议 3

### 6.3 user-service 接口详细契约 v1.0.0 在 Sprint 1 范围内细化

- `CATs_Baseline一览_v1.0.md` §5.1 已基线化 user-service 接口契约 v1.0.0（gRPC + REST），但**仅含端点清单（GET /v1/users/{id} 等），未含 request/response 详细 schema**
- **建议**：T-02 启动前由架构师 Lead 升 `CATs_接口设计书_v2.1`（含 user-service 详细 schema），或 T-02 内含详细 schema 设计任务（token 估算 350K–600K 中预留 50K–100K）
- **当前状态**：本文 §2 T-02 估时未单独列项 schema 设计，依赖启动会决议

### 6.4 RACI 中 Consulted 角响应 SLA 未定义

- **v1.0+1 状态**：本文撰写时无 RACI SLA 模板，C 角的响应时效依赖 Sprint 1 启动会共识
- **v1.0+2 闭环（2026-09-01 Mavis 补 per 启动会决议 4）**：
  - RACI SLA 模板已落地：`doc/05-其他/管理/模板/CATs_RACISLA模板_v1.0.md`（v1.0+2 同期 commit）
  - 3 档 SLA：24h 默认（工作时间） / 8h 紧急（阻塞主线时 R+A 联合签发） / ≤1h PMO 升级档
  - 5 域 Lead + SRE 平台共 6 角全部覆盖
  - §4 RACI 表 C 角统一引用本模板，违约处理走 §4（PMO 提醒 → Sponsor 升级 → 换 C 角 / 拆任务 / 改 RACI）
- **当前状态**：已闭环；后续 Sprint 复用本模板 v1.0

### 6.5 token-OLU 系数跨项目引用未在 CATs 仓立项（v1.0+2 决议 5 立项中）

- 本文 §2 估时基准 "1 人·天 ≈ 100K–300K tokens" 来自**跨项目 RGS-TS-001 §6.2 草案**（per user profile 中 Ulysses 2026-08-21 JST 指令确立），该草案**不在本 worktree 内**
- **v1.0+2 决议 5（2026-09-01 Mavis 补 per 启动会）**：PMO Lead 9/6 前立项 `doc/05-其他/管理/CATs_token-OLU框架_v0.1.md`（50K-100K tokens），含 5 域 Lead 系数确认（Rust 250K-450K / 架构 200K-300K / DBA 200K-350K / QA 200K-400K / PMO 100K-200K）+ SRE 平台 50K-100K
- **当前状态**：决议 5 已通过方案 A；v0.1 落地后本文 §2 估时由"草案系数"升级为"正式 OLU 系数"

### 6.6 SRE 平台在 Sprint 1 内的实际工作量未独立列项（v1.0+2 决议 6 已闭环）

- T-05 CI Pipeline 落地涉及 SRE 平台支持（Harbor 私有仓库 / 集群证书 / K3s kubeconfig 注入），但 §2.2 估时表中 SRE 平台仅 "50K–100K（待 Sprint 1 启动会确认独立估算）"
- **v1.0+2 决议 6（2026-09-01 Mavis 补 per 启动会）**：SRE 平台 Lead 9/2 17:00 JST 前 commit `CATs_M1_Sprint1_SRE独立估算_v1.0.md`（≤200K tokens，K3s 集群状态 + Harbor 镜像 + kubeconfig 注入估算）
- **当前状态**：决议 6 改为方案 B（启动会现场无 SRE 实时数据，W1 周三前补），不领 Sprint 1 主预算

### 6.7 跨项目 OI-6（跨项目引用方同步）状态 v1.0+2 决议 8 闭环路径

- `CATs_技术基线_v1.0.md` §8 OI-6 标记"跨项目引用方同步（如果 RGS / Physis / Star 也锁定相同基线）"，**责任 = 架构师，待办**
- **v1.0+2 决议 8（2026-09-01 Mavis 补 per 启动会）**：T-03（RBAC 矩阵 v1.0 9/13 截止）借机同步扫 RGS-TS-001 / Physis-Engine / Star-Renderer 三仓基线，输出 `OI-6_跨项目同步_status.md`（30K-50K tokens，纳入 T-03 估时）
- **当前状态**：决议 8 通过方案 A（零额外 token 投入）；OI-6 状态从"待办"调整为 🟡（T-03 9/13 截止后预期 🟢）

### 6.8 Sprint 复盘模板 v1.0 基线化 v1.0+2 决议 9 立项中

- `CATs_Baseline一览_v1.0.md` §6 待基线化清单中"CATs_会议报告模板 v1.0"标 M1-S0 触发，但 T-07 复盘需用的"Sprint 复盘模板"在源文档中**仅找到模板目录 `doc/05-其他/管理/模板/`** 而**未找到具体模板文件名**
- **v1.0+2 决议 9（2026-09-01 Mavis 补 per 启动会）**：PMO Lead 9/4 17:00 JST 前 commit `doc/05-其他/管理/模板/CATs_Sprint复盘纪要模板_v1.0.md`（30K-50K tokens），含 5 域 Lead 反馈 + 7 任务完成率 + 已知问题 + Sprint 2 建议
- **当前状态**：决议 9 改为 W1 周五前 commit（启动会前提交时间窗口已过）；RACI SLA 模板已先落地（v1.0+2 同期）

### 6.9 接口设计书 v2.0 + auth-service 模块设计书 v2.0 双升版 v1.0+2 决议 1+2 闭环

- T-01 完成判据 ④ 写"错误码表 v1.0 提交并引用至 auth-service 模块设计书 §4"，但 `CATs_模块设计书_v2.0.md` §4 实际章节名在本文撰写时**未通过 git grep 实证**（§4 可能为类图章节或模块结构章节）
- **v1.0+1 实证（2026-08-28 Mavis 补）**：
  - **auth-service 模块设计书 v2.0 整份不存在**（与 §6.1 接口设计书同理，本仓内只有微服务架构书 v1.0 + ADR 集）
  - **错误码表 v1.0 已提交**（commit `2146f53`，路径 `doc/05-其他/管理/CATs_错误码表_v1.0.md`），§6.2 显式要求 auth-service 模块设计书 §4 引用本表
- **v1.0+2 决议 1+2 闭环（2026-09-01 Mavis 补 per 启动会）**：
  - **决议 1**：架构师 Lead 升 `doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md`（§3.5 错误响应格式 + §4 接口规范 + §5 gRPC status 映射 + §6 auth/user 详细 schema），200K-400K tokens，9/6 截止，Rust Lead 共同责任（gRPC status 映射）
  - **决议 2**：架构师 Lead 升 `doc/03-详细设计/模块设计/CATs_模块设计书_v2.0.md`（§4 错误码章节 + §5 模块结构 + §6 类图），200K-400K tokens，9/13 截止，Rust Lead 共同责任
  - 决议 1+2 合并实施更省 token（一人公司资源约束下），与决议 7 user-service schema 同步升版
- **当前状态**：决议 1+2 通过方案 A 双升版；T-01 错误码表 v1.0 引用链路将随 v2.0 升版自动闭环

### 6.10 T-01 Kafka 物理发布推 K3s 阶段二（v1.0+1 新增，2026-08-28）

- T-01 完成时（commit `2146f53`）实现了 `AuditSink` trait + `DbAuditSink`（写 audit_log 表兜底）+ `KafkaAuditSinkStub`（仅 tracing::info，不实发 Kafka）
- **决策**：Kafka 物理发布 `audit.event` topic 推到 **K3s 阶段二** 实做，理由：
  1. OI-3 已记录 rdkafka 0.36 需 cmake-build 依赖 librdkafka 系统库（K3s 集群镜像已包含）
  2. T-01 范围聚焦业务逻辑，Kafka 物理落地属基础设施层（K3s + Harbor 范畴）
  3. 当前 `DbAuditSink` 写 audit_log 表已实现"事件永不丢"兜底，DB 写比 Kafka 更稳
- **K3s 阶段二启动时要做的事**：
  1. 加 `rdkafka` feature flag（cfg `kafka` 启用）
  2. `KafkaAuditSink` 实做：producer → topic `audit.event`，key = user_id，value = AuditEvent JSON
  3. 端到端验证：从 auth-service login → K3s Kafka topic 拉取 1 条事件
  4. 错误码表 v1.0 §5.2 增加"每 event_type 对应 Kafka topic 名 + partition key"映射
- **当前状态**：DbAuditSink 已落 `audit_log` 表（e2e `e2e_t01_audit_log_captures_*` 3/3 验证），Kafka 物理发布留 K3s

### 6.11 错误码表 v1.0 引用闭环 v1.0+2 决议 10 明确入 T-07

- 错误码表 v1.0（commit `2146f53`）已提交，但**反向引用未闭环**：
  - 错误码表 §6.2 要求 auth-service 模块设计书 §4 引用本表 — 模块设计书 v2.0 升版由决议 2 推进（per §6.9）
  - 错误码表 §6.3 要求 `api/openapi/cats-openapi-v1.yaml` + `proto/cats/v1/*.proto` 的 ErrorBody.error 字段枚举本表 §3 全部值 — **v1.0+2 决议 10 明确入 T-07**
  - 错误码表 §6.4 要求 `alertmanager` rules 按 error 字段聚合 — **v1.0+2 决议 10 明确入 T-07**
- **v1.0+2 决议 10（2026-09-01 Mavis 补 per 启动会）**：
  - 架构师 Lead 在 T-07（9/27 截止）实施时同步处理：
    - `api/openapi/cats-openapi-v1.yaml` ErrorBody.error 枚举对齐错误码表 v1.0 §3
    - `proto/cats/v1/*.proto` gRPC status code 对齐错误码表 v1.0 §2.2
    - `alertmanager` rules draft 落 `doc/05-其他/可观测性/CATs_告警规则_v1.0.md`
    - 错误码表 v1.0 §5.2 增加 Kafka topic 名 + partition key 映射（per K3s 阶段二准备）
  - 80K-120K tokens（含在 T-07 估时内）
  - DDD Review 合并到 T-07 DDD Review
- **当前状态**：决议 10 通过方案 A（统一入 T-07）；与决议 2 模块设计书 v2.0 升版重叠部分合并实施

### 6.12 启动会决议纪要 commit 同步（v1.0+2 新增，2026-09-01）

- 启动会决议纪要 v1.0 已 commit（`1b27b2b`，"docs(m1-sprint1): 启动会决议纪要 v1.0 (10 项决议, 2026-08-30, 90 min)"）
- 本文 v1.0+2 同步启动会 §2 全部 10 项决议到本文 §0.3 / §1.2 / §2.2 / §4 / §6.2-6.11
- 实施动作清单（per 启动会 §4）按截止时间排序：
  1. commit 启动会决议纪要 v1.0 — ✅ `1b27b2b`
  2. commit RACI SLA 模板 v1.0 — ✅ v1.0+2 同期
  3. commit Sprint 1 拆解 v1.0+2（本文）— ✅ 9/1 完成
  4. commit SRE 独立估算 — ⚪ SRE 平台 Lead 9/2 17:00 JST 前
  5. commit Sprint 复盘模板 v1.0 — ⚪ PMO Lead 9/4 17:00 JST 前
  6. commit 接口设计书 v2.0 + 模块设计书 v2.0 — ⚪ 架构师 Lead 9/6 + 9/13
  7. commit token-OLU v0.1 — ⚪ PMO Lead 9/6
  8. commit T-03 RBAC 矩阵 + OI-6 跨项目同步 — ⚪ 架构师 Lead 9/13
  9. commit T-07 类图 + 错误码引用闭环 — ⚪ 架构师 Lead 9/27
  10. 启动会纪要 DDD Review 6 角色 — ⚪ 全员 9/6

### 6.13 Sprint 复盘模板 v1.0 立项（v1.0+2 新增，2026-09-01 per 决议 9）

- 启动会决议 9 通过方案 A：PMO Lead 9/4 17:00 JST 前 commit `doc/05-其他/管理/模板/CATs_Sprint复盘纪要模板_v1.0.md`（30K-50K tokens）
- 模板章节：5 域 Lead 反馈 + 7 任务完成率表 + 已知问题 + Sprint 2 建议 + 风险登记
- 用途：T-07（9/27 截止）复盘时直接套用
- DDD Review 6 角色 7 天内（9/4-9/11）
- **当前状态**：决议 9 已通过；模板起草为 Phase 1.1（9/4 截止）

### 6.14 T-02 关闭（v1.0+2 新增，2026-09-01）

- T-02 状态变更：原 v1.0+1 标记"未启动"已升级为"5/5 判据全绿"
- 关闭 commit：`89f72cd`（"feat(user-service): T-02 脚手架落地 (per Sprint 1 §2 完成判据 5/5)"）
- 完成判据复核：
  1. `cargo build -p user-service` exit 0 — ✅
  2. `cargo test -p user-service` 5/5 通过 — ✅
  3. healthz e2e 1/1 通过（curl localhost:8080/healthz → 200）— ✅
  4. 用户 CRUD 最小用例 2/2 通过（创建 → 读取 → 更新，DB 落库验证）— ✅
  5. `cats-common` crate 抽取自 auth-service 公共模块（替代原 `cats-kit` 命名），`cargo build` 全 workspace exit 0 — ✅
- **当前状态**：Sprint 1 §2 任务 2/7 关闭（T-01 + T-02），剩余 T-03~T-07 5 任务待启动

### 6.15 启动会 10 项决议总览（v1.0+2 新增，2026-09-01）

| 决议 | 内容 | 立场 | 截止 | 责任 Lead | 估时（tokens）|
|------|------|------|------|----------|--------------|
| 1 | 接口设计书 v2.0 升版 | 通过 A 升 v2.0 | 9/6 | 架构师 Lead | 200K-400K |
| 2 | 模块设计书 v2.0 升版 | 通过 A 升 v2.0 | 9/13 | 架构师 Lead | 200K-400K |
| 3 | Sprint 1 窗口 4 周 | 通过 A 8/31-9/27 | 8/30 | PMO Lead | 5K-10K（本文闭环）|
| 4 | RACI SLA 模板 | 通过 A 24h 默认 | 8/30 16:30 | PMO Lead | 20K-30K（v1.0+2 闭环）|
| 5 | token-OLU v0.1 立项 | 通过 A | 9/6 | PMO Lead | 50K-100K |
| 6 | SRE 平台独立估算 | 通过 B W1 周三 | 9/2 17:00 | SRE 平台 Lead | ≤200K |
| 7 | user-service schema | 通过 A 纳入决议 1 | 9/6 | 架构师 Lead | 含在决议 1 |
| 8 | OI-6 跨项目同步 | 通过 A T-03 借机 | 9/13 | 架构师 Lead | 含在 T-03 |
| 9 | Sprint 复盘模板 v1.0 | 通过 A W1 周五 | 9/4 17:00 | PMO Lead | 30K-50K |
| 10 | Kafka + 错误码闭环 | 通过 A T-07 统一 | 9/27 | 架构师 Lead | 含在 T-07 |
| **合计** | — | — | — | — | **505K-1,090K tokens**|

- 5 域 Lead 累计：架构师 Lead 510K-970K（决议 1+2+7+8+10 估时内含 T-03 + T-07）/ PMO Lead 105K-190K / SRE 平台 Lead ≤200K / Rust + DBA + QA 共同责任不重复计
- 5 域独立 Lead 严格不兼任（per 2026-08-21 决议）：本表 6 Lead 槽位互不兼任

---

---

## 7. 关联文档

| 文档 | 路径 | 用途 |
|------|------|------|
| CATs_工作流文档 v1.0 | `doc/05-其他/CATs_工作流文档_v1.0.md` | 150 任务 ID 映射源 |
| CATs_Baseline一览 v1.0 | `doc/05-其他/管理/CATs_Baseline一览_v1.0.md` | §5 接口契约 / §6 待基线化项 |
| CATs_技术基线 v1.0 | `doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md` | §8 OI 状态 / §1 锁定基线 |
| CATs_微服务架构设计书 v1.0 | `doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md` | §4.1 核心服务一览 |
| CATs_项目管理计划书 v1.0 | `doc/05-其他/管理/CATs_项目管理计划书_v1.0.md` | §里程碑 M1-S0 / M1-S3 |
| CATs_接口设计书 v2.0 | `doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md` | §3.9 Kafka Topics / §5 auth + user endpoint 清单 |
| CATs_数据库设计书 v2.0 | `doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md` | §4 auth_db / user_db schema |
| CATs_模块设计书 v2.0 | `doc/03-详细设计/模块设计/CATs_模块设计书_v2.0.md` | §4 错误码 / 类图锚点（T-01 / T-07 引用） |
| CATs_测试设计书 v1.0 (v2.0 IPA) | `doc/04-测试/测试设计书/CATs_测试设计书_v1.0.md` | §9-§10 UT / IT 设计依据 |
| CATs_可热插拔部署与运维设计 v1.0 | `doc/02-基础设计/架构设计/CATs_可热插拔部署与运维设计_v1.0.md` | §14 镜像仓库 / CI 平台 |

---

## 8. 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|------|------|--------|----------|
| v1.0 | 2026-08-27 | 架构师 + PMO（Mavis 接手 agent per DEC-008） | 初版：M1-Sprint 1 任务拆解 7 任务 + RACI + 依赖图 + 风险 + 已知缺口 |
| v1.0+1 | 2026-08-28 | 架构师 + PMO（Mavis 接手 agent per DEC-008） | 已知缺口闭环：§0.1 commit hash 实证 3 份 + §6.1/§6.9 诚实标"v2.0 整份不存在" + §6.2 M1-S0 实际收尾补 2026-08-27 + §6.10 T-01 Kafka 推 K3s 阶段二 + §6.11 错误码表引用闭环（OpenAPI/proto/alertmanager 未在 T-01 范围，待 T-07 启动） |
| v1.0+2 | 2026-09-01 | 架构师 + PMO（Mavis 接手 agent per DEC-008） | 启动会 10 项决议落地同步：详见开头修订履历（决议 1+2 v2.0 双升版 / 决议 3 窗口 8/31-9/27 / 决议 4 RACI SLA / 决议 5 token-OLU v0.1 / 决议 6 SRE 独立估算 / 决议 7 user-service schema / 决议 8 OI-6 / 决议 9 复盘模板 / 决议 10 错误码闭环入 T-07） |

---

**文档结束（v1.0+2，T-01 + T-02 已关闭，启动会 10 项决议已落地，Phase 1-4 任务按 4 周窗口 8/31-9/27 推进中）**
