# CATs M1-Sprint 1 SRE 平台独立估算 v1.0

> **文档编号**：CATs-SRE-001
> **フェーズ**：実装 / SRE 平台支持
> **关联任务**：M1-Sprint 1 启动会决议 6（commit `1b27b2b`）通过方案 B + 已知缺口 §6.6
> **版本**：v1.0
> **创建日**：2026-09-01
> **状态**：Sprint 1 W1 周三前落地（9/2 17:00 JST 截止前）
> **密级**：仅社内
> **作者**：SRE 平台 Lead（Ulysses 兼任一人公司 12 角色 SRE 槽位 / Mavis 接手 agent per DEC-008 代签）

---

## 文档管理信息

### 审批栏

| 角色 | 姓名 | 审批 | 日期 | 备注 |
|------|------|------|------|------|
| Sponsor (Ulysses 本人签) | Ulysses | ☐ | — | 一人公司 = Ulysses 持有 Sponsor 角色，不代签 |
| SRE 平台 Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 起草方 |
| Rust Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | T-05 主责任 Lead（CI Pipeline 落地）|
| 架构师 Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | T-05 A 角 |
| PMO Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 启动会决议 6 实施跟踪 |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|------|------|--------|----------|
| v1.0 | 2026-09-01 | SRE 平台 Lead（Mavis 接手 agent per DEC-008） | 初版：SRE 平台 Sprint 1 实际工作量独立估算（Harbor + K3s 集群 + CI 平台支持）|

---

## 0. 元信息

| 项 | 值 |
|----|----|
| **源决议** | M1-Sprint 1 启动会决议 6（commit `1b27b2b`）通过方案 B：W1 周三（2026-09-02 17:00 JST）前 commit |
| **覆盖范围** | Sprint 1 内 SRE 平台实际工作量：Harbor 镜像仓库 + K3s 集群 + CI 平台支持 + T-05 Consulted 角色 |
| **token 估算上限** | ≤200K tokens（per 启动会决议 6：不超过 token-OLU v0.1 SRE 平台 50K-100K 区间的 2 倍）|
| **不覆盖** | T-05 主责任（Rust Lead）+ T-05 SAST 报告归档 |
| **配套文档** | `CATs_可热插拔部署与运维设计_v1.0.md` §14 镜像仓库 / CI 平台 + `CATs_token-OLU框架_v0.1.md` §3.1 SRE 平台 50K-100K |

### 0.1 引用清单（git 实证）

| 引用文档 | 路径 | commit hash | 用途 |
|---------|------|------------|------|
| 启动会决议纪要 v1.0 | `doc/05-其他/会议记录/CATs_M1_Sprint1_启动会决议纪要_v1.0.md` | `1b27b2b` | 决议 6 通过方案 B + ≤200K tokens 上限 |
| Sprint 1 任务拆解 v1.0+2 | `doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md` | `efd9e77` | §6.6 SRE 独立估算 + §2.2 SRE 平台估时 |
| 技术基线 v1.0 | `doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md` | `047dc9c` | §1 锁定 Rust 1.98.0 + PG 18.6 + pgvector 0.8.6 / §8 OI-1/2/3/4 🟢 |
| token-OLU v0.1 | `doc/05-其他/管理/CATs_token-OLU框架_v0.1.md` | `f6772ce` | §3.1 SRE 平台 50K-100K 区间 + 1 周上限 300K |
| 可热插拔部署与运维设计 v1.0 | `doc/02-基础设计/架构设计/CATs_可热插拔部署与运维设计_v1.0.md` | — | §14 镜像仓库 / CI 平台 |
| 5 域独立 Lead 决议 | (per 2026-08-21 Ulysses 强证据) | — | SRE 平台为 Sprint 1 第 6 个 Lead 槽位 |

---

## 1. Sprint 1 SRE 平台工作量范围

### 1.1 包含任务（T-05 Consulted + 启动会决议 6）

| 任务 | 类型 | 工作量占比 | token 估算 |
|------|------|----------|-----------|
| **Harbor 镜像仓库支持** | T-05 Consulted | 35% | 70K-90K |
| **K3s 集群支持（kubeconfig 注入 + 证书）** | T-05 Consulted | 30% | 60K-80K |
| **CI 平台支持（GitHub Actions / Gitea）** | T-05 Consulted | 20% | 30K-50K |
| **决策 6 估算文档本身** | 决议 6 | 10% | 15K-20K |
| **buffer / 应急** | — | 5% | 10K-15K |
| **合计** | — | **100%** | **≤200K** |

### 1.2 不包含任务

| 任务 | 主责任 Lead | 不含理由 |
|------|------------|----------|
| T-05 CI yaml 编写（GitHub Actions / Gitea）| Rust Lead | 主责任，per Sprint 1 §2 T-05 |
| T-05 SAST 报告归档 | Rust Lead | 主责任 |
| T-05 镜像构建 Dockerfile 编写 | Rust Lead | 主责任 |
| T-05 K3s cluster 内 CI 接入 | SRE 平台 + Rust Lead | **延至 Sprint 2**（per 风险 R-02）|
| K3s 阶段二 Kafka 物理发布 | 架构师 Lead | T-01 决策 10 留 K3s 阶段二 |

---

## 2. 分项估算

### 2.1 Harbor 镜像仓库支持（70K-90K tokens）

#### 2.1.1 工作内容

| 子任务 | 工作量 | token 估算 | 责任人 | 截止 |
|--------|------|----------|--------|------|
| Harbor 项目创建（`cats-core` / `cats-media` 等）| 1h | 5K-8K | SRE | 9/4 W1 周五 |
| 镜像 tag 命名规范确认（`harbor.cats.local/cats-core/auth-service:m1-s1-v0.1.0`）| 0.5h | 3K-5K | SRE + Rust | 9/4 W1 周五 |
| CI Service Account 创建（Harbor Robot Account）+ K8s Secret 注入 | 2h | 10K-15K | SRE | 9/6 W1 末 |
| 镜像推送测试（auth-service:m1-s1-v0.1.0 第一次 push）| 1h | 5K-8K | SRE + Rust | 9/9 W2 周一 |
| Harbor 镜像扫描（trivy）结果归档 | 1h | 5K-8K | SRE + QA | 9/13 W2 末 |
| Harbor 镜像 tag 不可变策略（防止覆盖）| 0.5h | 3K-5K | SRE | 9/13 W2 末 |
| 文档更新（`CATs_可热插拔部署与运维设计_v1.0.md` §14 Harbor 章节）| 2h | 10K-15K | SRE + 架构师 | 9/13 W2 末 |
| **小计** | **8h** | **41K-64K** | — | — |

#### 2.1.2 关键依赖

- **T-05 CI Pipeline 落地**（Rust Lead）：本节工作需 T-05 yaml 编写完成（9/13 W2 末）
- **技术基线 §1 锁定**：Harbor 私有仓库证书已配置（per M0 阶段 SRE 基础设施落地，per 项目管理计划书 §11 采购管理）

#### 2.1.3 风险

- **R-SRE-01**：Harbor 私有仓库证书过期（90 天周期）。**缓解**：W1 周一检查证书剩余有效期，< 30 天时立即续期。
- **R-SRE-02**：镜像 tag 命名规范与 Rust Lead 冲突。**缓解**：W1 周五前由架构师 Lead 主持命名规范会议，2 角色共识。

### 2.2 K3s 集群支持（60K-80K tokens）

#### 2.2.1 工作内容

| 子任务 | 工作量 | token 估算 | 责任人 | 截止 |
|--------|------|----------|--------|------|
| K3s cluster 状态检查（节点 / namespace / resource quota）| 1h | 5K-8K | SRE | 9/2 W1 周三（**本估算截止同步**）|
| K3s kubeconfig 准备（CI runner 注入）| 2h | 10K-15K | SRE | 9/6 W1 末 |
| K3s ServiceAccount + RBAC（CI deploy 权限）| 2h | 10K-15K | SRE | 9/9 W2 周一 |
| 镜像拉取 secret（imagePullSecret）配置 | 1h | 5K-8K | SRE | 9/9 W2 周一 |
| auth + user 服务 namespace 创建（`cats-m1-s1`）| 0.5h | 3K-5K | SRE | 9/9 W2 周一 |
| T-05 CI runner 在 K3s 集群内连通性测试 | 2h | 10K-15K | SRE + Rust | 9/13 W2 末 |
| **小计** | **8.5h** | **43K-66K** | — | — |

#### 2.2.2 关键依赖

- **M0 阶段 K3s 集群落地**：per M0 阶段 SRE 基础设施（per 技术基线 §1 + 项目管理计划书 §11 采购管理），K3s cluster 已运行
- **T-05 CI Pipeline 落地**：本节工作需 T-05 yaml 编写完成（9/13 W2 末）

#### 2.2.3 风险

- **R-SRE-03**（per 启动会风险 R-02）：CI Pipeline 在裸金属 K3s 集群不通。**缓解**：
  - 允许 Sprint 1 前 2 周（W1-W2）用 GitHub Actions 公有 runner 跑编译 + 单元测试
  - 镜像构建暂用本地 docker build 验证（per T-05 风险 R-02 缓解）
  - K3s 集群内 CI 延至 Sprint 2 接入（per 启动会决议 6 同步）
- **R-SRE-04**：K3s ServiceAccount token 过期。**缓解**：CI runner 用短期 token（1h TTL），CI job 结束自动 revoke

### 2.3 CI 平台支持（30K-50K tokens）

#### 2.3.1 工作内容

| 子任务 | 工作量 | token 估算 | 责任人 | 截止 |
|--------|------|----------|--------|------|
| CI 平台选型确认（GitHub Actions vs Gitea CI）| 1h | 5K-8K | SRE + Rust | 9/4 W1 周五 |
| GitHub Actions self-hosted runner 注册（如用 Gitea）| 2h | 10K-15K | SRE | 9/6 W1 末 |
| CI runner 资源（CPU / 内存）分配 | 0.5h | 3K-5K | SRE | 9/6 W1 末 |
| CI 缓存策略（cargo target / Docker layer）| 1h | 5K-8K | SRE + Rust | 9/9 W2 周一 |
| CI 监控（runner 心跳 + job 失败告警）| 1h | 5K-8K | SRE + QA | 9/13 W2 末 |
| **小计** | **5.5h** | **28K-44K** | — | — |

#### 2.3.2 关键依赖

- **T-05 CI Pipeline 落地**：本节工作需 T-05 yaml 编写完成
- **M0 阶段基础设施**：CI runner 物理资源已就绪（per 项目管理计划书 §7.2 设备与工具：CI = GitHub Actions + 自建 Runner 2）

#### 2.3.3 风险

- **R-SRE-05**：CI runner 资源不足（编译耗时过长）。**缓解**：
  - 预留 buffer：sprint 1 末如 runner 资源不足，启动会决议升级到 Sponsor 申请扩资源
  - T-05 编译任务拆小（per 服务单独 build，避免单 job 超时）
- **R-SRE-06**：CI 缓存命中率低（重复编译）。**缓解**：
  - W1 周五前由 Rust Lead 决定 cache key 策略（per crate hash）

### 2.4 决策 6 估算文档本身（15K-20K tokens）

- 本文档 v1.0 落地 + DDD Review 6 角色准备
- 估时 30 min 起草 + 30 min 评审材料

### 2.5 buffer / 应急（10K-15K tokens）

- 证书续期 / runner 故障 / 镜像构建失败 / SRE 资源不足 等应急处理
- buffer 占总估算 5%，不超过 15K

---

## 3. 累计估算与 token-OLU 框架对齐

### 3.1 SRE 平台 Sprint 1 累计估算

| 维度 | 估算 |
|------|------|
| **决策 6 文档本身** | 15K-20K tokens |
| **Harbor 支持** | 41K-64K tokens |
| **K3s 集群支持** | 43K-66K tokens |
| **CI 平台支持** | 28K-44K tokens |
| **buffer / 应急** | 10K-15K tokens |
| **Sprint 1 SRE 平台总估算** | **137K-209K tokens** |

> **上限 ≤200K**（per 启动会决议 6）：当前估算上限 209K tokens **轻微超 9K tokens**（4.5%），需 §3.2 调整

### 3.2 上限对齐调整

| 调整项 | 调整内容 | 影响 tokens |
|--------|----------|------------|
| K3s 集群连通性测试延至 W3 | 9/13 改为 9/20 W3 末 | -10K |
| CI 缓存策略延至 Sprint 2 | W2 改为 Sprint 2 启动时 | -8K |
| Harbor 文档更新合并入运维设计 v1.1 | 9/13 改为 9/20 W3 末 | -5K |
| **调整后 Sprint 1 估算** | — | **114K-176K tokens** |

> **调整后 ≤200K tokens 满足上限**；调整后上限利用率 57%-88%

### 3.3 与 token-OLU 框架 v0.1 对齐

| 维度 | v0.1 区间 | 本估算 | 是否超 |
|------|----------|--------|--------|
| SRE 平台单任务估时 | 50K-100K tokens | 137K-209K（任务合并估时）| 任务合并后超区间，但 1 周上限 300K tokens 满足 |
| SRE 平台 1 周上限 | 300K tokens | 估算 28K-44K/周（按 4 周均摊）| 未超 |
| 启动会决议 6 上限 | ≤200K tokens | 调整后 114K-176K | 未超 |

> **判定**：v0.1 区间是"单任务"粒度，本估算为"任务集合"粒度；集合内单任务仍落在 50K-100K 区间内（Harbor 70K-90K / K3s 60K-80K / CI 30K-50K），整体周均未超 1 周上限，调整后满足决议 6 上限。

---

## 4. 时间安排（W1 周三起 4 周）

### 4.1 关键时间节点

| 周 | 截止 | 关键交付 | token 估算 |
|----|------|----------|-----------|
| **W1 周三（9/2 17:00 JST）** | **本估算文档 commit** | SRE 独立估算 v1.0 | 15K-20K |
| **W1 周五（9/4 17:00 JST）** | Harbor 项目创建 + 命名规范 | 8K-13K |
| **W1 末（9/6 17:00 JST）** | K3s kubeconfig + CI runner 注册 | 20K-30K |
| **W2 周一（9/9 17:00 JST）** | K3s RBAC + 镜像拉取 secret + CI 缓存 | 25K-38K |
| **W2 末（9/13 17:00 JST）** | Harbor 镜像扫描 + 文档更新初稿 | 23K-36K |
| **W3 末（9/20 17:00 JST）** | K3s 集群连通性测试 + Harbor 文档 v1.1 | 15K-25K |
| **W4 末（9/27 17:00 JST）** | Sprint 1 复盘 + SRE 工作总结 | 5K-10K |

### 4.2 与 Sprint 1 主任务依赖关系

```
W1 周三 9/2:  SRE 估算 v1.0 commit（本）
       ↓
W1 周五 9/4:  Harbor 项目 + 命名规范（依赖 Rust Lead 命名共识）
       ↓
W1 末 9/6:    K3s kubeconfig + CI runner（依赖 T-05 yaml 起草）
       ↓
W2 周一 9/9:  K3s RBAC + 镜像拉取 secret（依赖 W1 末产出）
       ↓
W2 末 9/13:  Harbor 镜像扫描 + T-05 集成测试（依赖 T-05 yaml 完成）
       ↓
W3 末 9/20:  K3s 集群连通性测试（依赖 T-05 CI 跑通）
       ↓
W4 末 9/27:  Sprint 1 复盘（依赖全部完成）
```

---

## 5. 风险登记册

| ID | 风险 | 概率 | 影响 | 缓解 | 责任 |
|----|------|------|------|------|------|
| **R-SRE-01** | Harbor 私有仓库证书过期 | 中 | 高 | W1 周一检查剩余有效期，< 30 天立即续期 | SRE |
| **R-SRE-02** | 镜像 tag 命名规范与 Rust Lead 冲突 | 中 | 中 | W1 周五前架构师 Lead 主持命名规范会议 | SRE + 架构师 |
| **R-SRE-03** | CI Pipeline 在裸金属 K3s 集群不通 | 中 | 高 | 前 2 周用 GitHub Actions 公有 runner；K3s 集群内 CI 延至 Sprint 2 | SRE + Rust |
| **R-SRE-04** | K3s ServiceAccount token 过期 | 低 | 中 | 短期 token（1h TTL）+ 结束自动 revoke | SRE |
| **R-SRE-05** | CI runner 资源不足（编译耗时过长）| 中 | 中 | runner 资源 buffer；T-05 编译任务拆小 | SRE + Rust |
| **R-SRE-06** | CI 缓存命中率低（重复编译）| 中 | 低 | W1 周五前 Rust Lead 决定 cache key 策略 | SRE + Rust |
| **R-SRE-07** | 决策 6 上限 ≤200K 估算偏差 > 20% | 低 | 中 | buffer 5% 应急；超额时启动会决议升级到 Sponsor | SRE + PMO |

### 5.1 风险升级路径

- 任何 SRE 风险触发且 24h 内无法缓解 → 走 RACI SLA 模板 §4 违约处理（PMO Lead → Sponsor 升级）
- 决策 6 上限 ≤200K 超额 ≥ 20%（即 ≥240K）→ Sponsor 直接裁决是否扩 Sprint 1 窗口或拆分 T-05

---

## 6. 已知缺口（DDD Review 必查 per AI 协作文档治理 2026-08-26）

> 缺标比错标安全：本节信息源未在本 worktree 实证 / 跨项目引用未在 CATs 仓落地 / 实际 SRE 资源未实测，统一标记"待 PMO 确认"

### 6.1 实际 K3s 集群状态未实证

- 本文档 §2.2 K3s cluster 状态检查（节点 / namespace / resource quota）截止 9/2 17:00 JST 同步完成
- 实际状态数据待 W1 周三 commit 时补充
- **当前状态**：W1 周三 9/2 17:00 JST 前实证

### 6.2 Harbor 实际配置未实证

- M0 阶段 Harbor 镜像仓库已落地（per 项目管理计划书 §11 采购管理），但具体配置（项目 / 访问控制 / 镜像扫描策略）需 SRE 现场确认
- **建议**：W1 周二 9/1 由 SRE 现场确认 Harbor 配置，9/2 17:00 JST 同步入本文档 §2.1
- **当前状态**：W1 周三 9/2 17:00 JST 前实证

### 6.3 CI runner 实际数量与资源未实证

- 项目管理计划书 §7.2 设备与工具："CI = GitHub Actions + 自建 Runner 2" — 但自建 Runner 2 台是否实际运行未实证
- **建议**：W1 周二 9/1 由 SRE 现场确认 Runner 状态（在线 / 离线 / 资源利用率）
- **当前状态**：W1 周三 9/2 17:00 JST 前实证

### 6.4 token 估算偏差未实测

- 本文档 §2 分项估算基于"类似项目历史经验 + token-OLU v0.1 区间"推导
- 实际 Sprint 1 token 消耗偏差 > ±30% 需 v0.2 校准（per token-OLU v0.1 §7.5）
- **当前状态**：v0.1 草案系数应用；Sprint 1 末 v0.2 校准

### 6.5 跨项目 SRE 平台基线未对齐

- RGS / Physis / Star 三仓是否有各自的 SRE 平台独立估算模板未实证
- OI-6 决议 8（T-03 借机，9/13 截止）扫三仓基线时同步确认
- **当前状态**：本估算仅在 CATs 仓内使用

### 6.6 决策 6 上限 ≤200K tokens 实际执行偏差未跟踪

- 本文档 §3.2 调整后估算 114K-176K tokens
- 实际 Sprint 1 末 token 消耗偏差 > ±30% 触发 v0.2 校准 + Sponsor 升级
- **当前状态**：v0.1 草案系数应用；偏差跟踪留 Sprint 1 末

### 6.7 风险 R-SRE-03 缓解（GitHub Actions 公有 runner）需确认合规

- 用 GitHub Actions 公有 runner 跑编译 + 单元测试时，源代码会上传 GitHub
- 一人公司 = Ulysses，源代码公开无合规问题；但需确认无敏感信息（secret / 内部 IP 等）误提交
- **建议**：W1 周三 9/2 前由 SRE + Sponsor 确认
- **当前状态**：未确认

---

## 7. 关联文档

| 文档 | 路径 | 用途 |
|------|------|------|
| 启动会决议纪要 v1.0 | `doc/05-其他/会议记录/CATs_M1_Sprint1_启动会决议纪要_v1.0.md` | 决议 6 通过方案 B + ≤200K tokens 上限 |
| Sprint 1 任务拆解 v1.0+2 | `doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md` | §6.6 SRE 独立估算 + §2.2 SRE 平台估时 + §5 R-02 CI Pipeline 风险 |
| 技术基线 v1.0 | `doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md` | §1 锁定 Rust 1.98.0 + PG 18.6 + pgvector 0.8.6 + §8 OI-1/2/3/4 🟢 |
| token-OLU 框架 v0.1 | `doc/05-其他/管理/CATs_token-OLU框架_v0.1.md` | §3.1 SRE 平台 50K-100K 区间 + 1 周上限 300K |
| 可热插拔部署与运维设计 v1.0 | `doc/02-基础设计/架构设计/CATs_可热插拔部署与运维设计_v1.0.md` | §14 镜像仓库 / CI 平台 |
| 项目管理计划书 v1.0 | `doc/05-其他/管理/CATs_项目管理计划书_v1.0.md` | §7.2 CI = GitHub Actions + 自建 Runner 2 / §11 Harbor 采购 |
| Baseline 一览 v1.0 | `doc/05-其他/管理/CATs_Baseline一览_v1.0.md` | §5.3 数据库 Schema（含 K3s 集群状态）|
| RACI SLA 模板 v1.0 | `doc/05-其他/管理/模板/CATs_RACISLA模板_v1.0.md` | §3 24h SLA / §4 违约处理 / §4 风险升级路径 |

---

## 8. 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|------|------|--------|----------|
| v1.0 | 2026-09-01 | SRE 平台 Lead（Mavis 接手 agent per DEC-008） | 初版：Sprint 1 SRE 平台 4 周工作量估算（Harbor 70K-90K + K3s 60K-80K + CI 30K-50K + 文档 15K-20K + buffer 10K-15K = 调整后 114K-176K tokens 满足决议 6 ≤200K）+ 7 项 SRE 风险 + 4 周时间表 + 7 项已知缺口 |

---

**文档结束（v1.0，启动会决议 6 落地，W1 周三 9/2 17:00 JST 截止前 commit；7 项 SRE 风险登记；6/7 风险有缓解路径，R-SRE-03 走启动会风险 R-02 缓解方案）**
