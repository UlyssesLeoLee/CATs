# CATs Sprint 概要模板 v1.0

> **文档编号**：CATs-PMO-TPL-003
> **关联任务**：T-07 Sprint 概要 v1.0（per Sprint 1 任务拆解 v1.0+2 §2 line 136 + 150 任务 #148 复盘）
> **估时**：PMO Lead 100K-200K token（per 任务拆解 §2）
> **版本**：v1.0
> **创建日**：2026-09-11
> **状态**：DDD Review 草稿（6 角色 7 天评审待补，留 Sprint 1 末 9/27 前）
> **作者**：PMO Lead（Ulysses 兼一人公司 12 角色 PMO 岗位 / Mavis 接手 agent per DEC-008 代签）

---

## 文档管理信息

### 审批栏

| 角色 | 姓名 | 审批 | 日期 | 备注 |
|------|------|------|------|------|
| Sponsor (Ulysses 本人签) | Ulysses | ☐ | — | 一人公司 = Ulysses 持有 Sponsor 角色，不代签 |
| 架构师 Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 范围 + 技术基线对齐 |
| Rust Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 实施估时对齐 |
| DBA Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | DB schema 估时对齐 |
| QA Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 测试节奏 |
| PMO Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 主持方 |
| SRE 平台 Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | K3s 部署对齐 |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|------|------|--------|----------|
| v1.0 | 2026-09-11 | PMO Lead（Mavis 接手 agent per DEC-008） | 初版：Sprint 概要模板（6 段结构 + 12 子项），per T-07 Sprint 概要部分 + 启动会决议 9 Sprint 复盘模板（97b3bb1）派生 |

---

## 0. 元信息

| 项 | 值 |
|----|----|
| **模板用途** | Sprint 启动时使用的概要标准（per 启动会决议 9 + Sprint 1 复盘 v1.0 §2.3 实施动作清单 + 任务拆解 §6.13 Sprint 概要模板派生） |
| **源文档** | M1-Sprint 1 启动会决议纪要 v1.0（commit 1b27b2b）+ Sprint 1 任务拆解 v1.0+2（commit 622c21a 派生）+ Sprint 复盘纪要模板 v1.0（commit 97b3bb1）+ Sprint 1 复盘纪要 v1.0（commit 40ae33a） |
| **关联模板** | Sprint 复盘纪要模板 v1.0（97b3bb1，commit 97b3bb1）— Sprint N 末 复盘 = 本概要的下游 |
| **DDD Review** | 6 角色 7 天评审，9/4~9/11 截止已逾期 5 天（per Sprint 1 复盘 §9.1），留 Sprint 1 末 9/27 前补 |
| **模板结构** | 6 段（信息 + 范围 + RACI + 任务清单 + 风险 + 启动会）+ 12 子项 |

### 0.1 引用清单（git 实证）

| 引用文档 | 路径 | commit hash | 用途 |
|---------|------|------------|------|
| 启动会决议纪要 v1.0 | `doc/05-其他/会议记录/CATs_M1_Sprint1_启动会决议纪要_v1.0.md` | `1b27b2b` | 10 决议 + 5 风险 + 6 已知缺口 |
| Sprint 1 任务拆解 v1.0+2 | `doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md` | `622c21a` | §2 任务清单 + §2.2 估时 + §5 RACI + §6 已知缺口 |
| Sprint 复盘纪要模板 v1.0 | `doc/05-其他/管理/模板/CATs_Sprint复盘纪要模板_v1.0.md` | `97b3bb1` | Sprint 末复盘模板（本概要的下游） |
| Sprint 1 复盘纪要 v1.0 | `doc/05-其他/复盘纪要/CATs_Sprint1_复盘纪要_v1.0.md` | `40ae33a` | W1+W2 12 天复盘首次使用模板 |
| RACI SLA 模板 v1.0 | `doc/05-其他/管理/模板/CATs_RACISLA模板_v1.0.md` | `622c21a` | §4 违规处置 + §3 24h SLA |
| Baseline 一览 v1.0 | `doc/05-其他/管理/CATs_Baseline一览_v1.0.md` | `4f96f95` | §6 基线化清单 + Sprint N 任务派发基线 |

### 0.2 模板使用指南

> 每个 Sprint 启动时（T-07 实施或 Sprint N 启动会派发），由 PMO Lead 使用本模板，输出 `<项目>_Sprint<N>_概要_v1.0.md`。
>
> 模板占位符约定：
> - `[Sprint N]` → 替换为实际 Sprint 编号
> - `[YYYY-MM-DD]` → 替换为实际日期
> - `[3-4 句摘要]` → Sprint 范围 + 关键里程碑
> - `[估时区间]` → per token-OLU 框架 v0.1 §3 单位

---

## 1. Sprint 基础信息

| 项 | 值 |
|----|----|
| **Sprint 编号** | Sprint [N] |
| **Sprint 阶段** | [M1 / M2 / ...] |
| **周期** | [YYYY-MM-DD] ~ [YYYY-MM-DD]（4 周） |
| **启动会日期** | [YYYY-MM-DD] 14:00-15:30 JST（90 min） |
| **Sprint 末复盘日期** | [YYYY-MM-DD] 18:00 JST（per Sprint 复盘模板 97b3bb1） |
| **本概要版本** | v1.0 |
| **主持** | PMO Lead（一人公司 = Ulysses 兼 12 角色 PMO 岗位） |
| **关联 Sprint 复盘** | 9/27 18:00 JST（per Sprint 1 复盘 §11 修订历史） |

---

## 2. Sprint 范围

### 2.1 一句话摘要

> [3-4 句摘要：Sprint 阶段目标 + 关键里程碑 + 跨 Sprint 关联]

### 2.2 范围明细

| 范围类型 | 内容 | 估时 | 责任人 |
|---------|------|------|--------|
| **设计升版** | [设计书 v[N] 升 v[N+1] 范围] | [估时区间] | 架构师 Lead |
| **16 域实施** | [新增 / 升级 service crate 范围] | [估时区间] | Rust + DBA Lead |
| **可观测性** | [Prometheus / alertmanager / OTel 范围] | [估时区间] | SRE 平台 Lead |
| **QA 测试** | [单元 / 集成 / E2E 测试范围] | [估时区间] | QA Lead |
| **复盘** | Sprint 末复盘 v[N+1] 落档（per Sprint 复盘模板 97b3bb1） | 30K-50K | PMO Lead |

### 2.3 范围外（明确不做）

- [明确不在本 Sprint 范围的事项]
- [理由：依赖未到位 / 优先级 / 预算约束]

---

## 3. RACI 矩阵

### 3.1 5 域 Lead + SRE 平台 Lead RACI

| 任务 | 架构师 Lead | Rust Lead | DBA Lead | QA Lead | PMO Lead | SRE 平台 Lead |
|------|----------|-----------|----------|---------|----------|----------------|
| [任务 1] | R | A | C | C | I | I |
| [任务 2] | A | R | C | C | I | I |
| [任务 3] | C | C | A | I | I | R |
| [任务 N] | ... | ... | ... | ... | ... | ... |

> R = Responsible（执行）/ A = Accountable（最终责任）/ C = Consulted（咨询）/ I = Informed（通知）

### 3.2 5 域 Lead 不兼任（per 2026-08-21 决议 + 守门 #3 + 守门 #10）

- 5 域 Lead 槽位互不兼任（一人公司 = Ulysses 兼 + Mavis 永久代签 per 守门 #14 v3 + 8/27/9/8 多次强化）
- RACI 决策延迟 RACI SLA 24h（per RACI SLA 模板 v1.0 §3）
- 应急响应直接升 Sponsor（per 启动会 R-05 风险）

---

## 4. 任务清单（per 启动会决议 + 任务拆解 §2）

### 4.1 任务级跟踪

| 任务 | 标题 | 估时（token-OLU） | 责任人 | 截止 | 状态 | 关联 commit |
|------|------|-------------------|--------|------|------|-------------|
| [T-XX] | [任务名] | [估时区间] | [Lead] | [YYYY-MM-DD] | [plan/doing/done/blocked] | [commit hash 引用] |

### 4.2 估时单位（per token-OLU 框架 v0.1 §3）

- 1 人·天 ≈ 100K-300K tokens（含输入 + 输出 + 决策对话 + 验证往返）
- 1 人·周 ≈ 500K-1.5M tokens
- 1 SRE Lead 上限 = 1 人·周 ≈ 1M tokens（per NFR-OP-010）
- 5 域独立 Lead × 14-18 周 = 80K-120M tokens（待 SRE Lead + PM 校准）

### 4.3 优先级

- **P0** = 阻塞 Sprint 主线，0 推迟
- **P1** = 重要但可延后 1 周
- **P2** = 重要但可延后 1 月

---

## 5. 风险登记

### 5.1 风险识别（per 启动会 R-01..R-05 模板 + Sprint 1 复盘 §2.2 实证）

| # | 风险 | 评级 | 缓解 | 责任人 |
|---|------|------|------|--------|
| [R-XX] | [风险描述] | [高/中/低] | [缓解措施] | [Lead] |

### 5.2 已知缺口（per 启动会 §6 模板 + 任务拆解 §6）

| # | 已知缺口 | 评级 | 缓解 | 责任人 |
|---|----------|------|------|--------|
| [G-XX] | [缺口描述] | [高/中/低] | [缓解措施] | [Lead] |

### 5.3 RACI SLA 违规处置（per RACI SLA 模板 v1.0 §4）

- **决策类**（决议 / 任务派发）: 24h 响应，超时升 Sponsor
- **评审类**（DDD Review / 文档评审）: 7 天窗口，超时留 Sprint 末补
- **应急类**（故障响应 / 安全告警）: 立即响应，按 alertmanager 规则升级
- **任务类**（实施 / 落地）: 按截止日，超时升 Sprint 末复盘评审

---

## 6. 启动会议程

### 6.1 议程（per Sprint 1 启动会 commit 1b27b2b 模式）

| 时间 | 议题 | 主持 | 输出 |
|------|------|------|------|
| 14:00-14:10 | Sprint 范围 + 关键里程碑 | PMO Lead | §2 范围 + §4 任务 |
| 14:10-14:30 | 设计升版范围 | 架构师 Lead | §2.2 设计部分 |
| 14:30-14:50 | 16 域实施范围 | Rust + DBA Lead | §2.2 + §4 任务级 |
| 14:50-15:00 | 可观测性 + K3s 部署 | SRE 平台 Lead | §2.2 + §4 任务级 |
| 15:00-15:20 | QA + 评审节奏 | QA Lead | §3 RACI + §5 风险 |
| 15:20-15:30 | 拍板 + 签字 | Sponsor | 本概要签字栏 |

### 6.2 决议格式（per 守门 v28 拍板必带推荐项 + 9/1 14:58 拍板必用选项）

- 决议需给"推荐项 + 备选 1 + 备选 2"3 选项
- 推荐项标 "(推荐)" / "（推荐）"
- 拍板时间精确到 JST
- 责任人明确（5 域 Lead 槽位 + SRE 平台）
- 截止时间 + 估时

### 6.3 签字栏（per 守门 #14 v4 Mavis 审核）

| 角色 | 签字 | 时间 |
|------|------|------|
| Sponsor | Ulysses 本人签 | [YYYY-MM-DD] HH:MM JST |
| 架构师 Lead | Ulysses（Mavis 代签 per DEC-008） | [YYYY-MM-DD] HH:MM JST |
| Rust Lead | Ulysses（Mavis 代签 per DEC-008） | [YYYY-MM-DD] HH:MM JST |
| DBA Lead | Ulysses（Mavis 代签 per DEC-008） | [YYYY-MM-DD] HH:MM JST |
| QA Lead | Ulysses（Mavis 代签 per DEC-008） | [YYYY-MM-DD] HH:MM JST |
| PMO Lead | Ulysses（Mavis 代签 per DEC-008） | [YYYY-MM-DD] HH:MM JST |
| SRE 平台 Lead | Ulysses（Mavis 代签 per DEC-008） | [YYYY-MM-DD] HH:MM JST |

---

## 7. 升版流程

### 7.1 升版触发条件

- Sprint 范围变化（新增 / 删除 / 优先级调整）→ §2.2 + §4 任务清单
- RACI 变更（5 域 Lead 真人到位 / Mavis 永久代签）→ §3.1 + §3.2
- 风险 / 缺口 增减 → §5.1 + §5.2
- 启动会议程调整 → §6.1
- 升版至 v1.1 需走 DDD Review 6 角色 7 天评审 + CAB 决议

### 7.2 升版流程

1. **PR 起草**：PMO Lead 主责
2. **DDD Review**：6 角色 7 天评审（含 Sponsor 本人签）
3. **CAB 决议**：v1.x → v2.0 需走 CAB-002（参考 CAB-001 v1.0 流程）
4. **基线化**：v 升 B-y.y，Baseline一览 + Sprint 1 任务拆解同步
5. **引用同步**：Sprint 复盘纪要模板 v1.0（97b3bb1）+ RACI SLA 模板 v1.0 + token-OLU 框架 v0.1 + Baseline 一览 v1.0 同步

### 7.3 当前 v1.0 适用范围

- **时间窗口**：M1-Sprint 1+ 全部 Sprint（per 微服务架构设计书 v1.0 §4.1 8 域 MVP + 7 域 Sprint 1 范围 + Sprint 2-4 派生）
- **关联 commit**：1b27b2b (启动会决议 1-10) + 97b3bb1 (Sprint 复盘模板) + 40ae33a (Sprint 1 复盘 v1.0 首次使用) + 622c21a (Sprint 1 任务拆解 v1.0+2 派生)
- **升版预期**：v1.1 = Sprint 2 实施时（K3s 阶段二 Kafka 物理发布后）

---

## 8. 已知缺口（DDD Review 必查 per AI 协作文档治理 2026-08-26）

### 8.1 GAP-TPL-1: §3 RACI 矩阵 5 域 Lead 真人到位率 0%

- 5 域 Lead 永久 1 人 Ulysses 兼 + Mavis 永久代签（per Sprint 1 复盘 §3 + 守门 #14 v3 + 8/27/9/8 多次强化）
- **建议**: 长期接受现状，永久代签机制
- **当前状态**: 0 阻塞

### 8.2 GAP-TPL-2: §4.1 任务清单模板占位符 0 实际 Sprint 任务

- 本 v1.0 是模板，0 实际 Sprint N 任务填入
- 首次实际使用: Sprint 2 启动时（per V1.1 排期草案 b3001f6 + f118c91 派生）
- **建议**: Sprint 2 启动会使用本模板时填入 §4.1
- **当前状态**: 0 阻塞

### 8.3 GAP-TPL-3: §5.3 RACI SLA 24h 实际 0 RACI 决策违规

- Sprint 1 12 天 0 决策类 RACI SLA 违规（per Sprint 1 复盘 §5.1）
- 2 评审类违规（DDD Review 6 角色 7 天 + 决议 9 模板 9/4 截止已逾期）
- **建议**: Sprint 2 末继续跟踪 RACI SLA 统计
- **当前状态**: 0 阻塞

### 8.4 GAP-TPL-4: DDD Review 6 角色 7 天评审 9/4 截止已逾期 5 天

- 本模板 + Sprint 1 复盘 + Sprint 1 任务拆解 + Sprint 复盘模板 + RACI SLA 模板 + 画图 v1.0 等 15+ commit 待评审
- 5 域 Lead 真人到位率 0%，Mavis 永久代签
- **建议**: Sprint 1 末 9/27 前补 7 天评审窗口
- **当前状态**: 0 阻塞

### 8.5 GAP-TPL-5: §6.1 启动会议程 90 min 模板基于 Sprint 1 启动会 1b27b2b 实际

- Sprint 1 启动会 8/30 14:00-15:30 JST 实际 90 min（per 1b27b2b commit message）
- Sprint 2+ 启动会可调整议程时间（Sprint 2 范围更大可能需要 120 min）
- **建议**: Sprint 2 启动会前调整议程
- **当前状态**: 0 阻塞

---

## 9. 关联文档

| 文档 | 路径 | 用途 |
|------|------|------|
| Sprint 复盘纪要模板 v1.0 | `doc/05-其他/管理/模板/CATs_Sprint复盘纪要模板_v1.0.md` | 本概要的下游（Sprint 末复盘） |
| 启动会决议纪要 v1.0 | `doc/05-其他/会议记录/CATs_M1_Sprint1_启动会决议纪要_v1.0.md` | 10 决议 + 5 风险 + 6 已知缺口 |
| Sprint 1 任务拆解 v1.0+2 | `doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md` | §2 任务清单 + §2.2 估时 + §5 RACI |
| RACI SLA 模板 v1.0 | `doc/05-其他/管理/模板/CATs_RACISLA模板_v1.0.md` | §4 违规处置 + §3 24h SLA |
| token-OLU 框架 v0.1 | `doc/05-其他/管理/CATs_token-OLU框架_v0.1.md` | §3 单位 + §4 Sprint 估时 |
| Baseline 一览 v1.0 | `doc/05-其他/管理/CATs_Baseline一览_v1.0.md` | §6 基线化清单 + Sprint N 任务派发基线 |
| Sprint 1 复盘 v1.0 | `doc/05-其他/复盘纪要/CATs_Sprint1_复盘纪要_v1.0.md` | W1+W2 12 天复盘首次使用复盘模板 |
| CATs 画图 v1.0 | `doc/02-基础设计/架构设计/CATs_画图_v1.0.md` | 6 张 Mermaid 画图（per 决议 1-10 派生） |

---

## 10. 修订历史

| 版本 | 日期 | 修订人 | 修订内容 | 触发 |
|---|---|---|---|---|
| v1.0 | 2026-09-11 | Ulysses（一人公司 12 角色 PMO Lead per DEC-008）— Mavis 接手 | 初版：Sprint 概要模板（6 段 + 12 子项），per T-07 Sprint 概要部分 + 启动会决议 9 + Sprint 复盘模板 97b3bb1 派生 | 14:55 JST 09/11 Ulysses "跑完abc" 自驱响应 (per 9/8 15:19 第 6 次强化 Mavis 全权代理) |

---

**概要模板结束 (v1.0, 2026-09-11, T-07 Sprint 概要部分落地, 6 段结构 + 12 子项 + 5 已知缺口 + 9 关联文档引用)**
