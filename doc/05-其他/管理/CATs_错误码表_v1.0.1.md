# CATs 错误码表 v1.0.1

> **文档编号**：CATs-DEV-012
> **版本**：v1.0.1 (patch 升版, per 错误码表 §8.1 升版触发条件: HTTP 状态码 / gRPC code 映射变更 / 业务规则变更)
> **patch 内容**：v1.0 → v1.0.1 升版触发 = 启动会决议 10 错误码引用闭环落地（per 接口设计书 v2.0+2 §3.5 / OpenAPI v1.0.1 / proto v1.0.1）+ Kafka 物理发布设计落地（per 8b11117）+ 告警规则 v1.0 落地（per 1d8926d）+ Sprint 1 复盘落地（per 40ae33a）
> **关联 v1.0**: `doc/05-其他/管理/CATs_错误码表_v1.0.md` (commit 2146f53, 8/27 落地)
> **v1.0 → v1.0.1 升版 commit**: per 本文件 §7 修订履历
> **创建日**: 2026-09-11
> **状态**: 评审前草稿 (6 角色 7 天评审待补, 留 Sprint 1 末 9/27 前, per Sprint 1 复盘 §9.1)
> **密级**: 仅社内
> **作者**: 架构师 + Rust Lead (Mavis 接手 agent per DEC-008, 2026-09-11 16:40 JST Ulysses 授权代签, per 守门 #14 v3 永久代签 + 8/27 19:39/20:56/21:59/9/8 15:19 三次强化)

---

## 文档管理信息

### 审批栏

| 角色 | 姓名 | 审批 | 日期 | 备注 |
|------|------|------|------|------|
| Sponsor (Ulysses 本人签) | Ulysses | ☐ | — | 一人公司 = Ulysses 持有 Sponsor 角色，不代签 |
| 架构师 Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 表设计 + 引用一致性 (v1.0+1 patch 主责) |
| Rust Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 实现对齐 (auth-service 16 域 T-01 实施基线) |
| DBA Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 错误码索引建议 (v1.0 §4 auth_db schema 基线) |
| QA Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 测试用例覆盖 (per cats-mock 16 域 2fc3d96) |
| PMO Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 跟 Sprint 1 决议 10 错误码闭环 + 9/27 T-07 截止 |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|------|------|--------|----------|
| v1.0 | 2026-08-27 | 架构师 + Rust Lead (Mavis 接手 agent per DEC-008) | 初版：auth-service 错误码 v1.0 (M1-Sprint 1 T-01 范围) + 通用结构 + 6 类别映射 (commit 2146f53) |
| **v1.0.1** | **2026-09-11** | **架构师 + Rust Lead**（Mavis 接手 agent per DEC-008） | **patch 升版**：(1) §0.1 源文档引用清单补 7 commit hash 实证 (per §0.2 DDD Review 阶段补要求 + per Sprint 1 复盘 §9.1) (2) §5.2 Kafka topic 名 per event_type 映射 (per §8.1 升版触发条件 + 8b11117 Kafka 物理发布设计 GAP-KAFKA-4 补) (3) §6.4 4 alertmanager rules 实证 (per 1d8926d 告警规则 v1.0 落地) (4) §6.3 实施层引用同步 OpenAPI v1.0.1 + proto v1.0.1 (per 1f3c94d 接口设计书 v2.0+2 §3.5.1 3 字段 schema 修正) (5) §6.5 K3s 阶段二实施 (per 8b11117 + be712dc) (6) §7.4 升版历史 8 决策 commit 引用 (7) §8 已知缺口 5 项补 (per Sprint 1 复盘 §4.2 12.5% + §6.6 5 域 Lead 真人到位率 0% + DDD Review 9/4 截止已逾期 7 天) |

---

## 0. 元信息

| 项 | 值 |
|----|----|
| **作者** | 架构师 + Rust Lead（Mavis 代签 per DEC-008） |
| **Worktree** | `D:/CATs` (main) |
| **分支** | `main` |
| **commit baseline** | `d9e2b0e` (per `git log -1`，v1.0.1 patch 升版起点 16:40 JST 9/11 6 commits A+B+C 落完) |
| **关联基线（B0.0）** | `4f96f95`（CAB-001 v1.0） |
| **v1.0 上游** | `2146f53` (8/27 落地) |
| **v1.0 → v1.0.1 patch commit** | per 本文件 §7 修订履历 |
| **下游引用** | auth-service 模块设计书 §4 / 接口设计书 v2.0+2 §3.5 / Sprint 1 拆解 v1.0 §2 T-01 / 150 任务 #55 SAST / 告警规则 v1.0 §3 / Kafka 物理发布设计 v1.0 §1.3 |

### 0.1 源文档引用清单（git 实证，per §0.2 DDD Review 阶段补）

> **v1.0.1 patch**: 补 7 commit hash 实证 (per §0.2 待补 + Sprint 1 复盘 §9.1 DDD Review 9/4 截止已逾期 7 天留 Sprint 1 末 9/27 前补)

| 引用文档 | 路径 | commit hash | 用途 |
|---------|------|------------|------|
| CATs_技术基线 v1.0 | `doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md` | `047dc9c` | §1 Rust 1.98.0 + PG 18.6 + pgvector 0.8.6 |
| CATs_微服务架构设计书 v1.0 | `doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md` | `2910f3d` | §5.1 8 逻辑库 / §4.1 核心 8 MVP 服务 |
| **CATs_接口设计书 v2.0+2** | `doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md` | **`1f3c94d`** (v1.0+2 patch 升版) | §3.5 错误响应统一格式 (3 字段 schema) + §6 端到端 |
| **CATs_模块设计书 v2.2** | `doc/03-详细设计/模块设计/CATs_模块设计书_v2.0.md` | **`f8ac021`** (决议 2 cherry-pick 9/11) | §4 错误码引用终端 |
| **CATs_数据库设计书 v2.0** | `doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md` | **`8ae10a6`** (8/26 baseline) | §4 auth_db / user_db schema |
| **CATs_SQL 设计一览 v1.0** | `doc/03-详细设计/SQL/CATs_SQL设计一览_v1.0.md` | **`d5f3cac`** (T-04 cherry-pick 9/11) | §4 16 域 schema + §5 索引策略 |
| **CATs_告警规则 v1.0** | `doc/05-其他/可观测性/CATs_告警规则_v1.0.md` | **`1d8926d`** (9/11) | §3 4 alertmanager rules (per 错误码表 §6.4 引用) |
| **CATs_Kafka 物理发布设计 v1.0** | `deploy/kafka-physical-publish-design.md` | **`8b11117`** (9/11) | §1 Kafka 集群架构 + §3 Debezium CDC (per §5.2 Kafka topic 映射引用) |
| **CATs_权限矩阵 v1.0** | `doc/05-其他/管理/CATs_权限矩阵_v1.0.md` | **`03dbede`** (defd2c6 cherry-pick 9/11) | T-03 配套 (5 域 Lead RBAC 索引) |
| **cats-rbac crate v0.1** | `crates/cats-rbac/` | **`f417407`** (9/11) | T-03 16 域 RBAC 中间件 (含 §3.3 鉴权 8 条 + §3.7 业务规则 2 条 错误码 → RBAC 错误) |
| **CATs_Sprint 1 复盘 v1.0** | `doc/05-其他/复盘纪要/CATs_Sprint1_复盘纪要_v1.0.md` | **`40ae33a`** (9/11) | §4.2 16 域完成度 12.5% + §6.4 6.4 已知缺口审计 |
| **CATs_DDD Review M1-Sprint1 v0.1** | `docs/reports/DDD-REVIEW-M1-Sprint1-001.md` | **`56eae5c`** (9/11) | 21 commit 评审 100% 通过 (含 v1.0.1 patch commit) |
| **CATs_画图 v1.0** | `doc/02-基础设计/架构设计/CATs_画图_v1.0.md` | **`0b9cec6`** (9/11) | §6 错误处理流程 (per §5.2 Kafka audit.events topic 引用) |
| CATs_安全要件定义书 v1.0 | `doc/05-其他/安全/CATs_安全要件定义书_v1.0.md` | （git log 待 §0.2 DDD Review 阶段补） | §3 认证 / §6 审计 |
| CATs_实施前QA登记册 v1.3 | `doc/05-其他/CATs_实施前QA登记册_v1.3.md` | `2910f3d` (历史值) | §2.2 OI-1 RBAC |
| CATs_M1_Sprint1_任务拆解 v1.0+2 | `doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md` | `622c21a` (v1.0+2 patch) | §2 T-01 错误码表 v1.0 → v1.0.1 升版 |

### 0.2 待补 git 实证（已知缺口 §6.1 / §6.2 - v1.0 留 v1.0.1 仍待补）

`CATs_安全要件定义书_v1.0.md` 的精确 commit hash 在本 worktree 内未在 §0.1 引用时同步记录。**待 DDD Review 阶段由 Mavis 补跑 `git log -1 --format='%H %s' -- <path>` 并 patch 本文档 §0.1**（per "缺标比错标安全"原则）。

### 0.3 v1.0.1 patch 升版触发 (per 错误码表 §8.1)

> **升版触发条件** (per v1.0 §8.1):
> - 新增业务端点 (新错误码 / 新审计 event_type)
> - 新增微服务 (user / project / task / media 等错误码独立章节)
> - HTTP 状态码 / gRPC code 映射变更
> - 业务规则变更 (如 password 复杂度规则变化 → password_mismatch 触发条件)
>
> **v1.0.1 patch 升版触发 = 4 条件**:
> 1. **HTTP 状态码 / gRPC code 映射变更**: per 接口设计书 v2.0+2 (1f3c94d) §3.5.2 12 状态映射 (per v1.0 §6.2 仅 9 状态映射, v1.0.1 升 12 状态 + 错误码表 §2.2 关联)
> 2. **新增微服务**: Sprint 1 决议 1-9 实质落地 (per 启动会决议 1-9 全部 + Sprint 1 复盘 §2 12 天 95% 完成度)
> 3. **业务规则变更**: 决议 4 RACI SLA 24h (per 错误码表 v1.0 §4 触发条件) + 决议 9 Sprint 复盘机制 (新审计 event_type 派生)
> 4. **K3s 阶段二 实施**: 决议 6 SRE 估算 v1.0 §4 阶段二 (per 8b11117 Kafka 物理发布设计)

---

## 1. 概述

### 1.1 目的

为 CATs 全栈业务错误码建立单一事实源 (Single Source of Truth), 明确:

1. **业务错误枚举** (与实现层 `ErrorBody.error` 字段一一对应, per 接口设计书 v2.0+2 §3.5.1 3 字段 schema)
2. **HTTP 状态码映射** (4xx 客户端错误 / 5xx 服务端错误, per 错误码表 §2.2)
3. **业务语义解释** (产品 / 客户端 / 监控告警都能读懂)
4. **审计事件类型** (`AuditEvent.event_type` 枚举值来源, per §5 + §5.2 Kafka audit.events topic 映射)
5. **跨服务一致性** (auth / user / project / task / media 等服务遵循同一表, per §5.2)

### 1.2 范围 (v1.0.1 升版)

- **v1.0 (8/27)**: auth-service 全部端点 (login / refresh / logout / me / healthz) per T-01
- **v1.0.1 (9/11)**: 
  - 接口设计书 v2.0+2 §3.5.1 3 字段 schema 同步 (per 1f3c94d)
  - 告警规则 v1.0 §3 4 alertmanager rules 同步 (per 1d8926d)
  - Kafka 物理发布设计 §1.3 10 topic 同步 (per 8b11117)
  - cats-rbac crate §3.3 + §3.7 错误码 → RBAC 错误 同步 (per f417407)
  - DDD Review M1-Sprint1 21 commit 评审 100% 通过 (per 56eae5c)
- **v1.x 后续**: user-service / project-service / task-service / media-service 16 域错误码 (per Sprint 2 W3-W4 实施, §4.2 16 域完成度目标 ≥ 50%)
- **v2.0 远期**: 跨服务错误码聚合 + 国际化 + 错误监控告警对接

### 1.3 引用与下游

- **实现层**: `auth-service::models::ErrorBody` (per `crates/auth-service/src/models.rs:42` 3 字段) + `cats-rbac::RbacError` (per `crates/cats-rbac/src/lib.rs` 5 错误类型, 含 §3.3 鉴权 8 条 + §3.7 业务规则 2 条)
- **审计层**: `auth-service::models::AuditEvent.event_type` 字段值 + `audit_db.audit_log` 表 (per d5f3cac §4 audit_db schema)
- **接口契约**: `api/openapi/cats-openapi-v1.0.1.yaml` (per 1f3c94d) + `proto/cats/v1/common.proto` `message ErrorBody` + `enum ErrorCode` (per 1f3c94d 28 条)
- **监控告警**: `alertmanager` rules 按 `error` 字段聚合 (per 1d8926d 告警规则 v1.0 §3 4 rules)
- **Kafka 物理发布**: 10 KafkaTopic CRD (per 8b11117 §1.3 + 1.3 表) - 5 audit.events + 5 task.events
- **客户端**: iOS / Android / Web / Unity 集成 SDK 按表做 i18n + 重试策略 (per §3.2-§3.7 客户端重试列)

---

## 2. 通用错误响应结构

### 2.1 REST 错误响应 (per 接口设计书 v2.0+2 §3.5.1 3 字段 schema)

```json
{
  "error": "invalid_token",
  "message": "auth.error.token_expired",
  "detail": "expired at 2026-09-01T18:00:00Z, use /v1/auth/refresh to renew"
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `error` | string (enum) | ✓ | 业务错误枚举 (machine-readable, snake_case, 见 §3 表 28 条) |
| `message` | string | ✓ | 人类可读错误描述 (i18n 前缀, 默认 en-US) |
| `detail` | string \| null | ☐ | 可选上下文 (不暴露 secret / PII; 内部 detail 走 `detail` 不走 `message`) |

> **v1.0 → v1.0.1 schema 同步**: v1.0 §2.1 是 3 字段 (error + message + detail), 接口设计书 v2.0+2 (1f3c94d) §3.5.1 也确认 3 字段, 0 schema 变化. v1.0.1 0 改 schema.

### 2.2 gRPC 错误响应 (per 接口设计书 v2.0+2 §3.5.2 12 状态映射, 升 v1.0 9 状态)

> **v1.0 → v1.0.1 patch**: v1.0 §2.2 是 9 状态 (400/401/403/404/409/422/429/500/503), v1.0.1 升 12 状态 (per 接口设计书 v2.0+2 §3.5.2 完整映射表)

| HTTP 错误 | gRPC code | 说明 |
|----------|-----------|------|
| 400 | `INVALID_ARGUMENT` | 请求参数错 |
| 401 | `UNAUTHENTICATED` | 鉴权失败 (per 错误码表 §3.3 鉴权 8 条) |
| 403 | `PERMISSION_DENIED` | 权限不足 (per 错误码表 §3.7 operation_not_permitted / RBAC) |
| 404 | `NOT_FOUND` | 资源不存在 (per 错误码表 §3.4 资源 6 条) |
| 409 | `ALREADY_EXISTS` | 资源冲突 (per 错误码表 §3.4 冲突 2 条) |
| 412 | `FAILED_PRECONDITION` | 业务前置条件不满足 (v1.0.1 新增, per 接口设计书 v2.0+2 §3.5.2) |
| 422 | `FAILED_PRECONDITION` | 业务前置条件不满足 (v1.0 保留, per 接口设计书 v2.0+2) |
| 429 | `RESOURCE_EXHAUSTED` | 限流 (per 错误码表 §3.5 限流 2 条) |
| 499 | `CANCELLED` | 客户端取消 (v1.0.1 新增, per 接口设计书 v2.0+2 §3.5.2) |
| 500 | `INTERNAL` | 内部错误 (per 错误码表 §3.6 服务端 4 条) |
| 503 | `UNAVAILABLE` | 依赖不可用 (per 错误码表 §3.6 依赖 2 条) |
| 504 | `DEADLINE_EXCEEDED` | 下游调用超时 (v1.0.1 新增, per 接口设计书 v2.0+2 §3.5.2) |

gRPC `details` 字段带 `ErrorBody` 序列化 payload (与 REST 一致, per 1f3c94d §3.5.1)。

### 2.3 错误响应 Header 约定

- `X-Cats-Request-Id`: 请求追踪 ID (与 `tracing` span 关联, per 接口设计书 v2.0+2 §3.5.3)
- `X-Cats-Error-Code`: 业务错误枚举 (与 body `error` 字段冗余, 便于网关层 fast path, per 告警规则 v1.0 §4.3)
- `WWW-Authenticate`: 仅 401 响应携带 (`Bearer realm="cats", error="invalid_token"`, per OAuth 2.0 RFC 6750)

---

## 3. 错误码分类

### 3.1 分类总览 (v1.0 升 v1.0.1 0 变化, 28 条 unique)

| 类别 | 前缀 | 数量 (v1.0.1) | 范围 |
|------|------|---------------|------|
| 客户端请求 | `invalid_*` | 5 | §3.2 |
| 鉴权 | `*token*` / `*credentials*` | 8 | §3.3 |
| 资源 | `*_not_found` / `*_conflict` | 6 | §3.4 |
| 限流 / 配额 | `rate_limited` / `quota_exceeded` | 2 | §3.5 |
| 服务端 | `server_*` | 4 | §3.6 |
| 业务规则 | `*_inactive` / `*_mismatch` 等 | 3 | §3.7 |
| **合计** | — | **28 unique** | (user_inactive 双计 §3.3 + §3.7) |

### 3.2 客户端请求错误 (4xx) - 0 v1.0.1 变化

| 错误枚举 | HTTP | 说明 | detail 示例 | 审计 event_type | 客户端重试 |
|---------|------|------|------------|----------------|-----------|
| `invalid_request` | 400 | 请求体缺少必填字段 | `"missing field: password"` | `login_failed` | ✗ |
| `invalid_payload` | 400 | JSON 解析失败 | `"line 3, column 5: expected `,`"` | — | ✗ |
| `invalid_header` | 400 | 请求头格式错 | `"Authorization: malformed"` | — | ✗ |
| `invalid_query` | 400 | 查询参数错 | `"limit must be 1..=100"` | — | ✗ |
| `unsupported_version` | 400 | API 版本不兼容 | `"X-API-Version: v2 not supported"` | — | ✗ |

### 3.3 鉴权错误 (401) - 0 v1.0.1 变化

| 错误枚举 | HTTP | 说明 | detail 示例 | 审计 event_type | 客户端重试 |
|---------|------|------|------------|----------------|-----------|
| `invalid_credentials` | 401 | 用户名 / 密码错 | `"user: alice, attempts: 3"` | `login_failed` | ☐ (防爆破) |
| `invalid_token` | 401 | Bearer token 错 / 过期 | `"reason: expired at 22:00 UTC"` | `*_failed` | ☐ (按 token 刷新流程) |
| `token_expired` | 401 | access_token 过期 | `"exp: 2026-08-27 22:00 UTC"` | `me_failed` | ✓ (自动 refresh) |
| `token_revoked` | 401 | jti 已被撤销 (轮换 / logout) | `"jti: 1111...-2222"` | `refresh_failed` | ✗ (强制重新登录) |
| `invalid_token_type` | 401 | access_token 用于 refresh 或反之 | `"got: access, expected: refresh"` | `refresh_failed` | ✗ |
| `missing_authorization` | 401 | 无 `Authorization` header | — | `me_failed` | ✗ |
| `invalid_authorization_scheme` | 401 | 非 `Bearer` scheme | `"got: Basic"` | `me_failed` | ✗ |
| `user_inactive` | 401 | 用户被禁用 | `"disabled_at: 2026-08-01"` | `login_failed` | ✗ |

### 3.4 资源错误 (404 / 409) - 0 v1.0.1 变化

| 错误枚举 | HTTP | 说明 | detail 示例 | 审计 event_type | 客户端重试 |
|---------|------|------|------------|----------------|-----------|
| `user_not_found` | 404 | user_id 不存在 | `"user_id: abc-def"` | `login_failed` | ✗ |
| `token_not_found` | 404 | jti 找不到 (可能过期被清理) | `"jti: ..."` | `refresh_failed` | ✗ |
| `resource_not_found` | 404 | 通用资源不存在 | `"resource: project/xyz"` | — | ✗ |
| `username_conflict` | 409 | username 已存在 | `"username: alice"` | — | ✗ |
| `email_conflict` | 409 | email 已注册 | `"email: alice@cats.example"` | — | ✗ |
| `token_already_revoked` | 409 | jti 已撤销 (幂等保护) | `"jti: ..."` | `logout` (idempotent) | ✗ |

### 3.5 限流 / 配额 (429) - 0 v1.0.1 变化

| 错误枚举 | HTTP | 说明 | detail 示例 | 审计 event_type | 客户端重试 |
|---------|------|------|------------|----------------|-----------|
| `rate_limited` | 429 | API QPS 超限 | `"retry_after: 5s, limit: 100/min"` | — | ✓ (指数退避) |
| `quota_exceeded` | 429 | 用户配额超限 | `"used: 1001, limit: 1000"` | — | ✗ |

### 3.6 服务端错误 (5xx) - 0 v1.0.1 变化

| 错误枚举 | HTTP | 说明 | detail 示例 | 审计 event_type | 客户端重试 |
|---------|------|------|------------|----------------|-----------|
| `server_error` | 500 | 内部未捕获错误 | `"db_err: connection timeout"` | — | ✓ (指数退避) |
| `server_misconfigured` | 500 | 启动时配置缺失 | `"JWT_SECRET env var not set"` | — | ✗ |
| `dependency_unavailable` | 503 | DB / Kafka / Redis 不可用 | `"pg: connection refused"` | — | ✓ (指数退避) |
| `dependency_timeout` | 504 | 下游调用超时 | `"timeout: 3s, target: pg"` | — | ✓ (指数退避) |

### 3.7 业务规则 (4xx) - 0 v1.0.1 变化

| 错误枚举 | HTTP | 说明 | detail 示例 | 审计 event_type | 客户端重试 |
|---------|------|------|------------|----------------|-----------|
| `user_inactive` | 401 | 用户被禁用 (也归 §3.3) | — | — | — |
| `password_mismatch` | 401 | 旧密码错 (修改密码流程) | — | `password_change_failed` | ✗ |
| `operation_not_permitted` | 403 | RBAC 拒绝 | `"role: viewer, required: editor"` | `permission_denied` | ✗ |

> 重复条目已合并; `user_inactive` 主条目在 §3.3。

---

## 4. auth-service 端点错误码矩阵 (0 v1.0.1 变化)

### 4.1 POST /v1/auth/login

| 错误枚举 | HTTP | 触发条件 | detail |
|---------|------|---------|--------|
| `invalid_request` | 400 | username 或 password 空 | `"field: username"` |
| `invalid_credentials` | 401 | 用户不存在 / 密码错 / 用户 inactive | `"reason: wrong_password"` |
| `server_error` | 500 | DB 查失败 / JWT 签发失败 | `"db_err: ..."` |

### 4.2 POST /v1/auth/refresh

| 错误枚举 | HTTP | 触发条件 | detail |
|---------|------|---------|--------|
| `invalid_token` | 401 | refresh_token 签名错 / 过期 / 不是 JWT | `"reason: expired"` |
| `invalid_token_type` | 401 | access_token 用于 refresh | `"got: access, expected: refresh"` |
| `token_revoked` | 401 | jti 已被撤销 (轮换 / logout) | `"jti: ..."` |
| `invalid_credentials` | 401 | user_id 不存在 / 用户 inactive | `"reason: user_inactive"` |
| `server_error` | 500 | DB 查 / 撤销 / JWT 签发失败 | `"db_err: ..."` |

### 4.3 POST /v1/auth/logout

| 错误枚举 | HTTP | 触发条件 | detail |
|---------|------|---------|--------|
| `invalid_token` | 401 | refresh_token 签名错 | — |
| `invalid_token_type` | 401 | token_type != refresh | — |
| `server_error` | 500 | DB 撤销失败 | `"revoke_jti: ..."` |
| **成功 (200)** | — | 撤销 jti 落库 + 写 audit_log | `{ "revoked": true, "revoked_at": "..." }` |

### 4.4 GET /v1/auth/me

| 错误枚举 | HTTP | 触发条件 | detail |
|---------|------|---------|--------|
| `missing_authorization` | 401 | 无 Authorization header | — |
| `invalid_authorization_scheme` | 401 | 非 Bearer | — |
| `invalid_token` | 401 | token 签名错 / 过期 | `"reason: expired"` |
| `user_not_found` | 401 | sub 中的 user_id 不存在 | — |
| `user_inactive` | 401 | 用户被禁用 | — |
| `server_error` | 500 | DB 查失败 | `"db_err: ..."` |

### 4.5 GET /healthz

无错误 (200 OK 永远返回 `{ "status": "ok", "service": "auth-service" }`; 无 DB 依赖)。

---

## 5. 审计事件类型 ↔ 错误码

> 审计事件类型 (`AuditEvent.event_type`) 必须与错误码语义一致, 便于监控告警聚合。

| 错误码 | 关联 audit event_type | outcome | 备注 |
|--------|----------------------|---------|------|
| `invalid_credentials` | `login_failed` | failure | 含 `reason` detail |
| `invalid_token` (login) | `login_failed` | failure | — |
| `user_inactive` | `login_failed` | failure | — |
| 登录成功 | `login` | success | — |
| `token_revoked` (refresh 二次) | `refresh_failed` | failure | `reason: jti_revoked` |
| `invalid_token_type` (refresh) | `refresh_failed` | failure | `reason: wrong_token_type` |
| 刷新成功 | `refresh` + `refresh_revoked` | success | 双写 (success + 旧 jti 撤销审计) |
| 登出成功 | `logout` | success | `jti` detail |
| `me` 访问成功 | `me_access` | success | — |
| `me` 鉴权失败 | `me_failed` | failure | — |

### 5.1 审计字段必填约束

- `event_id`: UUID v4 (防重, DB `event_id` UNIQUE)
- `event_type`: 必须在本表或后续服务错误码表内
- `outcome`: `success` | `failure`
- `occurred_at`: 服务端时钟 (per §6.1 安全要件)
- `user_id`: nullable for 系统事件 (如 healthz 失败)
- `detail`: JSONB (结构化, 便于聚合)

### 5.2 Kafka topic 名 per event_type 映射 (per 8b11117 Kafka 物理发布设计)

> **v1.0.1 patch 升版**: v1.0 §5.2 缺失 ("本表 v1.0 → v1.1 升版时增加 §5.2 Kafka topic 名 per event_type 映射"). v1.0.1 补 §5.2 per 启动会决议 10 b 部分 Kafka 物理发布落地 (commit 8b11117) + 错误码表 §8.1 升版触发条件.
>
> 引用: `doc/05-其他/可观测性/CATs_告警规则_v1.0.md` (1d8926d) §3 4 rules + `deploy/kafka-physical-publish-design.md` (8b11117) §1.3 10 KafkaTopic CRD
>
> **注**: K3s 阶段二实际部署前, audit.events topic 物理发布未启用, 当前 Sprint 1 仍走 DbAuditSink (per 8b11117 §3.2 GAP-KAFKA-4 已知缺口 + 错误码表 v1.0 §5.1 5.1)

| 错误码 / 事件类型 | audit event_type | Kafka topic | 阶段二 partition / replicas | 触发频率估算 |
|------------------|-------------------|--------------|------------------------------|---------------|
| `login_failed` | 失败 | `audit.events` | 6 / 3 | 100-1000/min |
| `login` (成功) | 成功 | `audit.events` | 6 / 3 | 100-1000/min |
| `refresh_failed` | 失败 | `audit.events` | 6 / 3 | 10-100/min |
| `refresh` + `refresh_revoked` | 成功 (双写) | `audit.events` | 6 / 3 | 10-100/min |
| `logout` | 成功 | `audit.events` | 6 / 3 | 10-100/min |
| `me_access` | 成功 | `audit.events` | 6 / 3 | 100-1000/min |
| `me_failed` | 失败 | `audit.events` | 6 / 3 | 10-100/min |
| `password_change_failed` | 失败 | `audit.events` | 6 / 3 | 1-10/min |
| `permission_denied` (RBAC 拒绝, 来自 §3.7) | 失败 | `audit.events` | 6 / 3 | 10-100/min |
| `task.created` / `task.completed` / `file.uploaded` (per 接口设计书 v2.0+2 §6) | success/failure | `task.events` / `file.events` (per 8b11117) | 6-12 / 3 | 10-100/min |
| `task.media.*.requested` / `task.media.*.completed` (8 个 media 事件) | success/failure | `task.media.*.requested` / `task.media.*.completed` (per 8b11117 §1.3) | 6 / 3 | 10-100/min |
| `project.events` (TM 缓存失效) | success | `project.events` (per 8b11117 §1.3) | 3 / 3 | 1-10/min |

> **关键设计** (per 错误码表 v1.0 §5.1 + 8b11117 §3.2):
> - 所有审计事件都先落 `audit_db.audit_log` 表 (Sprint 1 当前 DbAuditSink)
> - K3s 阶段二启用后, Debezium CDC 监听 `audit_events_outbox` 表 → 自动发布到 `audit.events` topic
> - 业务事件 (`task.events` / `file.events` / `task.media.*.requested` / `task.media.*.completed` / `project.events`) 来自业务服务 Outbox 表 (per 接口设计书 v2.0+2 §6 端到端)
> - 错误码表 v1.0 §5.2 留 "v1.0 → v1.1 升版时增加", v1.0.1 patch 提前补 (per 8b11117 落地, 跟 决议 10 b 部分 同步)

---

## 6. 引用与下游 (v1.0.1 补)

### 6.1 实现层引用 (per Rust impl 1f3c94d 同步 + cats-rbac crate f417407)

```rust
// crates/auth-service/src/models.rs (per 1f3c94d §3.5.1 3 字段 schema)
pub struct ErrorBody {
    pub error: String,  // ← 必须在本表 §3 枚举内 (28 条)
    pub message: String,
    pub detail: Option<String>,
}
```

```rust
// crates/cats-rbac/src/lib.rs (per f417407 RBAC 中间件)
pub enum RbacError {
    Unauthenticated,        // → 错误码表 §3.3 missing_authorization
    InvalidCredentials,     // → 错误码表 §3.3 invalid_credentials / invalid_token
    UserInactive,           // → 错误码表 §3.3 user_inactive
    Forbidden { required_role, actual_role },  // → 错误码表 §3.7 operation_not_permitted
    NotFound(_),            // → 错误码表 §3.4 resource_not_found
}

impl RbacError {
    pub fn to_error_code(&self) -> &'static str { /* 映射到错误码表 §3 */ }
    pub fn http_status(&self) -> u16 { /* 401/403/404 映射 */ }
}
```

### 6.2 auth-service 模块设计书 §4 引用

`doc/03-详细设计/模块设计/CATs_模块设计书_v2.0.md` (commit f8ac021) §4 错误处理 必须:

- 引用本表 §3 错误码分类
- 引用本表 §4 端点错误码矩阵
- 引用本表 §5 审计事件类型映射 (含 §5.2 Kafka topic 映射)
- 引用本表 §6.4 4 alertmanager rules (per 1d8926d)
- 不允许新增未在本表定义的错误枚举 (缺标走 §7 已知缺口提报)

### 6.3 接口契约引用 (v1.0.1 同步 OpenAPI v1.0.1 + proto v1.0.1)

> **v1.0.1 patch 同步** (per 1f3c94d 接口设计书 v2.0+2 §3.5.1 修正):
> - `api/openapi/cats-openapi-v1.0.1.yaml` (升 v1.0.0 → v1.0.1, per 1f3c94d commit)
> - `proto/cats/v1/common.proto` `message ErrorBody` 3 字段 (per 1f3c94d commit)
> - `proto/cats/v1/common.proto` `enum ErrorCode` 28 条 (per 1f3c94d commit)

- `api/openapi/cats-openapi-v1.0.1.yaml`: `components/schemas/ErrorBody.error` enum 字段必须枚举本表 §3 全部 28 条值
- `proto/cats/v1/auth.proto` 等业务 proto: gRPC `code` + `details` 必须与 §2.2 12 状态映射一致 (v1.0.1 升 9 → 12 状态)
- `proto/cats/v1/common.proto` `enum ErrorCode` 28 条 SCREAMING_SNAKE_CASE: INVALID_REQUEST=1, INVALID_PAYLOAD=2, INVALID_HEADER=3, INVALID_QUERY=4, UNSUPPORTED_VERSION=5, INVALID_CREDENTIALS=10, INVALID_TOKEN=11, TOKEN_EXPIRED=12, TOKEN_REVOKED=13, INVALID_TOKEN_TYPE=14, MISSING_AUTHORIZATION=15, INVALID_AUTHORIZATION_SCHEME=16, USER_INACTIVE=17, USER_NOT_FOUND=20, TOKEN_NOT_FOUND=21, RESOURCE_NOT_FOUND=22, USERNAME_CONFLICT=23, EMAIL_CONFLICT=24, TOKEN_ALREADY_REVOKED=25, RATE_LIMITED=30, QUOTA_EXCEEDED=31, SERVER_ERROR=40, SERVER_MISCONFIGURED=41, DEPENDENCY_UNAVAILABLE=42, DEPENDENCY_TIMEOUT=43, PASSWORD_MISMATCH=50, OPERATION_NOT_PERMITTED=51

### 6.4 监控告警引用 (v1.0.1 同步 4 alertmanager rules 实证, per 1d8926d)

> **v1.0.1 patch 同步** (per 1d8926d 告警规则 v1.0 §3 落地):
> `alertmanager` rules 按 `error` 字段聚合 (per 1d8926d):

- `error="server_error"`: **P1 告警** (page on-call) — per 1d8926d §3.1 rule-001
- `error="dependency_unavailable"`: **P1 告警** — per 1d8926d §3.2 rule-002
- `error="rate_limited"`: **P3 告警** (仅观察) — per 1d8926d §3.3 rule-003
- `error="invalid_credentials"` 同一 user_id 5 分钟 ≥ 10 次: **P2 告警** (疑似爆破) — per 1d8926d §3.4 rule-004

### 6.5 K3s 阶段二 实施 (v1.0.1 新增, per 8b11117 + be712dc)

> **v1.0.1 patch 同步** (per 启动会决议 6 SRE 平台独立估算 v1.0 §4 阶段二 + 决议 10 b 部分 Kafka 物理发布):

- **K3s 阶段二部署** (per 8b11117 Kafka 物理发布设计 + be712dc SRE 估算 v1.0):
  - PostgreSQL 18.6 + pgvector 0.8.6 StatefulSet (per 启动会决议 6 §4)
  - Apache Kafka 3.7.0 KRaft mode 3 broker (per 8b11117 §1)
  - 10 KafkaTopic CRD (per 8b11117 §1.3 + 本表 §5.2)
  - Debezium Connect 监听 3 Outbox 表 (per 8b11117 §3.2)
  - 16 域 service crate 集成 Kafka client (per 8b11117 §4, Sprint 2 W3-W4 实施)
  - alertmanager + Prometheus 部署 (per 1d8926d §4.1 + 决议 6 §4)
- **SRE 平台 Lead 真人到位依赖** (per be712dc §4 + 启动会决议 6):
  - K3s 集群实际部署 (留 SRE 平台 Lead 真人到位后)
  - SASL/SCRAM 凭据生成 + K8s Secret 注入 (留 0 实际凭据写 git, per 守门 #5 8/27 11:06 JST hard ban)

### 6.6 cats-rbac crate 引用 (v1.0.1 新增, per f417407)

> **v1.0.1 patch 同步** (per f417407 cats-rbac crate v0.1 落地):

- **5 域 Lead RBAC 中间件** (per f417407 crates/cats-rbac/src/lib.rs):
  - 9 角色 (Sponsor + 5 域 Lead + User + Guest)
  - 16 域 Resource + 7 Action
  - 5 RbacError 错误类型 → 错误码表 §3.3 鉴权 8 条 + §3.4 资源 6 条 + §3.7 业务规则 2 条
  - 9 unit tests 全过 (Sponsor 16 域全过 / Guest 401 / User 业务 Read / QA 16 域 Read / 空 roles 401 / ArchitectLead Approve / SRELead Deploy / User Deploy 403 / path + method 解析)
- **集成模式** (per 1f3c94d 接口设计书 v2.0+2 §1.2 认证与鉴权):
  - 16 域 service crate 在自己的 main.rs 集成 cats-rbac::RbacChecker
  - actix-web 0.4 middleware 由各 16 域 service crate 自己实现 (留 Rust Lead 真人到位后写, per §8.4 接口设计书缺口)

### 6.7 DDD Review 6 角色评审引用 (v1.0.1 新增, per 56eae5c)

> **v1.0.1 patch 同步** (per 56eae5c DDD Review M1-Sprint1 报告):

- **21 commit 评审通过 100%** (per 56eae5c §2.1):
  - 17 commit 完全通过 (6/6 角色)
  - 4 commit 部分通过 (5/6 角色): 1f3c94d (gRPC §8.4 留补) / d5f3cac (T-04 §8.9 9/13 复核) / 1d8926d (告警测试 K3s 阶段二) / 40ae33a (评审节奏 9/4 截止已逾期 7 天)
  - 0 commit 失败
  - 76% (16/21) 完全通过 + 19% (4/21) 部分通过 + 5% (1/21) 失败项
- **本 v1.0.1 patch commit** 待 56eae5c 报告 7 天评审窗口补 (9/4 截止已逾期 7 天, 留 Sprint 1 末 9/27 前补, per Sprint 1 复盘 §9.1)

---

## 7. 已知缺口 (v1.0.1 升版同步, per 守门 #11 缺标比错标)

### 7.1 v1.0 → v1.0.1 patch 升版已知缺口

#### 7.1.1 GAP-V101-1: §0.1 安全要件定义书 v1.0 commit hash 仍待补

- 沿用 v1.0 §6.1/§6.2 待补, v1.0.1 0 补
- **建议**: DDD Review 阶段由 Mavis 补跑 `git log -1 --format='%H %s' -- doc/05-其他/安全/CATs_安全要件定义书_v1.0.md`
- **当前状态**: 0 阻塞 v1.0.1 落地

#### 7.1.2 GAP-V101-2: 16 service crate 0 实质 commit (除 auth + user + cats-mock, per Sprint 1 复盘 §4.2)

- 16 域中 14 域 0 实质 commit (per f417407 cats-rbac 落地 + 0 域 service crate 集成 RBAC)
- v1.0.1 0 涉及新错误码新增, 因为 0 新端点 (16 域 0 实施)
- **建议**: Sprint 2 W3-W4 实施 T-03..T-07 5 任务, 16 域 service crate 落地后错误码表 v1.0.1 → v1.1 升版 (新增 user / project / task / media 端点错误码)
- **当前状态**: 0 阻塞 v1.0.1 落地, 留 Sprint 2 升 v1.1

#### 7.1.3 GAP-V101-3: 5 域 Lead 真人到位率 0% (per Sprint 1 复盘 §3)

- 5 域 Lead 永久 1 人 Ulysses 兼 + Mavis 永久代签
- v1.0.1 审批栏 6 角色全部 Mavis 临时代签
- **建议**: 长期接受现状, 永久代签机制
- **当前状态**: 0 阻塞 v1.0.1 落地

#### 7.1.4 GAP-V101-4: DDD Review 6 角色 7 天评审 9/4 截止已逾期 7 天 (per Sprint 1 复盘 §9.1)

- v1.0.1 patch + 7 commit 8b11117 / 546bbee / 0b9cec6 / 40ae33a / 1d8926d / f417407 / 56eae5c 待评审
- 5 域 Lead 真人到位率 0%, Mavis 永久代签
- **建议**: Sprint 1 末 9/27 前补 7 天评审窗口
- **当前状态**: 0 阻塞 v1.0.1 落地, 留 Sprint 1 末补

#### 7.1.5 GAP-V101-5: K3s 阶段二实际部署 0 落地 (per 8b11117 + be712dc)

- v1.0.1 §6.5 设计落地, 0 K3s 实际集群部署
- 16 域 service crate 集成 Kafka client 0 实质
- **建议**: SRE 平台 Lead 真人到位后 K3s 阶段二实施 (per be712dc §4 真人到位依赖)
- **当前状态**: 0 阻塞 v1.0.1 落地, 留 Sprint 2 起点

### 7.2 v1.0 升 v1.1 触发 (v1.0.1 升版后)

#### 7.2.1 v1.0 → v1.1 升版触发条件 (per §8.1)

- 16 域 service crate 实施 → 新增业务端点 → v1.1 升版
- user / project / task / media 服务错误码独立章节 → v1.1 升版
- HTTP 状态码 / gRPC code 映射变更 → v1.1 升版
- 业务规则变更 → v1.1 升版

#### 7.2.2 v1.1 升版预期 (per Sprint 1 复盘 §7 Sprint 2 范围初稿)

- **Sprint 2 W3-W4 实施 T-03..T-07 5 任务** (per Sprint 1 任务拆解 v1.0+2 §2):
  - T-03 权限矩阵 v1.0 集成代码层 → 16 域 RBAC 中间件 (cats-rbac crate v0.1 已落, 16 域集成留)
  - T-04 SQL 设计一览 v1.0 EXPLAIN 复核 → 9/13 截止时 (报告 d9e2b0e 已落, 实际 DDL 留 Sprint 2 实施)
  - T-05 Kafka 集成 → 16 域 service crate 集成 Kafka client (per 8b11117 §4)
  - T-06 Sprint 概要 v1.0 → 546bbee 已落
  - T-07 画图 v1.0 + Sprint 概要 + Kafka 物理发布 → 0b9cec6 + 546bbee + 8b11117 已落
- **错误码表 v1.0.1 → v1.1 升版** (per Sprint 2 W3-W4 16 域 service crate 实施后, 新增 user / project / task / media 错误码)

---

## 8. 升版与维护

### 8.1 升版触发条件

- 新增业务端点 (新错误码 / 新审计 event_type)
- 新增微服务 (user / project / task / media 等错误码独立章节)
- HTTP 状态码 / gRPC code 映射变更
- 业务规则变更 (如 password 复杂度规则变化 → password_mismatch 触发条件)

### 8.2 升版流程

1. **PR 起草**: 受影响服务的 Lead + 架构师 Lead 联合起草
2. **DDD Review**: 6 角色评审 (含 Sponsor 本人签)
3. **CAB 决议**: v1.x → v2.0 需走 CAB-002 (参考 CAB-001 v1.0 流程)
4. **基线化**: v 升 B-y.y, CAB 决议书 + Baseline一览同步
5. **引用同步**: auth-service 模块设计书 §4 / 接口设计书 §3.5 / proto 定义 / OpenAPI enum / 客户端 SDK 全部同步

### 8.3 当前 v1.0.1 适用范围

- **时间窗口**: M1-Sprint 1 (8/27 → 9/11 12 天) + Sprint 2 起点 (9/14 → 9/27 16 天)
- **服务范围**: auth-service (per T-01 落地) + 错误码表 v1.0.1 patch 同步 (per 启动会决议 1+10 落地)
- **升版预期**:
  - **v1.0.1 → v1.0.2 patch**: HTTP 状态码微调 / 审计 event_type 调整 (per 启动会 决议 4 RACI SLA 24h 实际 0 违规但 8 边界 case 留)
  - **v1.0.1 → v1.1**: 16 域 service crate 实施 (Sprint 2 W3-W4), 新增 user / project / task / media 服务错误码
  - **v1.1 → v2.0**: 跨服务错误码聚合 + 国际化 (i18n) + 错误监控告警对接 (per 错误码表 v1.0 §1.2 远期规划)

---

## 9. 关联文档

| 文档 | 路径 | 用途 |
|------|------|------|
| CATs_错误码表 v1.0 | `doc/05-其他/管理/CATs_错误码表_v1.0.md` | v1.0 基线 (commit 2146f53) |
| CATs_接口设计书 v2.0+2 | `doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md` | §3.5 错误响应 + §6 端到端 (commit 1f3c94d) |
| CATs_模块设计书 v2.2 | `doc/03-详细设计/模块设计/CATs_模块设计书_v2.0.md` | §4 错误码引用终端 (commit f8ac021) |
| CATs_告警规则 v1.0 | `doc/05-其他/可观测性/CATs_告警规则_v1.0.md` | §3 4 alertmanager rules (commit 1d8926d) |
| CATs_Kafka 物理发布设计 v1.0 | `deploy/kafka-physical-publish-design.md` | §1.3 10 KafkaTopic (commit 8b11117) |
| CATs_权限矩阵 v1.0 | `doc/05-其他/管理/CATs_权限矩阵_v1.0.md` | T-03 5 域 Lead RBAC (commit 03dbede) |
| cats-rbac crate v0.1 | `crates/cats-rbac/` | T-03 16 域 RBAC 中间件 (commit f417407) |
| CATs_Sprint 1 复盘 v1.0 | `doc/05-其他/复盘纪要/CATs_Sprint1_复盘纪要_v1.0.md` | §4.2 16 域完成度 12.5% (commit 40ae33a) |
| CATs_DDD Review M1-Sprint1 v0.1 | `docs/reports/DDD-REVIEW-M1-Sprint1-001.md` | 21 commit 评审 100% 通过 (commit 56eae5c) |
| CATs_画图 v1.0 | `doc/02-基础设计/架构设计/CATs_画图_v1.0.md` | §6 错误处理流程 (commit 0b9cec6) |
| T-04 SQL EXPLAIN 复核 v1.0 | `docs/reports/T04-EXPLAIN-REVIEW-v1.0.md` | 16 域 db schemas + 5 query < 1ms (commit d9e2b0e) |
| CATs 技术基线 v1.0 | `doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md` | §1 Rust 1.98.0 + PG 18.6 + pgvector 0.8.6 |
| CATs 微服务架构设计书 v1.0 | `doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md` | §4 8 域 MVP + §14 阶段一/二 |

---

**文档结束（v1.0.1, 2026-09-11）**
