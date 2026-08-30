# CATs M1-Sprint 1 启动会决议纪要 v1.0

> **文档编号**：CATs-PMO-013
> **版本**：v1.0
> **会议日期**：2026-08-30 14:00-15:30 JST（90 min）
> **密级**：仅社内
> **主持**：PMO Lead（Ulysses 兼任一人公司 12 角色 PMO 槽位 / Mavis 接手 agent per DEC-008 代签）

---

## 文档管理信息

### 审批栏

| 角色 | 姓名 | 审批 | 日期 | 备注 |
|------|------|------|------|------|
| Sponsor (Ulysses 本人签) | Ulysses | ☐ | — | 一人公司 = Ulysses 持有 Sponsor 角色，不代签 |
| 架构师 Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 议程 1+2+7+8+10 主责任 |
| Rust Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 议程 1+7 共同责任 |
| DBA Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 议程 7 共同责任（user_db schema） |
| QA Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | DDD Review 共同责任 |
| PMO Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 主持 / 议程 3+4+5+6+9 主责任 |
| SRE 平台 Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 议程 6 主责任 |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|------|------|--------|----------|
| v1.0 | 2026-08-30 | PMO Lead + 架构师（Mavis 接手 agent per DEC-008） | 初版：M1-Sprint 1 启动会 10 项议程决议 + 实施动作清单 + 风险 + 已知缺口 |

---

## 0. 元信息

| 项 | 值 |
|----|----|
| **会议时间** | 2026-08-30 14:00-15:30 JST（90 min） |
| **会议形式** | 一人公司现场（Ulysses 兼任 6 角色）+ SRE 平台 Lead 远程 |
| **主持** | PMO Lead（Ulysses / Mavis 代签 per DEC-008） |
| **出席** | 6 角色全员（Sponsor + 架构师 + Rust Lead + DBA + QA + PMO + SRE 平台） |
| **缺席罚则** | 一人公司 = Ulysses 兼任 6 角色，无缺席 |
| **输入** | Sprint 1 启动会议程 v1.0（commit `00e025a`）10 项议程 |
| **输出** | 10 项决议 + 实施动作清单（按截止时间排序）+ 风险 + 已知缺口 |
| **配套 Excel** | 无（决议在文档内可读） |
| **密级** | 仅社内 |

### 0.1 源文档引用清单（git 实证）

| 引用文档 | 路径 | commit hash | 用途 |
|---------|------|------------|------|
| CATs_M1_Sprint1_启动会议程 v1.0 | `doc/05-其他/管理/CATs_M1_Sprint1_启动会议程_v1.0.md` | `00e025a` | §2 10 项议程基线（本次决议的输入） |
| CATs_M1_Sprint1_任务拆解 v1.0.md (v1.0+1) | `doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md` | `59a4f70` | §6 已知缺口（议程来源） |
| CATs_错误码表 v1.0 | `doc/05-其他/管理/CATs_错误码表_v1.0.md` | `2146f53` | §6.2 显式要求模块设计书 §4 / 接口设计书 §3.5 引用（议程 1+2 + 决议 10 依据） |
| CATs_技术基线 v1.0 | `doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md` | `047dc9c` | §8 OI 状态（OI-1/2/3/4 全 🟢）+ §1 锁定 Rust 1.98.0 + PG 18.6 + pgvector 0.8.6 |
| CATs_Baseline一览 v1.0 | `doc/05-其他/管理/CATs_Baseline一览_v1.0.md` | `4f96f95` | §5.1 接口契约 v1.0.0（已基线化端点清单）+ §6 待基线化清单 |
| CATs_项目管理计划书 v1.0 | `doc/05-其他/管理/CATs_项目管理计划书_v1.0.md` | `d1b10fe` | §6.1 里程碑表（M1-S0/M1-S3 节点） |
| CATs_微服务架构设计书 v1.0 | `doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md` | `2910f3d` | §4.1 核心 8 MVP 服务（议程 7 user-service 范围） |
| CATs_工作流文档 v1.0 | `doc/05-其他/CATs_工作流文档_v1.0.md` | `d1b10fe` | 150 任务 ID 映射（议程 5 估时基准） |

### 0.2 文档结束标识

v1.0，2026-08-30 启动会决议归档（90 min 完成，10/10 决议通过）。

---

## 1. 会议背景

### 1.1 M1-S0 已收尾（per CAB-001 + D-Day 6 角色签字 + OI-3 e2e 5/5）

- **评审会 D-Day**：2026-08-27 16:33 JST，6 角色现场签字 + 代签
- **CAB-001 v1.0 决议书落地**：commit `4f96f95`
- **B0.0 基线生效**：v1.0 = B0.0（per 评审会决议）
- **OI-1 / OI-2 / OI-3 / OI-4** 全部 🟢（per 技术基线 v1.0 §8）

### 1.2 Sprint 1 任务进度

- **T-01 关闭**：per commit `2146f53`（`CATs_错误码表_v1.0.md` 同步落地）— refresh 轮换 + logout + 错误码表 v1.0 + 5/5 判据全绿
- **T-02 关闭**：per commit `89f72cd`（user-service 脚手架 + cats-common 复用 + healthz + CRUD stub + 5/5 判据全绿）
- **T-03 / T-04 / T-05 / T-06 / T-07**：未启动，待本次会议决议

### 1.3 Sprint 1 起点

- M1-S0 实际收尾 = 2026-08-27
- **Sprint 1 起点 ≥ 2026-08-28**（per 议程 3 决议）

### 1.4 启动会的目标

> **从 B0.0 → Sprint 1 实做阶段的转折点**：把 Sprint 1 拆解 v1.0+1 §6 的 10 项已知缺口**全部转成决议**，确保 T-03~T-07 启动无阻塞。

---

## 2. 决议（10 项）

### 决议 1：接口设计书 v2.0 升版路径

- **PMO 立场**：**通过方案 A**（升 v2.0）
- **理由**：
  - 错误码表 v1.0 §6.2（commit `2146f53`）**显式**要求接口设计书 v2.0 §3.5 引用本表
  - 方案 B 引用分散到微服务架构书 v1.0 + ADR 集会导致 Sprint 2+ 维护成本翻倍
  - 方案 C 临时草案易变债（per 2026-08-26 强证据"缺标比错标安全"）
  - 一人公司资源约束下，方案 A 的 200K-400K tokens 投入可接受
- **实施动作**：
  1. 架构师 Lead 升 v2.0：§3.5 错误响应格式 + §4 接口规范 + §5 gRPC status 映射 + §6 auth/user 端点详细 schema
  2. Rust Lead review §5 gRPC status 映射（实现角度）
  3. DDD Review 6 角色 7 天内
- **截止**：2026-09-06（Sprint 1 W1 末，per 议程 1 推荐）
- **责任人**：
  - 主责任：架构师 Lead
  - 共同责任：Rust Lead（gRPC status 映射）
- **估时**：200K-400K tokens

### 决议 2：auth-service 模块设计书 v2.0 升版路径

- **PMO 立场**：**通过方案 A**（升 v2.0，与决议 1 同步）
- **理由**：
  - 决议 1 已通过升 v2.0 接口设计书 → 模块设计书 §4（错误码引用）必须同步升
  - 否则错误码表 v1.0 §6.2 引用链路断裂（auth-service 模块设计书 §4 是引用终端）
  - 一人公司资源约束下，决议 1 + 2 合并实施更省 token
- **实施动作**：
  1. 架构师 Lead 升 v2.0：§4 错误码章节（引用错误码表 v1.0 §3 + §4）+ §5 模块结构 + §6 类图
  2. 与决议 1 接口设计书 v2.0 §3.5 同步升版
  3. DDD Review 6 角色 7 天内
- **截止**：2026-09-13（Sprint 1 W2 末，per 议程 2 推荐）
- **责任人**：
  - 主责任：架构师 Lead
  - 共同责任：Rust Lead
- **估时**：200K-400K tokens

### 决议 3：Sprint 1 窗口日期

- **PMO 立场**：**通过方案 A**（4 周：2026-08-31 ~ 2026-09-27）
- **理由**：
  - M1-S0 已 2026-08-27 收尾，Sprint 1 起点 8/28（per 议程 3 推荐）
  - 4 周窗口与 M1-S3（2026-10~12-15）起点间隔 3 周缓冲
  - 6 周窗口（方案 B）会推迟 M1-S2 启动，与 150 任务工作流节奏不符
  - token-OLU 估时总和 1.75M-3.2M tokens ≈ 5 域 Lead 累计 17-31 人·天，4 周（20 工作日）合理
- **实施动作**：
  1. PMO Lead 同步更新 Sprint 1 拆解 v1.0+1 §6.2（项目计划书 commit hash 已知缺口 + 窗口日期补）
  2. 启动会纪要 commit 触发 DDD Review 7 天内
- **截止**：本日（2026-08-30）
- **责任人**：PMO Lead
- **估时**：5K-10K tokens（v1.0+1 patch）

### 决议 4：RACI Consulted 响应 SLA

- **PMO 立场**：**通过方案 A**（24h 内必须回复，默认）
- **理由**：
  - 方案 B 8h 强约束会破坏咨询密度（QA Lead / DBA Lead 经常被 5 任务并行咨询）
  - 方案 C 不定 SLA 依赖口头共识会变债（per 2026-08-26 强证据）
  - 24h = 工作时间 ≈ 1 个工作日，符合 PMO 标准
  - 紧急情况走 PMO 升级到 Sponsor 直接裁决（per 已知缺口 §5.5）
- **实施动作**：
  1. PMO Lead 起草 RACI SLA 模板 v1.0（覆盖 5 域 Lead + SRE 共 6 角）
  2. RACI SLA 模板落地至 `doc/05-其他/管理/模板/CATs_RACISLA模板_v1.0.md`
  3. 启动会后 1h 内 commit
- **截止**：2026-08-30 16:30 JST
- **责任人**：PMO Lead
- **估时**：20K-30K tokens

### 决议 5：token-OLU v0.1 立项

- **PMO 立场**：**通过方案 A**（立项 CATs token-OLU v0.1）
- **理由**：
  - 跨项目 RGS-TS-001 §6.2 草案（不在本 worktree 内）依赖性高 → 立项可解耦
  - 方案 B 继续引用 RGS-TS-001 草案会让 CATs 项目进度受 RGS 进度影响
  - 立项 50K-100K tokens 投入 vs Sprint 1-3 总计 5 域 Lead × 60-90 周 = 60-120M tokens 的 OLU 决策基础 — 投入产出比高
  - token-OLU 系数区间（100K-300K）需 CATs 仓独立确认（per 已知缺口 §5.2）
- **实施动作**：
  1. PMO Lead 起草 `doc/05-其他/管理/CATs_token-OLU框架_v0.1.md`
  2. 含 5 域 Lead 系数确认（Rust 250K-450K / 架构 200K-300K / DBA 200K-350K / QA 200K-400K / PMO 100K-200K）+ SRE 平台 50K-100K
  3. 引用 RGS-TS-001 §6.2 草案但独立立项
  4. DDD Review 6 角色
- **截止**：2026-09-06（Sprint 1 W1 末）
- **责任人**：PMO Lead
- **估时**：50K-100K tokens

### 决议 6：SRE 平台独立估算

- **PMO 立场**：**通过方案 A**（启动会现场提估算）→ **改为方案 B**（Sprint 1 W1 周三前补，因现场无 SRE 平台 Lead 实时数据）
- **理由**：
  - 启动会现场（2026-08-30 15:30 JST）SRE 平台 Lead 需查 K3s 集群 + Harbor 镜像状态后提估算
  - 现场强行估算会有"约 50K-100K tokens"的不准确占位
  - W1 周三（2026-09-02）前补，可拿到实际 cluster status → 估算更准
  - per R-02 风险（CI Pipeline 在裸金属 K3s 集群不通），SRE 资源估算是关键 unblock
- **实施动作**：
  1. SRE 平台 Lead W1 周三（2026-09-02 17:00 JST）前 commit `CATs_M1_Sprint1_SRE独立估算_v1.0.md`
  2. 含 K3s 集群状态 + Harbor 镜像 + kubeconfig 注入 token 估算（≤ 200K tokens）
  3. 不超过 token-OLU v0.1（per 决议 5）SRE 平台 50K-100K 区间的 2 倍（200K 上限）
- **截止**：2026-09-02 17:00 JST
- **责任人**：SRE 平台 Lead
- **估时**：≤ 200K tokens

### 决议 7：user-service 详细 schema

- **PMO 立场**：**通过方案 A**（纳入接口设计书 v2.0，决议 1 的一部分）
- **理由**：
  - 方案 B（T-02 内含）已实做（commit `89f72cd` 包含 models.rs），但详细 schema 需 v2.0 文档化
  - 接口设计书 v2.0 §6（决议 1 实施）已包含 user-service 端点清单（GET/POST/PUT /v1/users/{id}）
  - 不重复造结构
- **实施动作**：
  1. 架构师 Lead 在决议 1 实施时同步补 §6 user-service 详细 schema（request/response JSON 示例）
  2. 引用 `crates/user-service/src/models.rs` 已实做的 struct
  3. Rust Lead + DBA Lead review（user_db schema 已在 T-02 commit `89f72cd` 落地）
- **截止**：2026-09-06（与决议 1 同步）
- **责任人**：
  - 主责任：架构师 Lead
  - 共同责任：Rust Lead + DBA Lead
- **估时**：50K-100K tokens（决议 1 估时范围内）

### 决议 8：OI-6 跨项目引用方同步

- **PMO 立场**：**通过方案 A**（T-03 借机验证）
- **理由**：
  - 方案 B 独立 T-08 需 150K-300K tokens，与 Sprint 1 估时紧张不符
  - T-03（RBAC 权限矩阵 v1.0）本就涉及"角色 × 资源"矩阵，借机验证 RGS / Physis / Star 三个项目是否锁定 B0.0 自然
  - 零额外 token 投入
- **实施动作**：
  1. 架构师 Lead 在 T-03（RBAC 矩阵）实施时同步扫 3 个项目基线
  2. 扫 RGS-TS-001 / Physis-Engine / Star-Renderer 三仓的 `技术基线` 类文档
  3. 输出 `OI-6_跨项目同步_status.md`（轻量报告，30K-50K tokens）
  4. 同步入 Sprint 1 拆解 v1.0+1 §6.7
- **截止**：2026-09-13（Sprint 1 W2 末，与 T-03 一致）
- **责任人**：架构师 Lead
- **估时**：30K-50K tokens（含在 T-03 估时内）

### 决议 9：Sprint 复盘模板 v1.0 基线化

- **PMO 立场**：**通过方案 A**（启动会前 PMO 提交）→ **延后到 W1 周五**（per 实际进度）
- **理由**：
  - 方案 B 用临时模板会变债（per 2026-08-26 强证据）
  - 启动会前提交（截止 2026-08-30 14:00 JST）时间窗口已过，改为 W1 周五（2026-09-04）前 commit
  - W1 周五前可保证模板与 T-07 复盘需求对齐（避免后期返工）
- **实施动作**：
  1. PMO Lead 起草 `doc/05-其他/管理/模板/CATs_Sprint复盘纪要模板_v1.0.md`
  2. 含 5 域 Lead 反馈章节 + 7 任务完成率表 + 已知问题 + Sprint 2 建议
  3. DDD Review 6 角色 7 天内
- **截止**：2026-09-04 17:00 JST（Sprint 1 W1 周五）
- **责任人**：PMO Lead
- **估时**：30K-50K tokens

### 决议 10：T-01 Kafka 推 K3s 阶段二 + 错误码表引用闭环

- **PMO 立场**：**通过方案 A**（T-07 统一处理 OpenAPI + proto + alertmanager + 模块设计书）
- **理由**：
  - 错误码表 v1.0 §6.2-§6.4 三处引用未在 T-01 范围（commit `2146f53` 已记录为已知缺口）
  - T-07（类图 v1.0 + Sprint 复盘）天然涉及 OpenAPI / proto 文件 review，借机补错误码表引用闭环
  - 模块设计书 v2.0 升版（决议 2）已含 §4 错误码引用 → 决议 10 与决议 2 重叠部分合并
  - 方案 B 单独 T-08 会拆散评审节奏（6 角色 DDD Review 每次 ≤ 90 min）
- **实施动作**：
  1. 架构师 Lead 在 T-07 实施时同步处理：
     - `api/openapi/cats-openapi-v1.yaml` ErrorBody.error 枚举对齐错误码表 v1.0 §3
     - `proto/cats/v1/*.proto` gRPC status code 对齐错误码表 v1.0 §2.2
     - `alertmanager` rules draft 落 `doc/05-其他/可观测性/CATs_告警规则_v1.0.md`（按 error 字段聚合）
     - 错误码表 v1.0 §5.2 加 Kafka topic 名 + partition key 映射（per 决议 10 + K3s 阶段二准备）
  2. DDD Review 6 角色（合并到 T-07 DDD Review）
- **截止**：2026-09-27（Sprint 1 末，与 T-07 一致）
- **责任人**：架构师 Lead
- **估时**：80K-120K tokens（含在 T-07 估时内）

---

## 3. 决议汇总

| 决议 | 立场 | 截止 | 责任人 | 估时（tokens） |
|------|------|------|--------|----------------|
| 1 接口设计书 v2.0 | 通过 A 升 v2.0 | 2026-09-06 | 架构师 Lead | 200K-400K |
| 2 模块设计书 v2.0 | 通过 A 升 v2.0 | 2026-09-13 | 架构师 Lead | 200K-400K |
| 3 Sprint 1 窗口 | 通过 A 4 周 | 2026-08-30 | PMO Lead | 5K-10K |
| 4 RACI SLA | 通过 A 24h | 2026-08-30 16:30 | PMO Lead | 20K-30K |
| 5 token-OLU v0.1 | 通过 A 立项 | 2026-09-06 | PMO Lead | 50K-100K |
| 6 SRE 独立估算 | 通过 B W1 周三 | 2026-09-02 17:00 | SRE 平台 Lead | ≤ 200K |
| 7 user-service schema | 通过 A 纳入 v2.0 | 2026-09-06 | 架构师 Lead | 含在决议 1 |
| 8 OI-6 跨项目同步 | 通过 A T-03 借机 | 2026-09-13 | 架构师 Lead | 含在 T-03 |
| 9 Sprint 复盘模板 | 通过 A W1 周五 | 2026-09-04 17:00 | PMO Lead | 30K-50K |
| 10 Kafka + 错误码闭环 | 通过 A T-07 统一 | 2026-09-27 | 架构师 Lead | 含在 T-07 |
| **合计** | — | — | — | **505K-1,090K** |

### 3.1 5 域独立 Lead 不兼任（per 2026-08-21 决议）

| Lead | 承担决议 | 累计 token 估算 |
|------|---------|----------------|
| 架构师 Lead | 1+2+7+8+10 | 510K-970K |
| Rust Lead | 1+2+7（共同） | 含在架构师范围内 |
| DBA Lead | 7（共同） | 含在架构师范围内 |
| QA Lead | （DDD Review 共同） | 0（评审不领预算）|
| PMO Lead | 3+4+5+9 | 105K-190K |
| SRE 平台 Lead | 6 | ≤ 200K |
| **合计** | 10 决议 | **505K-1,090K** |

> 5 域独立 Lead 严格不兼任：架构师/Rust/DBA/QA/PMO 5 槽位互不兼任，SRE 平台单独槽位。共同责任（决议 1+2+7）属于协作评审，不重复计算预算。

---

## 4. 实施动作清单（按截止时间排序）

| # | 动作 | 责任人 | 截止 | 关联决议 |
|---|------|--------|------|----------|
| 1 | commit 启动会决议纪要 v1.0 | Mavis（代签） | 2026-08-30 16:00 | — |
| 2 | commit RACI SLA 模板 v1.0 | PMO Lead | 2026-08-30 16:30 | 4 |
| 3 | commit Sprint 1 拆解 v1.0+2（窗口日期 + 决议结果同步） | Mavis | 2026-08-30 17:00 | 3 |
| 4 | commit CATs_M1_Sprint1_SRE独立估算_v1.0 | SRE 平台 Lead | 2026-09-02 17:00 | 6 |
| 5 | commit CATs_Sprint复盘纪要模板_v1.0 | PMO Lead | 2026-09-04 17:00 | 9 |
| 6 | commit CATs_接口设计书_v2.0 + CATs_模块设计书_v2.0（§3.5 + §4 错误码） | 架构师 Lead | 2026-09-06 | 1+2 |
| 7 | commit CATs_token-OLU框架_v0.1 | PMO Lead | 2026-09-06 | 5 |
| 8 | commit T-03 启动 + OI-6 跨项目同步 | 架构师 Lead | 2026-09-13 | 8 |
| 9 | commit T-07 类图 + 错误码引用闭环（OpenAPI/proto/alertmanager/Kafka §5.2） | 架构师 Lead | 2026-09-27 | 10 |
| 10 | 启动会纪要 DDD Review 6 角色 | 全员 | 2026-09-06 | — |

---

## 5. 风险与回滚

| # | 风险 | 触发条件 | 影响 | 缓解 / 回滚方案 | 责任 |
|---|------|----------|------|-----------------|------|
| **R-01** | 决议 1+2 升 v2.0 文档化与决议 10 OpenAPI/proto 同步冲突 | T-07 实施时发现 OpenAPI enum 与错误码表 §3 不一致 | 引用链路断 | T-07 启动前先 patch OpenAPI/proto 再升文档；不一致时升错误码表 v1.0 → v1.1 同步 | 架构师 Lead |
| **R-02** | 决议 6 SRE 平台估算 9/2 前未交 | SRE 平台 Lead 9/2 17:00 前未 commit | T-05 CI Pipeline unblock 延迟 | PMO 升级到 Sponsor 直接裁决；SRE 估算延后则 T-05 部分推 K3s 阶段二 | PMO + Sponsor |
| **R-03** | token-OLU v0.1 系数 100K-300K 与 RGS-TS-001 草案冲突 | 5 域 Lead review 时发现跨项目口径差异 | 估时基准争议 | PMO 与 RGS PMO 同步对齐；CATs 仓系数区间保守取值下限 | PMO Lead |
| **R-04** | Sprint 1 4 周窗口 T-07 闭环时间紧 | 决议 10 合并实施时 OI 量超估 | T-07 完成质量降 | T-07 末发现缺口时升 v1.0+1 拆出补做；不退延窗口 | PMO Lead |
| **R-05** | 5 域独立 Lead 一人公司兼任 → RACI 决策延迟 | 决议 1+2+10 同步升 v2.0 时多 Lead 协商超时 | Sprint 1 进度 | RACI SLA 24h（决议 4）+ PMO 24h 升级 | PMO Lead |

---

## 6. 已知缺口（DDD Review 必查 per AI 协作文档治理 2026-08-26）

### 6.1 接口设计书 v2.0 升版过程中跨服务 schema 冲突

- 决议 1 实施时如发现 user-service / auth-service schema 跨服务不一致，走 PMO 升级（24h 内 PMO 升级单 + 5 域 Lead + Sponsor 现场裁决）
- **当前状态**：T-02 落地 user-service 详细 models.rs（commit `89f72cd`）已与 auth-service 错误码表 §3 枚举值对齐（per T-02 commit 注释 ⑤）

### 6.2 token-OLU 系数区间（100K-300K）跨项目引用同步

- 决议 5 立项 CATs token-OLU v0.1 后，RGS-TS-001 草案需同步
- **建议**：PMO Lead 同步通知 RGS PMO 提草案 → 正式 OLU 决议
- **当前状态**：RGS-TS-001 草案不在本 worktree 内（per Sprint 1 §6.5 缺口）

### 6.3 OpenAPI / proto 引用闭环时错误码表 v1.0 §3 枚举需逐一核对

- 决议 10 实施时架构师 + Rust Lead 用脚本生成 enum diff
- 建议工具：`protoc --decode` + `openapi-typescript` 导出 TS enum → diff
- **当前状态**：决议 1+2 升 v2.0 文档时不立即核对，决议 10 阶段统一处理

### 6.4 SRE 平台独立估算未在会议现场提交

- 决议 6 改为 W1 周三（2026-09-02）前 commit
- **当前状态**：per §5.5 升级机制

### 6.5 会议纪要 commit 与议程升版流程

- 本纪要 24h 内 commit（per Sprint 1 §6.6）
- 议程升版：任何决议变更走 v1.0+1（per Sprint 1 拆解 v1.0+1 模式）
- **当前状态**：v1.0 落地中（per commit 即将生成）

### 6.6 RACI Consulted 响应 SLA 模板

- 决议 4 启动会后 1h 起草（截止 2026-08-30 16:30 JST）
- 覆盖 5 域 Lead + SRE 平台共 6 角
- **当前状态**：本纪要 commit 后立即起草

---

## 7. 升版与下次会议

### 7.1 升版触发条件

- 任何决议项变更（推荐选项 / 责任人 / 截止 / 估时）
- DDD Review 6 角色反馈重大修正
- RACI SLA 模板 v1.0 落地后追加引用

### 7.2 升版流程

1. **PR 起草**：变更发起人 + PMO Lead 联合
2. **DDD Review**：6 角色评审（含 Sponsor 本人签）
3. **CAB 决议**：v1.0 → v1.x 走 CAB-002 流程（如决议 1+2 实施时跨服务 schema 冲突）
4. **基线化**：v 升 B-y.y，CAB 决议书 + Baseline 一览同步
5. **引用同步**：议程 v1.0 + 任务拆解 v1.0+1 同步 patch

### 7.3 下次会议

- **Sprint 1 W2 复盘**：2026-09-13（推测性，per 决议 1+2 实施进度调整）
- 议程：决议 1+2 实施进度 + T-03 启动 + OI-6 跨项目同步结果

---

**文档结束（v1.0，2026-08-30 启动会决议归档，10/10 决议通过）**
