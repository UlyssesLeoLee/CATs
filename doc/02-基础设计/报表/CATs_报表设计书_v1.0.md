# CATs 报表设计書 v1.0

> **文档编号**：CATs-BD-027（CATs 报表设计）  
> **フェーズ**：27 帳票設計  
> **关联任务**：150 任务 #27  
> **版本**：v1.0（评审会前草稿）  
> **创建日**：2026-08-20  
> **作者**：BA + 架构师

---

## 文档管理信息

### 审批栏

| 角色 | 审批 | 日期 |
|------|------|------|
| BA | ☐ | — |
| 架构师 | ☐ | — |
| 客户代表 | ☐ | — |
| PM | ☐ | — |

### 修订履历

| 版本 | 日期 | 修订人 | 修订内容 |
|------|------|--------|----------|
| **v1.0** | **2026-08-20** | **BA** | **评审前草稿：基于 F1-F11 需求 + 接口设计** |

---

## 1. 目的

定义 CATs 系统所有**报表/导出/打印**类输出的格式与生成规则，作为：

- 项目报告（PM/月度）的基线
- UAT 验收（任务 90-95）的依据
- 客户交付物格式的基线

---

## 2. 范围

### 2.1 包含

| 类别 | 项 |
|------|-----|
| 业务报表 | 项目报告、翻译进度、术语统计、TM 命中率 |
| 运维报表 | 监控周报、可用性月报、SLA 报告 |
| QA 报表 | 缺陷报告、覆盖率报告、测试进度 |
| 财务报表 | 工时报告、成本报告（可选） |
| 导出格式 | PDF / Excel / CSV / JSON / XLIFF / TBX |
| 行业标准 | XLIFF 1.2/2.1、TBX、TMX、SRX |

### 2.2 不包含

- 实时仪表盘（属监控范畴）
- 用户自定义报表（v2.0）
- 商业 BI 集成（v2.0）

---

## 3. 业务报表

### 3.1 项目报告

| 字段 | 类型 | 来源 |
|------|------|------|
| 项目 ID | string | project-svc |
| 项目名 | string | project-svc |
| 客户 | string | user-svc |
| 状态 | enum | project-svc |
| 起止日期 | date | project-svc |
| 总字数 | int | analytics-svc |
| 已翻译 | int | analytics-svc |
| 已审校 | int | analytics-svc |
| 完成率 | float | 计算 |
| TM 命中率 | float | analytics-svc |
| 术语覆盖率 | float | analytics-svc |
| 预计完成日 | date | 估算 |
| PM | string | user-svc |
| 团队 | list | project-svc |

**格式**：Excel + PDF  
**频度**：每周 / 每月 / 里程碑  
**订阅**：PM + 客户 + Sponsor

### 3.2 翻译进度日报

| 字段 | 类型 | 来源 |
|------|------|------|
| 日期 | date | analytics-svc |
| 译者 | string | user-svc |
| 项目 | string | project-svc |
| 翻译字数 | int | analytics-svc |
| 审校字数 | int | analytics-svc |
| TM 命中 | float | analytics-svc |
| LLM 使用 | int | analytics-svc |
| 工时 | float | time-track |

**格式**：Excel + Dashboard  
**频度**：每日 18:00 自动生成  
**订阅**：PM + 团队 lead

### 3.3 术语统计

| 字段 | 类型 | 来源 |
|------|------|------|
| 项目 | string | project-svc |
| 术语库 | string | term-svc |
| 总术语数 | int | term-svc |
| 已使用 | int | analytics-svc |
| 使用率 | float | 计算 |
| 缺失术语 | list | analytics-svc |
| 来源 | enum | term-svc |
| 创建人 | string | user-svc |
| 更新日期 | date | term-svc |

**格式**：Excel + CSV  
**频度**：每周 / 每月 / 季度

### 3.4 TM 命中率报告

| 字段 | 类型 | 来源 |
|------|------|------|
| 项目 | string | project-svc |
| 时间段 | date range | — |
| 总查询数 | int | analytics-svc |
| 命中数 | int | analytics-svc |
| 命中率 | float | 计算 |
| 100% 匹配 | float | 计算 |
| 95-99% 匹配 | float | 计算 |
| 85-94% 匹配 | float | 计算 |
| < 85% 匹配 | float | 计算 |
| LLM 补全率 | float | 计算 |

**格式**：Excel + Dashboard  
**频度**：每周 / 每月

---

## 4. 运维报表

### 4.1 监控周报

| 字段 | 类型 | 来源 |
|------|------|------|
| 周 | date range | — |
| 可用性 | float | Prometheus |
| P95 响应 | float | Prometheus |
| 错误率 | float | Prometheus |
| 流量 | int | Prometheus |
| 告警数 | int | Alertmanager |
| 故障数 | int | incident-svc |
| MTTR | float | incident-svc |
| 容量 | float | Prometheus |
| 备份 | bool | backup-svc |
| 变更 | int | change-svc |

**格式**：Markdown + PDF  
**频度**：每周一 09:00  
**订阅**：SRE + PM + Sponsor

### 4.2 可用性月报

| 字段 | 类型 | 来源 |
|------|------|------|
| 月 | date | — |
| SLA 目标 | 99.9% | — |
| 实际可用性 | float | Prometheus |
| 计划停机 | float | change-svc |
| 非计划停机 | float | incident-svc |
| 故障次数 | int | incident-svc |
| P0 故障 | int | incident-svc |
| P1 故障 | int | incident-svc |
| MTTR | float | incident-svc |
| MTBF | float | incident-svc |
| 容量趋势 | chart | Prometheus |
| 成本 | ¥ | fin-svc |

**格式**：PDF + Dashboard  
**频度**：每月 1 号 09:00  
**订阅**：Sponsor + PM + SRE + 客户

### 4.3 SLA 报告（客户版）

| 字段 | 类型 | 来源 |
|------|------|------|
| 期间 | date range | — |
| 服务等级 | 99.9% | 合同 |
| 实际等级 | float | Prometheus |
| 服务积分 | int | 计算 |
| 补偿 | ¥ | 计算 |
| 故障明细 | list | incident-svc |
| 改进措施 | text | SRE |

**格式**：PDF（签字版）  
**频度**：季度 / 年度  
**订阅**：客户 + Sponsor

---

## 5. QA 报表

### 5.1 缺陷报告

| 字段 | 类型 | 来源 |
|------|------|------|
| ID | string | issue-svc |
| 标题 | string | issue-svc |
| 严重度 | enum | issue-svc |
| 优先级 | enum | issue-svc |
| 模块 | string | issue-svc |
| 状态 | enum | issue-svc |
| 创建人 | string | user-svc |
| 责任人 | string | user-svc |
| 创建日 | date | issue-svc |
| 关闭日 | date | issue-svc |
| 修复时长 | float | 计算 |
| 回归 | bool | issue-svc |
| 备注 | text | — |

**格式**：Excel + Dashboard  
**频度**：实时 + 每周汇总

### 5.2 测试覆盖率

| 字段 | 类型 | 来源 |
|------|------|------|
| 模块 | string | CI |
| 行覆盖率 | float | tarpaulin / coverage.py |
| 分支覆盖率 | float | tarpaulin / coverage.py |
| 函数覆盖率 | float | tarpaulin / coverage.py |
| UT 数量 | int | CI |
| 失败数 | int | CI |
| 跳过数 | int | CI |
| 趋势 | chart | CI |

**格式**：HTML + Dashboard  
**频度**：每日 + 每次 PR

### 5.3 测试进度

| 字段 | 类型 | 来源 |
|------|------|------|
| 测试阶段 | enum | QA |
| 计划用例 | int | QA |
| 执行用例 | int | QA |
| 通过 | int | QA |
| 失败 | int | QA |
| 阻塞 | int | QA |
| 跳过 | int | QA |
| 通过率 | float | 计算 |
| 缺陷数 | int | issue-svc |
| 完成度 | float | 计算 |
| 趋势 | chart | QA |

**格式**：Excel + Dashboard  
**频度**：每日 + 每周

---

## 6. 行业标准导出

### 6.1 XLIFF（翻译交换）

| 字段 | 说明 |
|------|------|
| 标准 | XLIFF 1.2 / 2.1 |
| 命名空间 | xml:lang |
| 元素 | `<file>` `<unit>` `<segment>` `<source>` `<target>` |
| 元数据 | `<metadata>` + 命名空间 |
| 状态 | translated / reviewed / final |

**生成器**：`xliff-svc`  
**支持版本**：1.2 + 2.1  
**导入兼容**：Trados / MemoQ / Wordfast

### 6.2 TMX（翻译记忆交换）

| 字段 | 说明 |
|------|------|
| 标准 | TMX 1.4 |
| 元素 | `<tu>` `<tuv>` `<seg>` |
| 语言 | xml:lang |
| 元数据 | prop / note |

### 6.3 TBX（术语库交换）

| 字段 | 说明 |
|------|------|
| 标准 | TBX 2.3 / 3.0 |
| 元素 | `<termEntry>` `<langSet>` `<tig>` `<term>` |
| 字段 | partOfSpeech / definition / context |

### 6.4 SRX（分段规则交换）

| 字段 | 说明 |
|------|------|
| 标准 | SRX 2.0 |
| 元素 | `<languagemap>` `<rule>` `<break>` `<beforebreak>` `<afterbreak>` |
| 用途 | 自定义分段规则 |

---

## 7. 报表生成架构

### 7.1 同步生成

```
请求 → bff → report-svc → analytics-svc (聚合查询) → 模板引擎 → 输出
```

### 7.2 异步生成

```
请求 → bff → report-svc → analytics-svc (数据采集) → Kafka → worker-svc (生成) → MinIO (存储) → 通知
```

### 7.3 报表模板引擎

- 选型：Apache Velocity / Thymeleaf / React PDF
- 模板版本化：Git 管理
- 模板多语言：i18n

### 7.4 报表存储

| 报表 | 存储 | 留存 |
|------|------|------|
| 周报/月报 | MinIO | 7 年 |
| 实时报表 | 内存 + Redis | 24h |
| 客户报告 | MinIO + 签名 | 永久 |
| 临时报表 | MinIO | 30 天 |

---

## 8. 权限与订阅

### 8.1 权限

| 报表 | 角色 |
|------|------|
| 项目报告 | 项目 PM / Sponsor / 客户 |
| 翻译进度日报 | PM / 团队 lead |
| 监控周报 | SRE / PM / Sponsor |
| 可用性月报 | Sponsor / PM / SRE / 客户 |
| SLA 报告 | Sponsor / 客户 |
| 缺陷报告 | QA / PM / 开发者 |
| 覆盖率 | QA / 开发者 |
| 测试进度 | QA / PM / Sponsor |

### 8.2 订阅

- 邮件订阅
- 站内通知
- Webhook（企业 IM）
- 文件下载

---

## 9. 与 150 任务 / 文档关联

| 任务 / 文档 | 关联 |
|-------------|------|
| 27 帳票設計 | 本文 |
| F1-F11 需求 | 数据来源 |
| 接口设计书 §3.9 worker | 异步生成 |
| 数据库设计书 analytics | 数据源 |
| 模块设计书 report-svc | 实现 |
| 138 成果物管理 | 交付物基线 |
| 90 UAT 计划书 | 验收 |

---

## 10. 关联文档

| 文档 | 路径 |
|------|------|
| CATs_需求规格说明书 v2.0 F1-F11 | `01-需求\需求规格说明\` |
| CATs_微服务架构设计书 v1.0 | `02-基础设计\架构设计\` |
| CATs_接口设计书 v2.0 §3.9 | `03-详细设计\接口设计\` |
| CATs_模块设计书 v2.0 | `03-详细设计\模块设计\` |
| CATs_测试设计书 v1.0 | `04-测试\测试设计书\` |
| CATs_UAT 计划书 v1.0 | `04-测试\UAT\` |

---

## 11. 待办 / Open Items

| # | 项 | 责任 | 期望关闭 |
|---|----|------|----------|
| OI-1 | 报表模板库建立 | BA + 前端 | M1-S1 |
| OI-2 | 客户报表需求确认 | BA + 客户 | M1-S2 |
| OI-3 | 行业标准导入兼容测试 | QA | M2-ST |
| OI-4 | SLA 计算公式 | SRE + 商务 | M1-S0 |
| OI-5 | 报表性能基准 | 架构师 | M1-S2 |

---

**文档结束**
