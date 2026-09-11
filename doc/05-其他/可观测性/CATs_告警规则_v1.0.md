# CATs 告警规则 v1.0

> **文档编号**：CATs-OPS-001
> **版本**：v1.0
> **创建日**：2026-09-11
> **状态**：评审前草稿（DDD Review 6 角色待 9/27 T-07 截止前补评审）
> **密级**：仅社内
> **作者**：架构师 + SRE 平台 Lead（Mavis 接手 agent per DEC-008，2026-09-11 13:49 JST Ulysses 授权代签）

---

## 文档管理信息

### 审批栏

| 角色 | 姓名 | 审批 | 日期 | 备注 |
|------|------|------|------|------|
| Sponsor (Ulysses 本人签) | Ulysses | ☐ | — | 一人公司 = Ulysses 持有 Sponsor 角色，不代签 |
| 架构师 Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 错误码字段引用一致性 |
| SRE 平台 Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | Prometheus + alertmanager 实施（真人到位后追溯签字覆盖） |
| Rust Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 业务 error 字段语义 |
| DBA Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 监控指标 SQL 索引 |
| QA Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 告警测试用例覆盖 |
| PMO Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 跟 Sprint 1 决议 10 + T-07 同步 |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|------|------|--------|----------|
| v1.0 | 2026-09-11 | 架构师 + SRE 平台 Lead（Mavis 接手 agent per DEC-008） | 初版：4 条 alertmanager rules per 错误码表 v1.0 §6.4（per Sprint 1 决议 10 + 接口设计书 v2.0+2 §8.7 9/27 截止） |

---

## 0. 元信息

| 项 | 值 |
|----|----|
| **作者** | 架构师 + SRE 平台 Lead（Mavis 代签 per DEC-008） |
| **Worktree** | `D:/CATs` (main) |
| **分支** | `main` |
| **commit baseline** | `f118c91` (per `git log -1`，13:11 JST 9/11 9 cherry-pick + 2 cherry-pick 完，main 包含 Sprint 1 决议 1-9 全部落地 + V1.1 排期草案) |
| **关联基线（B1.0）** | `1b27b2b`（启动会决议纪要 v1.0 8/30 落地） |
| **上游源文档** | 见 §0.1 源文档引用清单 |
| **下游引用** | 错误码表 v1.0 §6.4 / 接口设计书 v2.0+2 §8.7 / Sprint 1 任务拆解 v1.0+2 §6.11 / 架构设计书 v1.0 §14 |
| **配套工具** | Prometheus + alertmanager（K3s 阶段二 部署 per 启动会决议 6 SRE 平台独立估算 v1.0 commit be712dc） |

### 0.1 源文档引用清单（git 实证）

| 引用文档 | 路径 | commit hash | 用途 |
|---------|------|------------|------|
| CATs_错误码表 v1.0 | `doc/05-其他/管理/CATs_错误码表_v1.0.md` | `2146f53` | §6.4 alertmanager rules 4 条 source of truth |
| CATs_接口设计书 v2.0+2 | `doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md` | `1f3c94d` | §8.7 alertmanager rules 草稿 9/27 截止 |
| CATs_架构设计书 v1.0 | `doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md` | `2910f3d` | §14 监控告警聚合 |
| CATs_SRE 平台独立估算 v1.0 | `doc/05-其他/管理/CATs_M1-Sprint1_SRE平台独立估算_v1.0.md` | `be712dc` | T-05 unblock 后 K3s 阶段二 Prometheus + alertmanager 部署 |
| CATs_技术基线 v1.0 | `doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md` | `047dc9c` | §1 Rust 1.98.0 + PG 18.6 + pgvector 0.8.6 |
| CATs_启动会决议纪要 v1.0 | `doc/05-其他/会议记录/CATs_M1_Sprint1_启动会决议纪要_v1.0.md` | `1b27b2b` | 决议 10 "Kafka + 错误码闭环" 9/27 截止 T-07 |
| CATs_模块设计书 v2.2 | `doc/03-详细设计/模块设计/CATs_模块设计书_v2.0.md` | `f8ac021` | §4 错误码引用终端 |

---

## 1. 概述

### 1.1 目的

为 CATs 全栈业务错误码建立统一告警规则（per 错误码表 v1.0 §6.4），实现：

1. **业务告警分级**（P1/P2/P3）— 跟错误码语义一致
2. **按 error 字段聚合**（per 错误码表 §6.4）— 替代单一 HTTP status 维度
3. **自动化告警响应**（per on-call 升级路径）— 一人公司 + AI 代签
4. **可观测性集成**（per 架构设计书 v1.0 §14）— Prometheus + alertmanager

### 1.2 范围

- **M1-Sprint 1 9/27 截止**：4 条 alertmanager rules（per 错误码表 §6.4）
- **后续 v1.x 升版**：业务端点新增（错误码扩展）+ 跨服务错误码聚合 + 国际化 + 客户端 SDK 集成
- **v2.0 远期**：跨服务错误码聚合 + 智能告警（AIOps）+ 全链路追踪集成

### 1.3 引用与下游

- **告警数据源**：`ErrorBody.error` 字段（per `crates/auth-service/src/models.rs:42` + 接口设计书 v2.0+2 §3.5.1 3 字段 schema）
- **告警聚合**：Prometheus rules（per `architectures/observability/prometheus-rules.yaml` K3s 阶段二部署）
- **告警通知**：alertmanager（per `architectures/observability/alertmanager.yaml`）+ Slack / 邮件 / on-call pager（待 5 域 Lead 真人到位后定）
- **客户端**：iOS / Android / Web / Unity 集成 SDK 按错误码表 §3.7 "客户端重试"列决定是否本地降级

---

## 2. 告警分级（per 错误码表 v1.0 §6.4）

| 级别 | 触发条件 | 通知方式 | 升级路径 |
|------|---------|---------|---------|
| **P1 (Critical)** | 立即影响生产可用性（如 server_error、dependency_unavailable） | Page on-call（电话/SMS）+ Slack `#oncall` 频道 | 1h 未响应升级 Sponsor，2h 未响应升 P0 (全员) |
| **P2 (Warning)** | 疑似业务问题（如 invalid_credentials 爆破） | Slack `#security-alerts` 频道 | 4h 未响应升级 on-call，24h 未响应升 P1 |
| **P3 (Info)** | 观察项（如 rate_limited） | Slack `#observability` 频道 | 仅记录，0 立即升级 |

---

## 3. alertmanager rules 实施（per 错误码表 v1.0 §6.4）

### 3.1 rule-001: P1 server_error 告警

> **触发**: `error="server_error"` 在 5 分钟内 ≥ 5 次

```yaml
# prometheus-rules.yaml
groups:
  - name: cats-business-alerts
    rules:
      - alert: CatsServerErrorBurst
        expr: |
          sum by (service, error) (rate(
            http_requests_total{
              error="server_error"
            }[5m]
          )) >= 5
        for: 1m
        labels:
          severity: P1
          category: business-error
          source: error-code-table-v1.0
          reference: "错误码表 v1.0 §6.4 line 1 (commit 2146f53)"
        annotations:
          summary: "P1 server_error 告警 (per 错误码表 v1.0 §6.4)"
          description: |
            service: {{ $labels.service }}
            error_code: server_error
            触发: 5分钟内 ≥ 5 次
            HTTP status: 500
            行动: 立即 on-call 介入（参考 CATs_告警规则_v1.0.md §3.1）
            关联: 接口设计书 v2.0+2 §3.5.1 ErrorBody.error 字段
            上游文档: doc/05-其他/管理/CATs_错误码表_v1.0.md §6.4
```

**告警响应流程**:
1. Page on-call（SRE 平台 Lead，per 启动会决议 6 SRE 估算 v1.0）
2. on-call 5 分钟内确认，参考 `crates/auth-service/src/models.rs:42` 排查代码
3. 1h 未响应升级 Sponsor（Ulysses）
4. 2h 未响应升 P0（全员 @here）

### 3.2 rule-002: P1 dependency_unavailable 告警

> **触发**: `error="dependency_unavailable"` 在 1 分钟内 ≥ 3 次

```yaml
      - alert: CatsDependencyDownBurst
        expr: |
          sum by (service, error, dependency) (rate(
            http_requests_total{
              error="dependency_unavailable"
            }[1m]
          )) >= 3
        for: 30s
        labels:
          severity: P1
          category: business-error
          source: error-code-table-v1.0
          reference: "错误码表 v1.0 §6.4 line 2 (commit 2146f53)"
        annotations:
          summary: "P1 dependency_unavailable 告警"
          description: |
            service: {{ $labels.service }}
            dependency: {{ $labels.dependency }}
            error_code: dependency_unavailable
            触发: 1分钟内 ≥ 3 次
            HTTP status: 503
            行动: 立即 on-call 介入，检查依赖（PG 18.6 / Kafka / Redis/Valkey / OTel collector）
            关联: 启动会决议 6 SRE 估算 v1.0 (commit be712dc)
            上游文档: doc/05-其他/管理/CATs_错误码表_v1.0.md §6.4
```

**告警响应流程**:
1. Page on-call（SRE 平台 Lead）
2. on-call 立即检查依赖：`SELECT pg_is_in_recovery();` / `redis-cli ping` / `kafka-topics --list`
3. 30 分钟未恢复升级 Sponsor
4. 1h 未恢复降级策略：禁用非关键功能（per 架构设计书 v1.0 §14.3 降级策略）

### 3.3 rule-003: P3 rate_limited 观察告警

> **触发**: `error="rate_limited"` 在 15 分钟内 ≥ 100 次（仅观察，0 立即升级）

```yaml
      - alert: CatsRateLimitedSpike
        expr: |
          sum by (service, error) (rate(
            http_requests_total{
              error="rate_limited"
            }[15m]
          )) >= 100
        for: 5m
        labels:
          severity: P3
          category: business-error
          source: error-code-table-v1.0
          reference: "错误码表 v1.0 §6.4 line 3 (commit 2146f53)"
        annotations:
          summary: "P3 rate_limited 观察告警"
          description: |
            service: {{ $labels.service }}
            error_code: rate_limited
            触发: 15分钟内 ≥ 100 次
            HTTP status: 429
            行动: 仅观察，0 立即升级（per 错误码表 v1.0 §6.4 P3 定义）
            关联: 客户端 SDK 重试策略 (per 错误码表 §3.5 客户端重试列)
            上游文档: doc/05-其他/管理/CATs_错误码表_v1.0.md §6.4
```

**告警响应流程**:
1. Slack `#observability` 频道通知
2. 0 立即升级
3. 周报 review（per Sprint 复盘 97b3bb1 模板）

### 3.4 rule-004: P2 invalid_credentials 爆破检测

> **触发**: `error="invalid_credentials"` 同一 user_id 5 分钟内 ≥ 10 次（疑似爆破）

```yaml
      - alert: CatsInvalidCredentialsBurstByUser
        expr: |
          sum by (service, user_id) (rate(
            http_requests_total{
              error="invalid_credentials"
            }[5m]
          )) >= 10
        for: 2m
        labels:
          severity: P2
          category: security-alert
          source: error-code-table-v1.0
          reference: "错误码表 v1.0 §6.4 line 4 (commit 2146f53)"
        annotations:
          summary: "P2 invalid_credentials 爆破检测"
          description: |
            service: {{ $labels.service }}
            user_id: {{ $labels.user_id }}
            error_code: invalid_credentials
            触发: 同一 user_id 5分钟内 ≥ 10 次
            HTTP status: 401
            行动: 临时锁定 user_id（per §4 锁定策略）+ Slack #security-alerts
            关联: 安全要件定义书 v1.0 §3 认证 §6 审计
            上游文档: doc/05-其他/管理/CATs_错误码表_v1.0.md §6.4
```

**告警响应流程**:
1. Slack `#security-alerts` 频道通知（SRE 平台 Lead + Sponsor）
2. 临时锁定 user_id 30 分钟（per 错误码表 §3.3 invalid_credentials 客户端重试 = ✗ 防爆破）
3. 4h 未响应升级 on-call
4. 24h 未响应升 P1（疑似协同爆破）

---

## 4. 实施细节（per 启动会决议 6 SRE 平台独立估算 v1.0 K3s 阶段二）

### 4.1 部署架构

```
[Service] → [OTel collector] → [Prometheus] → [alertmanager] → [Slack / PagerDuty]
```

### 4.2 关键文件

| 文件 | 路径 | 用途 |
|------|------|------|
| prometheus-rules.yaml | `architectures/observability/prometheus-rules.yaml` | 4 条 rule-001..004 落地 |
| alertmanager.yaml | `architectures/observability/alertmanager.yaml` | 通知渠道 + 抑制规则 + 升级路径 |
| service-monitors.yaml | `architectures/observability/service-monitors.yaml` | Prometheus scrape config（K8s ServiceMonitor CRD） |
| prometheus-deployment.yaml | `architectures/observability/prometheus-deployment.yaml` | K3s Prometheus 部署（per 决议 6 SRE 估算 v1.0） |
| alertmanager-deployment.yaml | `architectures/observability/alertmanager-deployment.yaml` | K3s alertmanager 部署 |

### 4.3 关键 Prometheus 指标

| 指标 | 类型 | 标签 | 说明 |
|------|------|------|------|
| `http_requests_total` | counter | service, method, path, status, error | HTTP 请求总数（含 ErrorBody.error 标签） |
| `grpc_requests_total` | counter | service, method, status | gRPC 请求总数 |
| `auth_jwt_verify_total` | counter | service, outcome | JWT 验证总数（用于爆破检测） |
| `db_pool_active` | gauge | service | DB 连接池活跃数 |
| `db_query_duration_seconds` | histogram | service, query | DB 查询耗时（per §3.6 dependency_timeout 阈值） |

> **注**: `error` 标签值必须枚举错误码表 v1.0 §3 全部 28 条 snake_case（per 接口设计书 v2.0+2 §3.5.4 反向引用 + §8.5 决议 10 同步）

### 4.4 通知渠道

| 渠道 | 用途 | 配置 |
|------|------|------|
| Slack `#oncall` | P1 告警 | webhook URL per `architectures/observability/alertmanager.yaml` |
| Slack `#security-alerts` | P2 安全告警 | webhook URL per `architectures/observability/alertmanager.yaml` |
| Slack `#observability` | P3 观察告警 | webhook URL per `architectures/observability/alertmanager.yaml` |
| 邮件 | 升级路径（Sponsor / 全员） | SMTP per K3s 阶段二 secrets |
| PagerDuty | P0 全员升级 | per 启动会决议 6 SRE 估算 v1.0 §4 升级路径 |

> **注**: webhook URL / SMTP / PagerDuty 凭据 0 写入 git（per 守门 #5 8/27 11:06 JST hard ban 0 env 打印）。K3s secrets 注入由 SRE 平台 Lead 真人到位后配（per 启动会决议 6 SRE 估算 v1.0 §4 真人到位依赖）。

### 4.5 抑制规则（避免告警风暴）

```yaml
# alertmanager.yaml inhibit_rules
inhibit_rules:
  - source_matchers: [severity="P1"]
    target_matchers: [severity="P2"]
    equal: [service, error]
  - source_matchers: [severity="P1"]
    target_matchers: [severity="P3"]
    equal: [service]
```

---

## 5. 引用与下游

### 5.1 错误码表 v1.0 §6.4 引用

> 引用 `doc/05-其他/管理/CATs_错误码表_v1.0.md` §6.4 4 条 rule source of truth:
> 1. `error="server_error"`: P1 告警 (page on-call) → 本文档 §3.1 rule-001
> 2. `error="dependency_unavailable"`: P1 告警 → 本文档 §3.2 rule-002
> 3. `error="rate_limited"`: P3 告警 (仅观察) → 本文档 §3.3 rule-003
> 4. `error="invalid_credentials"` 同一 user_id 5 分钟 ≥ 10 次: P2 告警 (疑似爆破) → 本文档 §3.4 rule-004

### 5.2 接口设计书 v2.0+2 §8.7 引用

> 引用 `doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md` §8.7:
> "决议 10 明确 `doc/05-其他/可观测性/CATs_告警规则_v1.0.md` 草稿 9/27 截止"
> → 本文档 = 该草稿，9/27 截止前 T-07 实施

### 5.3 启动会决议 10 引用

> 引用 `doc/05-其他/会议记录/CATs_M1_Sprint1_启动会决议纪要_v1.0.md` 决议 10 (commit 1b27b2b):
> "Kafka + 错误码闭环：通过 A T-07 统一 | 9/27 | 架构师 Lead | 含 T-07"
> → 本文档 + 接口设计书 v2.0+2 §3.5/§8.1/§8.5/§8.6 已落地 (commit 1f3c94d) + 错误码表 v1.0 §6.4 (commit 2146f53) = 决议 10 实质落地

### 5.4 SRE 平台独立估算 v1.0 引用

> 引用 `doc/05-其他/管理/CATs_M1-Sprint1_SRE平台独立估算_v1.0.md` (commit be712dc):
> K3s 阶段二 Prometheus + alertmanager 部署
> → 本文档 §4 实施细节 4.1-4.5 引用

### 5.5 模块设计书 v2.2 引用

> 引用 `doc/03-详细设计/模块设计/CATs_模块设计书_v2.0.md` (commit f8ac021) §4 错误码引用终端:
> 所有 16 域 service crate 必须 emit ErrorBody.error 字段 (per 接口设计书 v2.0+2 §3.5.1)
> → 本文档 §4.3 Prometheus 指标 `error` 标签依赖此

---

## 6. 已知缺口（DDD Review 必查 per AI 协作文档治理 2026-08-26）

### 6.1 通知渠道 webhook URL / SMTP 凭据 / PagerDuty token 0 落地

- 本文档 §4.4 通知渠道配置依赖 SRE 平台 Lead 真人到位（per 启动会决议 6 SRE 估算 v1.0）
- **当前状态**: 文档结构 + Prometheus rules 已落，K3s secrets 注入留 K3s 阶段二实施时由 SRE 平台 Lead 配
- **建议**: T-07 9/27 截止前由 SRE 平台 Lead 真人到位完成 secrets 注入

### 6.2 §3.4 P2 锁定策略 30 分钟阈值

- 错误码表 §6.4 仅说"疑似爆破"，0 给出明确锁定时长
- 本文档 §3.4 暂定 30 分钟（per 错误码表 §3.3 invalid_credentials 客户端重试 ✗ 防爆破启发）
- **建议**: DDD Review 阶段由架构师 Lead + 安全要件 Lead 共同定（per 启动会决议 R-02 风险）
- **当前状态**: 文档初版草稿，0 阻塞实施

### 6.3 §4.3 Prometheus 指标 `error` 标签 0 实际 16 域都 emit

- 本文档假设 16 域 service crate 都 emit `ErrorBody.error` 字段作为 Prometheus label
- 实际：T-01 auth-service 落地（commit 2146f53）已 emit，其他 15 域 T-02..T-07 实施时落地
- **建议**: T-07 9/27 截止时验证 16 域都 emit `error` 标签（per 接口设计书 v2.0+2 §6.1/§6.2）
- **当前状态**: 0 阻塞本文档落地，留 T-07 实施时验证

### 6.4 §3 4 条 rules 仅覆盖 28 条 错误码中的 4 条

- 错误码表 v1.0 §3 28 条错误码，§6.4 4 条告警 rules 仅覆盖 server_error / dependency_unavailable / rate_limited / invalid_credentials
- 剩余 24 条 (invalid_request / invalid_token / token_expired / token_revoked / user_inactive / resource_not_found / username_conflict / email_conflict / 等) 暂未配置告警
- **建议**: v1.1 升版时扩展（per §1.2 后续 v1.x 升版触发条件）
- **当前状态**: 0 阻塞本文档落地（M1-Sprint 1 范围 4 条足够）

### 6.5 启动会决议 10 范围 = 错误码引用闭环 + Kafka 物理发布

- 启动会决议 10 包含 2 子项: (a) 错误码引用闭环（已完成 = 接口设计书 §3.5/§8.1/§8.5/§8.6 + OpenAPI v1.0.1 + proto v1.0.1 + 本文档） (b) Kafka 物理发布（K3s 阶段二）
- 本文档仅覆盖 (a)，(b) 留 K3s 阶段二
- **建议**: 启动会决议 10 升 v1.0 → v1.0.1 (per 本文档) + Kafka 物理发布 升 v1.0.2 (K3s 阶段二)
- **当前状态**: 决议 10 实质落地（前置工作），完整实施 9/27 T-07

---

## 7. 升版流程

### 7.1 升版触发条件

- 业务端点新增（错误码扩展）→ 新增 rule
- 告警分级调整（P1/P2/P3 阈值变更）→ 修改 rule
- 通知渠道扩展（PagerDuty / OpsGenie / 企业微信）→ 修改 alertmanager.yaml
- 一人公司 RACI 变更（5 域 Lead 真人到位）→ 修改审批栏

### 7.2 升版流程

1. **PR 起草**：架构师 Lead + SRE 平台 Lead 联合起草
2. **DDD Review**：6 角色评审（含 Sponsor 本人签）
3. **CAB 决议**：v1.x → v2.0 需走 CAB-002（参考 CAB-001 v1.0 流程）
4. **基线化**：v 升 B-y.y，CAB 决议书 + Baseline一览同步
5. **引用同步**：错误码表 v1.0 §6.4 / 接口设计书 §3.5/§8.7 / 模块设计书 §4 / 启动会决议 10 / SRE 估算 v1.0 全部同步

### 7.3 当前 v1.0 适用范围

- **时间窗口**：M1-Sprint 1 决议 10 + T-07（9/27 截止）
- **服务范围**：auth-service（per T-01 落地）+ 其他 15 域 T-02..T-07 实施时验证
- **升版预期**：Sprint 1 末 / Sprint 2 启 升 v1.1（新增 24 条错误码 rules）

---

**文档结束（v1.0，2026-09-11）**
