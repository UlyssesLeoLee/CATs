# CATs MVP 商业版 — 管理员手册 v1.0

> **文档编号**：CATs-MVP-ADMIN-001
> **版本**：v1.0
> **创建日**：2026-09-19
> **作者**：架构师(Mavis 接手 agent per DEC-008)
> **审批**：架构师(Mavis 接手 agent per DEC-008) + 自审
> **修订人**：Ulysses（一人公司 12 角色 per DEC-008）— Mavis 接手
> **状态**：DDD Review 草稿（6 角色 7 天评审待补）
> **密级**：可对外公开（MVP 客户管理员培训用）

---

## 0. 阅读指南

本书是 CATs MVP 商业版的**管理员手册**，供客户方 Org Admin / Project Admin / Sponsor 角色在日常运维、用户管理、配额监控、审计追溯时使用。所有操作均通过 Next.js 控制台（Web）完成，部分高级操作（数据导入导出、审计查询）通过 REST API + 审计日志完成。

---

## 1. 角色与权限（5 域 Lead + Sponsor + User）

### 1.1 角色一览（per cats-rbac §3 角色枚举 + 权限矩阵 v1.0）

| 角色 | 中文 | 权限范围 | 真人到位情况（MVP）|
|---|---|---|---|
| **Sponsor** | 决策人 | 全部权限 + 决策审批 + 系统配置 | Ulysses 兼（Mavis 代签 per 守门 #14 v3）|
| **ArchitectLead** | 架构师 Lead | 决议 1+2 实施 / 接口设计 + 模块设计审批 | 真人到位前 Mavis 临时代签 |
| **RustLead** | Rust Lead | 16 域 service crate 实施 / 代码评审 | 真人到位前 Mavis 临时代签 |
| **DatabaseLead** | DBA Lead | DB schema 审批 + T-04 EXPLAIN 复核 | 真人到位前 Mavis 临时代签 |
| **QualityLead** | QA Lead | cats-mock 维护 + DDD Review 节奏把控 | 真人到位前 Mavis 临时代签 |
| **PlatformLead** | 平台 Lead | K3s 部署 + 告警规则 | 真人到位前 Mavis 临时代签 |
| **ReviewLead** | 评审主持 Lead | 6 角色 DDD Review 主持 | 真人到位前 Mavis 临时代签 |
| **ProjectLead** | 项目负责人（PM） | Sprint 1 + WBS 跟踪 + 复盘 | 真人到位前 Mavis 临时代签 |
| **SRELead** | SRE Lead | SRE 平台独立估算 + Kafka 物理发布 | 真人到位前 Mavis 临时代签 |
| **User** | 业务用户 | 翻译 / 审校 / 项目内操作 | 真人（客户翻译员）|

> **关键事实**：MVP v1.0 阶段 5 域 Lead 真人到位率 0%（per Sprint 1 复盘 v1.0 §3.2），全部由 Mavis 临时代签 per 守门 #14 v3 + 9/8 15:19 第 6 次强化。真人到位后追溯签字覆盖修订历史。

### 1.2 Org 维度权限矩阵

| 操作 | Sponsor | ArchitectLead | PlatformLead | ProjectLead | User |
|---|---|---|---|---|---|
| 创建 Org | ✅ | ❌ | ❌ | ❌ | ❌ |
| 配置计费 / 套餐 | ✅ | ❌ | ❌ | ❌ | ❌ |
| 创建项目 | ✅ | ❌ | ❌ | ✅ | ⚠（受限）|
| 邀请成员 | ✅ | ✅ | ✅ | ✅ | ❌ |
| 分配角色 | ✅（Org 维度）| ❌ | ❌ | ✅（项目维度）| ❌ |
| 删除项目 | ✅ | ❌ | ❌ | ✅ | ❌ |
| 查看审计日志 | ✅ | ✅ | ✅ | ⚠（项目内）| ❌ |
| 导出审计日志 | ✅ | ✅ | ✅ | ✅ | ❌ |
| 配置告警订阅 | ✅ | ✅ | ✅ | ✅ | ❌ |
| 查看用量报表 | ✅ | ❌ | ❌ | ✅ | ⚠（个人）|
| 数据导入（TM / 术语） | ✅ | ❌ | ❌ | ✅ | ❌ |
| 跨项目共享 TM | ✅ | ❌ | ❌ | ✅ | ❌ |
| 系统配置（K8s / DB） | ✅ | ❌ | ✅ | ❌ | ❌ |

> 详细资源 × 操作矩阵参见《CATs 权限矩阵 v1.0》（commit `03dbede`）。

---

## 2. 项目管理

### 2.1 创建项目

**入口**：控制台 → 左侧导航 → 项目 → 「+ 新建项目」

```
┌────────────────────────────────────────┐
│  新建项目                              │
├────────────────────────────────────────┤
│ 项目名: [仙剑 7 本地化]                │
│ 描述: [第一章主线剧情对白]              │
│ 源语言: [中文（简体）▼]                │
│ 目标语言: [日语 ▼] [英语 ▼] [韩语 ▼]  │
│ 领域: [游戏本地化 ▼]                   │
│ 敏感级别: [普通 ▼] / [合规敏感]        │
│   └─ 合规敏感: 强制本地 LLM 模型       │
│ 术语库: [继承组织默认词库 ▼]           │
│   └─ 或选择: [医学专用术语 v2.0]       │
│ TM 继承: [继承组织 TM ▼]              │
│   └─ 或选择: [空（从零开始）]          │
│ 截止日期: [2026-10-15]                │
│ 分配译员: [搜索...]                    │
│                                        │
│ [取消]                  [创建项目]     │
└────────────────────────────────────────┘
```

**关键字段说明**：

| 字段 | 取值 | 说明 |
|---|---|---|
| 源语言 | BCP-47 代码 | 默认 `zh-CN` |
| 目标语言 | BCP-47 代码（多选）| 支持 `en-US` / `ja-JP` / `ko-KR` / `fr-FR` 等 50+ 语种 |
| 敏感级别 | 普通 / 合规敏感 | 合规敏感 = 强制本地 LLM，fail-closed |
| 术语库 | 继承 / 指定 / 空 | 术语命中强制替换 |
| TM 继承 | 继承 / 指定 / 空 | pgvector 语义召回 |

### 2.2 项目状态机

```
[已创建] ──▶ [进行中] ──▶ [审校中] ──▶ [已完成]
              │              │            │
              └── [暂停]      └── [驳回]   └── [归档]
                  │              │
                  └── [恢复]      └── [回到进行中]
```

| 状态 | 可执行操作 | 自动流转触发 |
|---|---|---|
| 已创建 | 编辑项目信息、分配译员、上传文件 | 上传第一个文件 → 进行中 |
| 进行中 | 翻译、审校、上传新文件 | 全部段译完 → 审校中 |
| 审校中 | 审校、驳回、编辑 | 全部段批准 → 已完成 |
| 已完成 | 导出、归档、复制为新项目 | 手动归档 |
| 暂停 | 恢复、归档 | 手动恢复 |
| 驳回 | 回到进行中、修改译文 | 译员修改后回传 → 审校 |
| 归档 | 只读、导出 | 手动 |

### 2.3 项目内术语管理

**添加术语**：

```
项目详情 → 术语库 → 「+ 添加术语」
├─ 源文: "World"
├─ 目标文: "世界"
├─ 强制替换: ✅
├─ 大小写敏感: ❌
├─ 备注: [仙剑 7 专有术语]
└─ [保存]
```

**批量导入**（CSV / TBX）：

```
项目详情 → 术语库 → 「导入」
├─ 文件: [选择 .csv 或 .tbx]
├─ 列映射: 源文[Column A] / 目标文[Column B] / 强制[Column C]
├─ 冲突策略: [跳过] / [覆盖] / [创建新版本]
└─ [开始导入]
```

**冲突解决**：同名术语不同译法 → 创建版本（v1 / v2 / ...），项目可指定使用哪个版本。

### 2.4 项目内 TM 管理

**查看 TM 命中率**：

```
项目详情 → TM → 「统计」
├─ 总段数: 3,800
├─ Exact Match: 2,847 段 (74.9%)
├─ Fuzzy Match (>= 75%): 482 段 (12.7%)
├─ Semantic Match (>= 0.82): 198 段 (5.2%)
├─ No Match: 273 段 (7.2%)
└─ 命中率: 92.8%
```

**导入历史 TM**：

```
项目详情 → TM → 「导入」
├─ 文件: [选择 .tmx 或 .xliff]
├─ 源语言 / 目标语言: [自动检测 ▼]
├─ 冲突策略: [跳过] / [覆盖] / [追加]
└─ [开始导入]
```

**TM 导出**：

```
项目详情 → TM → 「导出」→ [.tmx] / [.xliff] / [.csv]
```

---

## 3. 术语管理（跨项目 / 全局）

### 3.1 创建全局术语库

**入口**：控制台 → 组织设置 → 术语库 → 「+ 新建术语库」

```
名称: [医学专用术语 v2.0]
描述: [覆盖 ICD-11 / SNOMED CT 命名空间]
共享范围: [组织内所有项目 ▼] / [指定项目...]
审核流程: [Sponsor 审批 ▼] / [Project Lead 直接发布]
版本策略: [SCD Type 2 ▼]（保留历史版本）
```

### 3.2 术语审核（Sponsor）

术语新增 / 修改 / 删除 → 进 Sponsor 待审批队列 → Sponsor 在控制台审批 → 发布生效。

```
审批队列:
├─ #1234 添加术语 "COVID-19" → "新冠" 申请人: Alice 紧急度: 高
├─ #1235 修改术语 "World" → "世界" 申请人: Bob 原因: v1 译法不准
└─ #1236 删除术语 "deprecated_term" 申请人: Carol 原因: 项目下线
```

### 3.3 术语版本管理（SCD Type 2）

每条术语保留完整历史版本，项目可指定使用哪个版本：

```
术语 "World" 的版本:
├─ v3 (2026-09-15, 当前): "World" → "世界"
├─ v2 (2026-06-01): "World" → "全球"
└─ v1 (2026-03-01): "World" → "天下"
```

---

## 4. 用户管理

### 4.1 邀请成员

```
组织设置 → 成员 → 「+ 邀请成员」
├─ 邮箱: [alice@lingo-game.com]
├─ 角色: [User ▼] / [Project Lead] / [Sponsor]
├─ 项目分配: [仙剑 7] [商品详情页]
├─ 邀请消息: [可选]
└─ [发送邀请]
```

邀请邮件 7 天有效，逾期自动失效。接受邀请后用户进组织 + 自动分配项目。

### 4.2 角色变更

```
组织设置 → 成员 → 选择用户 → 「编辑角色」
├─ 当前角色: User
├─ 新角色: Project Lead
├─ 影响范围: [组织维度] / [项目维度]
├─ 生效时间: [立即生效 ▼] / [指定日期]
└─ [确认]
```

> 角色变更进审计日志，旧角色会话立即失效（per auth-service 设计）。

### 4.3 停用 / 删除成员

```
组织设置 → 成员 → 选择用户 → 「停用」
├─ 立即撤销: 所有 JWT / Refresh Token
├─ 数据保留: 译文 / TM 回存保留（关联到匿名用户）
├─ 审计保留: 历史操作记录保留 7 年（合规要求）
└─ [确认停用]
```

> 「删除」与「停用」区别：删除 = 7 天后匿名化个人信息（per GDPR）；停用 = 仅撤销访问权，数据保留。

---

## 5. 审计日志

### 5.1 审计事件类型（28 条 snake_case 错误码 × 14 类业务事件）

**业务事件类型**：

| 类别 | 事件 | 进 audit_db | 进 Kafka topic |
|---|---|---|---|
| 用户管理 | user.created / user.deleted / user.role_changed | ✅ | `audit.events` |
| 组织管理 | org.member_added / org.member_removed / org.subscription_changed | ✅ | `audit.events` |
| 项目管理 | project.created / project.archived / project.sensitive_flag_changed | ✅ | `audit.events` |
| 翻译操作 | translation.segment_edited / translation.tm_saved / translation.completed | ✅ | `audit.events` |
| 术语操作 | glossary.added / glossary.modified / glossary.deleted | ✅ | `audit.events` |
| 模型调用 | ai.model_invoked / ai.tokens_consumed / ai.fail_closed_triggered | ✅ | `audit.events` |
| 鉴权 | auth.login_success / auth.login_failed / auth.token_refresh | ✅ | `audit.events` |
| 文件操作 | file.uploaded / file.downloaded / file.deleted | ✅ | `audit.events` |

### 5.2 查看审计日志

```
控制台 → 组织设置 → 审计 → 筛选
├─ 时间范围: [2026-09-01] 至 [2026-09-19]
├─ 事件类型: [所有 ▼] / [用户管理] / [翻译操作] / ...
├─ 操作用户: [搜索...]
├─ 资源类型: [所有 ▼] / [Project] / [User] / ...
├─ 严重度: [所有 ▼] / [Info] / [Warning] / [Critical]
└─ [查询]
```

### 5.3 审计事件样例

```json
{
  "event_id": "ev_8a7f3b9c4d2e1f0a",
  "event_type": "user.role_changed",
  "severity": "info",
  "actor": {
    "user_id": "u_alice_001",
    "email": "alice@lingo-game.com",
    "ip": "10.20.30.45",
    "user_agent": "Tauri/1.0.0 (Windows NT 10.0)"
  },
  "resource": {
    "type": "User",
    "id": "u_bob_002"
  },
  "action": {
    "type": "role_changed",
    "before": "User",
    "after": "ProjectLead"
  },
  "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
  "occurred_at": "2026-09-19T08:15:23Z",
  "metadata": {
    "reason": "晋升",
    "approved_by": "u_sponsor_001"
  }
}
```

### 5.4 审计日志导出

```
控制台 → 组织设置 → 审计 → 「导出」
├─ 时间范围: [2026-09-01] 至 [2026-09-30]
├─ 格式: [JSON Lines ▼] / [CSV]
├─ 包含字段: [全字段] / [自定义]
├─ 加密: [AES-256-GCM ▼]
└─ [生成导出] → 走 MinIO 预签名 URL 下载（7 天有效期）
```

### 5.5 审计保留策略

- **保留时长**：合规要求 7 年（per 中国网络安全法 / GDPR）
- **存储**：audit_db 表分区（按月）+ 冷数据归档到 MinIO + 定期清理热数据
- **完整性**：每条审计事件带 HMAC-SHA256 签名（per audit-service 设计 §3.5）

---

## 6. 配额监控 + 告警订阅

### 6.1 用量配额维度

| 维度 | 单位 | 配额检查点 |
|---|---|---|
| 用户数 | seats | user-service 创建用户时 |
| 月翻译段数 | segments / 月 | task-service 创建任务时 |
| 存储容量 | GB | file-service 上传文件时 |
| LLM Token 数 | tokens / 月 | ai-gateway 调用时 |
| 媒体处理分钟数 | minutes / 月 | asr-service / ocr-service 调用时 |

### 6.2 配额监控仪表盘

```
控制台 → 组织设置 → 配额 → 当前周期 (2026-09)
┌────────────────────────────────────────────────┐
│  用户数: 18 / 20 (90%)         ⚠ 接近上限       │
│  月翻译段数: 412,000 / 1,000,000 (41%)        │
│  存储容量: 12.4 GB / 50 GB (25%)              │
│  LLM Tokens: 5.2M / 50M (10%)                 │
│  媒体处理分钟数: 240 / 5,000 min (5%)          │
└────────────────────────────────────────────────┘
```

### 6.3 告警订阅

```
控制台 → 组织设置 → 配额 → 告警订阅 → 「+ 添加规则」
├─ 规则名: [用户数接近上限]
├─ 维度: [用户数 ▼]
├─ 阈值: [>= 90%]
├─ 持续时间: [持续 5 分钟]
├─ 严重度: [Warning ▼] / [Critical]
├─ 通知渠道:
│   ├─ [✅] 邮件: [admin@lingo-game.com]
│   ├─ [✅] Slack: [#cats-alerts]
│   ├─ [ ] Webhook: [https://...]
│   └─ [ ] 短信: [+86 138...]
├─ 通知频率: [立即 ▼] / [聚合 5min] / [聚合 1h]
└─ [保存]
```

### 6.4 内置告警规则（4 alertmanager rules，per 告警规则 v1.0）

| Rule | 阈值 | 持续时间 | 严重度 |
|---|---|---|---|
| `CatsServiceUnhealthy` | K8s pod not ready | 5min | warning |
| `CatsKafkaConsumerLag` | consumer lag > 10000 | 10min | warning |
| `CatsPGConnectionsHigh` | pg_stat_activity count > 80% pool size | 5min | critical |
| `CatsJWTExpiryHigh` | exp < 24h 比例 > 10% | 5min | warning |

> 详细规则 per `doc/05-其他/可观测性/CATs_告警规则_v1.0.md`（commit `1d8926d`）。v1.1 扩展到 12 条。

---

## 7. 系统配置（Sponsor 专属）

### 7.1 计费配置

```
控制台 → 组织设置 → 计费
├─ 套餐: [Enterprise ▼]
├─ 计费周期: [月度 ▼] / [季度] / [年度]
├─ 付款方式: [银行转账 ▼] / [Stripe]
├─ 发票抬头: [灵游游戏工作室]
├─ 税号: [91110000...]
└─ [保存]
```

### 7.2 敏感策略配置

```
控制台 → 组织设置 → 合规策略
├─ 默认敏感级别: [普通 ▼]
├─ 强制本地 LLM 模型: [Qwen2.5-7B-Local ▼]
├─ 合规 fail-closed: [✅ 启用]
├─ 数据保留策略: [7 年 ▼]
├─ GDPR 匿名化窗口: [删除 30 天后 ▼]
└─ [保存]
```

### 7.3 SSO / OIDC 配置（Enterprise）

```
控制台 → 组织设置 → 身份认证 → 「启用 SSO」
├─ 提供商: [Okta ▼] / [Azure AD] / [自建 OIDC]
├─ Issuer URL: [https://lingo-game.okta.com]
├─ Client ID: [...    ]
├─ Client Secret: [***]（加密存储）
├─ 默认角色映射: [User ▼]
├─ 强制 SSO: [✅ 所有用户必须 SSO 登录]
└─ [保存] → 测试连接 → 启用
```

---

## 8. 已知缺口（v1.0 MVP 管理）

| # | 缺口 | 临时方案 | GA 时间 |
|---|---|---|---|
| A-1 | 全局 TM 共享仅限组织内 | 客户需在组织内规划 | v1.1（跨组织共享 + Marketplace）|
| A-2 | 告警规则仅 4 条 | 手动监控 | v1.1 扩展 12 条 |
| A-3 | 审计日志查询无全文检索 | 走 PostgreSQL `tsvector` + 客户端过滤 | v1.1（集成 Elasticsearch）|
| A-4 | 计费仅支持银行转账 / Stripe | 客户需联系销售 | v1.1（添加支付宝 / 微信）|
| A-5 | SSO 仅支持 OIDC | SAML v1.1 GA | v1.1 |

---

## 9. 关联文档（git 实证）

| 引用文档 | 路径 | commit hash | 用途 |
|---|---|---|---|
| 权限矩阵 v1.0 | `doc/05-其他/管理/CATs_权限矩阵_v1.0.md` | `03dbede` | 5 域 Lead + Sponsor + User 角色矩阵 |
| cats-rbac README | `crates/cats-rbac/README.md` | `f417407` | RBAC 中间件代码层 + 代签机制 |
| 接口设计书 v2.0+2 | `doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md` | `1f3c94d` | §3 9 服务 API + §6 端到端 |
| 错误码表 v1.0.1 | `doc/05-其他/管理/CATs_错误码表_v1.0.1.md` | `a4d8b86` | 28 条 snake_case 错误码 |
| 告警规则 v1.0 | `doc/05-其他/可观测性/CATs_告警规则_v1.0.md` | `1d8926d` | 4 alertmanager rules |
| 测试 Mock 项目设计书 v1.0 | `doc/02-基础设计/测试/CATs_测试Mock项目设计书_v1.0.md` | `2fc3d96` | §6 cats-mock 4 大模块 |

---

## 10. 修订历史

| 版本 | 日期 | 修订人 | 修订内容 | 触发 |
|---|---|---|---|---|
| v1.0 | 2026-09-19 | 架构师(Mavis 接手 agent per DEC-008) | 初版：5 域 Lead + Sponsor + User 角色矩阵 + 项目/术语/用户/审计/配额/告警/SSO 7 章节 + 28 类审计事件 + 4 alertmanager rules + 5 已知缺口 + 6 条引用 git 实证 | V1.1-PLAN §2.1 Sprint 2 MVP 商业版落地 |