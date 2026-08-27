# CATs 错误码表 v1.0

> **文档编号**：CATs-DEV-012
> **版本**：v1.0
> **创建日**：2026-08-27
> **状态**：评审前草稿（待 6 角色签字基线化 per 8/27 16:33 JST 流程）
> **密级**：仅社内
> **作者**：架构师 + Rust Lead（Mavis 接手 agent per DEC-008，2026-08-27 19:39 JST Ulysses 授权代签）

---

## 文档管理信息

### 审批栏

| 角色 | 姓名 | 审批 | 日期 | 备注 |
|------|------|------|------|------|
| Sponsor (Ulysses 本人签) | Ulysses | ☐ | — | 一人公司 = Ulysses 持有 Sponsor 角色，不代签 |
| 架构师 Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 表设计 + 引用一致性 |
| Rust Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 实现对齐 |
| DBA Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 错误码索引建议 |
| QA Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 测试用例覆盖 |
| PMO Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 与 Sprint 1 §6 已知缺口对齐 |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|------|------|--------|----------|
| v1.0 | 2026-08-27 | 架构师 + Rust Lead（Mavis 接手 agent per DEC-008） | 初版：auth-service 错误码 v1.0（M1-Sprint 1 T-01 范围）+ 通用结构 + 6 类别映射 |

---

## 0. 元信息

| 项 | 值 |
|----|----|
| **作者** | 架构师 + Rust Lead（Mavis 代签 per DEC-008） |
| **Worktree** | `D:/CATs-wt-t01` |
| **分支** | `feature/cats-t01-auth-deepening` |
| **commit baseline** | `abb6f79`（per `git log -1`，M1-Sprint 1 §2 T-01 起点） |
| **关联基线（B0.0）** | `4f96f95`（CAB-001 v1.0） |
| **上游源文档** | 见 §0.1 源文档引用清单 |
| **下游引用** | auth-service 模块设计书 §4 / 接口设计书 v2.0 §3.5 / Sprint 1 拆解 v1.0 §2 T-01 / 150 任务 #55 SAST |
| **配套 Excel** | 无（表格在文档内可读） |

### 0.1 源文档引用清单（git 实证）

| 引用文档 | 路径 | commit hash | 用途 |
|---------|------|------------|------|
| CATs_技术基线 v1.0 | `doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md` | `047dc9c` | §1 Rust 1.98.0 + PG 18.6 + pgvector 0.8.6 |
| CATs_微服务架构设计书 v1.0 | `doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md` | `2910f3d` | §5.1 8 逻辑库 / §4.1 核心 8 MVP 服务 |
| CATs_接口设计书 v2.0 | `doc/02-基础设计/接口设计/CATs_接口设计书_v2.0.md` | （git log 待 §0.2 实证） | §3.5 错误响应统一格式 |
| CATs_安全要件定义书 v1.0 | `doc/05-其他/安全/CATs_安全要件定义书_v1.0.md` | （git log 待 §0.2 实证） | §3 认证 / §6 审计 |
| CATs_实施前QA登记册 v1.3 | `doc/05-其他/CATs_实施前QA登记册_v1.3.md` | `2910f3d`（历史值） | §2.2 OI-1 RBAC |
| CATs_M1_Sprint1_任务拆解 v1.0 | `doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md` | `8a83e19` | §2 T-01 错误码表 v1.0 提交并引用至 auth-service 模块设计书 §4 |

### 0.2 待补 git 实证（已知缺口 §6.1 / §6.2）

`CATs_接口设计书_v2.0.md` 与 `CATs_安全要件定义书_v1.0.md` 的精确 commit hash 在本 worktree 内未在 §0.1 引用时同步记录；本文撰写时仅通过 grep 引用其 §3.5 / §6 内容。**待 DDD Review 阶段由 Mavis 补跑 `git log -1 --format='%H %s' -- <path>` 并 patch 本文档 §0.1**（per "缺标比错标安全"原则）。

---

## 1. 概述

### 1.1 目的

为 CATs 全栈业务错误码建立单一事实源（Single Source of Truth），明确：

1. **业务错误枚举**（与实现层 `ErrorBody.error` 字段一一对应）
2. **HTTP 状态码映射**（4xx 客户端错误 / 5xx 服务端错误）
3. **业务语义解释**（产品 / 客户端 / 监控告警都能读懂）
4. **审计事件类型**（`AuditEvent.event_type` 枚举值来源）
5. **跨服务一致性**（auth / user / project / task / media 等服务遵循同一表）

### 1.2 范围

- **M1-Sprint 1 v1.0**：auth-service 全部端点（login / refresh / logout / me / healthz）
- **后续 v1.x 升版**：user-service / project-service / task-service / media-service 依次纳入
- **v2.0 远期**：跨服务错误码聚合 + 国际化 + 错误监控告警对接

### 1.3 引用与下游

- **实现层**：`auth-service::models::ErrorBody.error` 字段值（per `crates/auth-service/src/models.rs`）
- **审计层**：`auth-service::models::AuditEvent.event_type` 字段值
- **接口契约**：`api/openapi/cats-openapi-v1.yaml` + `proto/cats/v1/*.proto`（gRPC status mapping）
- **监控告警**：`alertmanager` rules 按 `error` 字段聚合（per 可观测性设计 §7）
- **客户端**：iOS / Android / Web / Unity 集成 SDK 按表做 i18n + 重试策略

---

## 2. 通用错误响应结构

### 2.1 REST 错误响应

```json
{
  "error": "invalid_token",
  "message": "token validation failed: expired",
  "detail": "expired at 2026-08-27 22:00:00 UTC"
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `error` | string | ✓ | 业务错误枚举（machine-readable, snake_case, 见 §3 表） |
| `message` | string | ✓ | 人类可读错误描述（i18n 前缀, 默认 en-US） |
| `detail` | string \| null | ☐ | 可选上下文（不暴露 secret / PII; 内部 detail 走 `detail` 不走 `message`） |

### 2.2 gRPC 错误响应

| HTTP 错误 | gRPC code | 说明 |
|----------|-----------|------|
| 400 | `INVALID_ARGUMENT` | 请求参数错 |
| 401 | `UNAUTHENTICATED` | 鉴权失败 |
| 403 | `PERMISSION_DENIED` | 权限不足 |
| 404 | `NOT_FOUND` | 资源不存在 |
| 409 | `ALREADY_EXISTS` | 资源冲突 |
| 422 | `FAILED_PRECONDITION` | 业务前置条件不满足 |
| 429 | `RESOURCE_EXHAUSTED` | 限流 |
| 500 | `INTERNAL` | 内部错误 |
| 503 | `UNAVAILABLE` | 依赖不可用 |

gRPC `details` 字段带 `ErrorBody` 序列化 payload（与 REST 一致）。

### 2.3 错误响应 Header 约定

- `X-Cats-Request-Id`: 请求追踪 ID（与 `tracing` span 关联）
- `X-Cats-Error-Code`: 业务错误枚举（与 body `error` 字段冗余，便于网关层 fast path）
- `WWW-Authenticate`: 仅 401 响应携带（`Bearer realm="cats", error="invalid_token"`）

---

## 3. 错误码分类

### 3.1 分类总览

| 类别 | 前缀 | 数量 (v1.0) | 范围 |
|------|------|------------|------|
| 客户端请求 | `invalid_*` | 5 | §3.2 |
| 鉴权 | `*token*` / `*credentials*` | 8 | §3.3 |
| 资源 | `*_not_found` / `*_conflict` | 6 | §3.4 |
| 限流 / 配额 | `rate_limited` / `quota_exceeded` | 2 | §3.5 |
| 服务端 | `server_*` | 4 | §3.6 |
| 业务规则 | `*_inactive` / `*_mismatch` 等 | 3 | §3.7 |
| **合计** | — | **28** | — |

### 3.2 客户端请求错误 (4xx)

| 错误枚举 | HTTP | 说明 | detail 示例 | 审计 event_type | 客户端重试 |
|---------|------|------|------------|----------------|-----------|
| `invalid_request` | 400 | 请求体缺少必填字段 | `"missing field: password"` | `login_failed` | ✗ |
| `invalid_payload` | 400 | JSON 解析失败 | `"line 3, column 5: expected `,`"` | — | ✗ |
| `invalid_header` | 400 | 请求头格式错 | `"Authorization: malformed"` | — | ✗ |
| `invalid_query` | 400 | 查询参数错 | `"limit must be 1..=100"` | — | ✗ |
| `unsupported_version` | 400 | API 版本不兼容 | `"X-API-Version: v2 not supported"` | — | ✗ |

### 3.3 鉴权错误 (401)

| 错误枚举 | HTTP | 说明 | detail 示例 | 审计 event_type | 客户端重试 |
|---------|------|------|------------|----------------|-----------|
| `invalid_credentials` | 401 | 用户名 / 密码错 | `"user: alice, attempts: 3"` | `login_failed` | ☐（防爆破） |
| `invalid_token` | 401 | Bearer token 错 / 过期 | `"reason: expired at 22:00 UTC"` | `*_failed` | ☐（按 token 刷新流程） |
| `token_expired` | 401 | access_token 过期 | `"exp: 2026-08-27 22:00 UTC"` | `me_failed` | ✓（自动 refresh） |
| `token_revoked` | 401 | jti 已被撤销（轮换 / logout） | `"jti: 1111...-2222"` | `refresh_failed` | ✗（强制重新登录） |
| `invalid_token_type` | 401 | access_token 用于 refresh 或反之 | `"got: access, expected: refresh"` | `refresh_failed` | ✗ |
| `missing_authorization` | 401 | 无 `Authorization` header | — | `me_failed` | ✗ |
| `invalid_authorization_scheme` | 401 | 非 `Bearer` scheme | `"got: Basic"` | `me_failed` | ✗ |
| `user_inactive` | 401 | 用户被禁用 | `"disabled_at: 2026-08-01"` | `login_failed` | ✗ |

### 3.4 资源错误 (404 / 409)

| 错误枚举 | HTTP | 说明 | detail 示例 | 审计 event_type | 客户端重试 |
|---------|------|------|------------|----------------|-----------|
| `user_not_found` | 404 | user_id 不存在 | `"user_id: abc-def"` | `login_failed` | ✗ |
| `token_not_found` | 404 | jti 找不到（可能过期被清理）| `"jti: ..."` | `refresh_failed` | ✗ |
| `resource_not_found` | 404 | 通用资源不存在 | `"resource: project/xyz"` | — | ✗ |
| `username_conflict` | 409 | username 已存在 | `"username: alice"` | — | ✗ |
| `email_conflict` | 409 | email 已注册 | `"email: alice@cats.example"` | — | ✗ |
| `token_already_revoked` | 409 | jti 已撤销（幂等保护）| `"jti: ..."` | `logout` (idempotent) | ✗ |

### 3.5 限流 / 配额 (429)

| 错误枚举 | HTTP | 说明 | detail 示例 | 审计 event_type | 客户端重试 |
|---------|------|------|------------|----------------|-----------|
| `rate_limited` | 429 | API QPS 超限 | `"retry_after: 5s, limit: 100/min"` | — | ✓（指数退避） |
| `quota_exceeded` | 429 | 用户配额超限 | `"used: 1001, limit: 1000"` | — | ✗ |

### 3.6 服务端错误 (5xx)

| 错误枚举 | HTTP | 说明 | detail 示例 | 审计 event_type | 客户端重试 |
|---------|------|------|------------|----------------|-----------|
| `server_error` | 500 | 内部未捕获错误 | `"db_err: connection timeout"` | — | ✓（指数退避） |
| `server_misconfigured` | 500 | 启动时配置缺失 | `"JWT_SECRET env var not set"` | — | ✗ |
| `dependency_unavailable` | 503 | DB / Kafka / Redis 不可用 | `"pg: connection refused"` | — | ✓（指数退避） |
| `dependency_timeout` | 504 | 下游调用超时 | `"timeout: 3s, target: pg"` | — | ✓（指数退避） |

### 3.7 业务规则 (4xx)

| 错误枚举 | HTTP | 说明 | detail 示例 | 审计 event_type | 客户端重试 |
|---------|------|------|------------|----------------|-----------|
| `user_inactive` | 401 | 用户被禁用（也归 §3.3） | — | — | — |
| `password_mismatch` | 401 | 旧密码错（修改密码流程） | — | `password_change_failed` | ✗ |
| `operation_not_permitted` | 403 | RBAC 拒绝 | `"role: viewer, required: editor"` | `permission_denied` | ✗ |

> 重复条目已合并；`user_inactive` 主条目在 §3.3。

---

## 4. auth-service 端点错误码矩阵

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
| `token_revoked` | 401 | jti 已被撤销（轮换 / logout） | `"jti: ..."` |
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

无错误（200 OK 永远返回 `{ "status": "ok", "service": "auth-service" }`；无 DB 依赖）。

---

## 5. 审计事件类型 ↔ 错误码

> 审计事件类型（`AuditEvent.event_type`）必须与错误码语义一致，便于监控告警聚合。

| 错误码 | 关联 audit event_type | outcome | 备注 |
|--------|----------------------|---------|------|
| `invalid_credentials` | `login_failed` | failure | 含 `reason` detail |
| `invalid_token` (login) | `login_failed` | failure | — |
| `user_inactive` | `login_failed` | failure | — |
| 登录成功 | `login` | success | — |
| `token_revoked` (refresh 二次) | `refresh_failed` | failure | `reason: jti_revoked` |
| `invalid_token_type` (refresh) | `refresh_failed` | failure | `reason: wrong_token_type` |
| 刷新成功 | `refresh` + `refresh_revoked` | success | 双写（success + 旧 jti 撤销审计） |
| 登出成功 | `logout` | success | `jti` detail |
| `me` 访问成功 | `me_access` | success | — |
| `me` 鉴权失败 | `me_failed` | failure | — |

### 5.1 审计字段必填约束

- `event_id`: UUID v4（防重，DB `event_id` UNIQUE）
- `event_type`: 必须在本表或后续服务错误码表内
- `outcome`: `success` | `failure`
- `occurred_at`: 服务端时钟（per §6.1 安全要件）
- `user_id`: nullable for 系统事件（如 healthz 失败）
- `detail`: JSONB（结构化，便于聚合）

---

## 6. 引用与下游

### 6.1 实现层引用

```rust
// auth-service/src/models.rs
pub struct ErrorBody {
    pub error: String,  // ← 必须在本表 §3 枚举内
    pub message: String,
    pub detail: Option<String>,
}
```

```rust
// auth-service/src/handlers.rs
fn unauthorized() -> HttpResponse {
    HttpResponse::Unauthorized().json(ErrorBody {
        error: "invalid_credentials".to_string(),  // ← §3.3
        ...
    })
}
```

### 6.2 auth-service 模块设计书 §4 引用

`doc/02-基础设计/模块设计/auth-service/CATs_auth-service_模块设计书_*.md §4 错误处理` 必须：

- 引用本表 §3 错误码分类
- 引用本表 §4 端点错误码矩阵
- 引用本表 §5 审计事件类型映射
- 不允许新增未在本表定义的错误枚举（缺标走 §7 已知缺口提报）

### 6.3 接口契约引用

- `api/openapi/cats-openapi-v1.yaml`: `components/schemas/ErrorBody.error` enum 字段必须枚举本表 §3 全部值
- `proto/cats/v1/auth.proto`: gRPC `code` + `details` 必须与 §2.2 映射一致

### 6.4 监控告警引用

`alertmanager` rules 按 `error` 字段聚合：

- `error="server_error"`: P1 告警（page on-call）
- `error="dependency_unavailable"`: P1 告警
- `error="rate_limited"`: P3 告警（仅观察）
- `error="invalid_credentials"` 同一 user_id 5 分钟 ≥ 10 次: P2 告警（疑似爆破）

---

## 7. 已知缺口（DDD Review 必查 per AI 协作文档治理 2026-08-26）

### 7.1 §0.1 接口设计书 / 安全要件 commit hash 未实证

- `CATs_接口设计书_v2.0.md` §3.5 错误响应统一格式 commit hash 未在 §0.1 引用时同步记录
- `CATs_安全要件定义书_v1.0.md` §3 认证 / §6 审计 commit hash 未在 §0.1 引用时同步记录
- **建议**：DDD Review 阶段由 Mavis 补跑 `git log -1 --format='%H %s' -- <path>` 并 patch §0.1
- **当前状态**：本表内容基于 §3.5 / §6 的语义约定，**未引用具体 commit hash**，如接口设计书 v2.0 升 v2.1 引入新错误枚举，需本表 v1.0 → v1.1 同步升版

### 7.2 T-01 Kafka audit 物理落地延后

- 本表 §5 审计事件类型 ↔ 错误码 已设计，但 `KafkaAuditSink` 仅是 stub（per `crates/auth-service/src/audit.rs`）
- Kafka 物理发布推到 K3s 阶段二（per Sprint 1 拆解 v1.0 §6.10 已知缺口）
- **当前状态**：T-01 范围内用 `DbAuditSink` 兜底（写 `audit_log` 表，事件永不丢）；Kafka topic 物理发布留 K3s
- **后续**：K3s 阶段二启动时，本表 v1.0 → v1.1 增加 §5.2 Kafka topic 名 per event_type 映射

### 7.3 多语言（i18n）message 字段未实现

- §2.1 `message` 字段默认 en-US，未做 i18n 资源文件分离
- **建议**：v1.1 增加 `Accept-Language` header 协商 + i18n 资源文件（`i18n/errors_zh-CN.json` 等）
- **当前状态**：T-01 范围仅 en-US，i18n 留 Sprint 2+

### 7.4 客户端 SDK 重试策略未基线化

- §3.2-§3.7 表格中"客户端重试"列基于 Rust Lead 经验判断
- **建议**：iOS / Android / Web 三个 SDK Lead 各自 review 一遍，必要时调整
- **当前状态**：仅供客户端参考，无强制约束

### 7.5 gRPC status code 映射覆盖不完整

- §2.2 表只覆盖了常见映射，proto 定义中可能有更细的 status code
- **建议**：与 `crates/proto` 实际生成的 `.rs` 对齐后补全
- **当前状态**：基础映射可用，未穷举

---

## 8. 修订与升版流程

### 8.1 升版触发条件

- 新增业务端点（新错误码 / 新审计 event_type）
- 新增微服务（user / project / task / media 等错误码独立章节）
- HTTP 状态码 / gRPC code 映射变更
- 业务规则变更（如 password 复杂度规则变化 → `password_mismatch` 触发条件）

### 8.2 升版流程

1. **PR 起草**：受影响服务的 Lead + 架构师 Lead 联合起草
2. **DDD Review**：6 角色评审（含 Sponsor 本人签）
3. **CAB 决议**：v1.x → v2.0 需走 CAB-002（参考 CAB-001 v1.0 流程）
4. **基线化**：v 升 B-y.y，CAB 决议书 + Baseline一览同步
5. **引用同步**：auth-service 模块设计书 §4 / 接口设计书 §3.5 / proto 定义 / OpenAPI enum / 客户端 SDK 全部同步

### 8.3 当前 v1.0 适用范围

- **时间窗口**：M1-Sprint 1（2026-08-27 起，至 Sprint 1 收尾）
- **服务范围**：auth-service（per T-01 完成判据 ④）
- **升版预期**：Sprint 1 末 / Sprint 2 启 升 v1.1（增加 user-service 错误码章节）

---

**文档结束（v1.0，2026-08-27）**
