# CATs 接口设计书

**系统名称:** CATs — 全媒体 AI 辅助翻译 SaaS 平台

---

## 文档管理信息

| 项目 | 内容 |
|---|---|
| 文档编号 | CATs-DD-API-002 |
| 文档名 | 接口设计书（详细设计 / 微服务 REST+gRPC+Kafka 契约） |
| 版本 | 第 2.1 版 |
| 创建日 | 2026-08-26 |
| 作者 | 架构师 + Rust Lead + DBA（worker 代签 per DEC-008） |
| 状态 | 评审前草稿 |
| 密级 | 仅社内 |
| 上游文档 | [CATs 微服务架构设计书 v1.0](../../02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md)、[CATs 技术选型书 v2.0](../../02-基础设计/技术选型/CATs_技术选型书_v2.0.md)、[CATs 命名变更说明](../../02-基础设计/架构设计/CATs_命名变更说明.md)、[CATs_技术基线 v1.0](../../02-基础设计/技术选型/CATs_技术基线_v1.0.md)（**§1 PostgreSQL 18.6 + pgvector 0.8.6**）、[OFCAT 接口设计书 v1.0（历史/旧架构参考，格式沿用）](./OFCAT_接口设计书_v1.0.md) |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|---|---|---|---|
| 1.0 | 2026-06-25 | 架构师 | （OFCAT）扩展↔引擎 API-01~10 完整契约、SSE 事件、错误码，见历史文档 |
| 2.0 | 2026-08-18 | 架构师 | 全面重做：微服务化后 15 个服务的 REST/gRPC/Kafka 完整接口契约，承接《CATs 微服务架构设计书 v1.0》§4、§6、§9 |
| **2.1** | **2026-08-26** | **架构师 + Rust Lead + DBA** | **基线升级：PostgreSQL 引用统一引用 CATs_技术基线_v1.0 §1（PostgreSQL 18.6 + pgvector 0.8.6）** |

### 审批栏

| 角色 | 姓名 | 审批日 | 签字 |
|---|---|---|---|
| 起草 | 架构师 + Rust Lead + DBA | 2026-08-26 |  |
| 评审 |  |  |  |
| 批准 |  |  |  |

---

## 0. 阅读指南

本书是《CATs 微服务架构设计书 v1.0》§21「与详细设计的衔接」中承诺的接口详细设计文档，覆盖 15 个微服务（8 个核心业务服务 + translation-core + 6 个媒体处理服务）的完整对外/对内契约：REST/gRPC API 清单、Kafka 事件 schema、错误码、鉴权方式、版本管理策略。数据库表结构见《CATs 数据库设计书 v2.0》，服务内部模块划分见《CATs 模块设计书 v2.0》。涉及后端技术栈版本（PostgreSQL / pgvector 等）一律以 [CATs_技术基线_v1.0 §1](../../02-基础设计/技术选型/CATs_技术基线_v1.0.md) 为准。

---

## 1. 通用约定

### 1.1 基址与协议

| 项 | 约定 |
|---|---|
| 对外基址 | `https://api.cats.internal/v1/*`（REST，经 Envoy Gateway HTTPRoute） |
| 对外 gRPC 基址 | `grpc.cats.internal:8443`（经 Envoy Gateway GRPCRoute，TLS 终结） |
| 服务间内部调用 | K3s 集群内 Service DNS，如 `task-service.cats-core.svc.cluster.local:8080`，不经过 Envoy Gateway |
| 编码 | `application/json; charset=utf-8`；时间为 ISO-8601 UTC 字符串（`2026-08-18T09:00:00Z`） |
| gRPC 编码 | Protobuf 3，包名版本化（见 §5） |
| 幂等键 | 所有写操作（POST 创建类）支持请求头 `Idempotency-Key: <uuid>`，服务端在 24h 内对相同 Key 返回首次结果，不重复执行副作用 |

### 1.2 认证与鉴权

| 层级 | 方式 | 说明 |
|---|---|---|
| 客户端/控制台 → Envoy Gateway | `Authorization: Bearer <JWT>` | JWT 由 auth-service 签发（RS256），Gateway 侧用 `SecurityPolicy`（Envoy Gateway JWT 扩展）做签名与过期时间校验，校验失败直接 401，不转发到后端，减轻业务服务鉴权负担 |
| Gateway → 核心业务服务 | JWT 透传 + Header `X-Cats-User-Id`/`X-Cats-Org-Id`/`X-Cats-Roles`（Gateway 从 JWT claims 解析后注入，业务服务信任该 Header，不重复验签） | 业务服务只做**业务级权限**校验（如"这个项目是否属于该 org_id"），不重复做签名校验 |
| 服务间同步调用（gRPC） | mTLS（K8s 内部 CA，cert-manager 签发短期证书，自动轮换）+ 服务账号 Token（K8s ServiceAccount Token Volume 投影，作为 gRPC metadata `x-service-token` 二次校验） | 对应架构设计书 §14：阶段一/二不上服务网格，用应用层 mTLS + NetworkPolicy 组合实现 |
| Kafka 生产/消费 | SASL/SCRAM 凭据 + 按 Topic 的 ACL（架构设计书 §14） | 每个服务独立凭据，最小权限 |

### 1.3 统一错误信封

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "字段 target_lang 不是合法的 BCP-47 语言代码",
    "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
    "details": { "field": "target_lang" }
  }
}
```

### 1.4 统一错误码表

| code | HTTP | gRPC status | 含义 | 适用范围 |
|---|---|---|---|---|
| `VALIDATION_ERROR` | 400 | `INVALID_ARGUMENT` | 请求体校验失败 | 全部 |
| `UNAUTHORIZED` | 401 | `UNAUTHENTICATED` | JWT 缺失/过期/签名无效 | Gateway 层拦截为主 |
| `FORBIDDEN` | 403 | `PERMISSION_DENIED` | 已认证但无权限访问该资源（RBAC/租户隔离） | 全部业务服务 |
| `NOT_FOUND` | 404 | `NOT_FOUND` | 资源不存在 | 全部 |
| `CONFLICT` | 409 | `ALREADY_EXISTS` | 唯一约束冲突/状态机非法迁移 | task-service/project-service |
| `COMPLIANCE_BLOCKED` | 409 | `FAILED_PRECONDITION` | 敏感项目命中合规策略且本地模型不可达，fail-closed（延续 OFCAT 原则） | translation-core |
| `QA_BLOCKED` | 422 | `FAILED_PRECONDITION` | QA 阻断项存在，禁止提交/导出 | translation-core、task-service |
| `RATE_LIMITED` | 429 | `RESOURCE_EXHAUSTED` | 超出 Envoy `BackendTrafficPolicy` 限流阈值 | Gateway 层为主 |
| `UPSTREAM_ERROR` | 502 | `UNAVAILABLE` | 依赖的下游服务/模型调用失败（含重试后仍失败） | translation-core/asr/ocr |
| `INTERNAL_ERROR` | 500 | `INTERNAL` | 服务内部错误 | 全部 |
| `UPSTREAM_TIMEOUT` | 504 | `DEADLINE_EXCEEDED` | 下游调用超时 | translation-core/asr/ocr |

### 1.5 版本管理策略

- **REST**：URL 路径版本化 `/v1/...`，破坏性变更（字段删除/类型变更/语义变更）必须发布 `/v2/...` 新路径并至少并行运行 2 个发布周期，非破坏性变更（新增可选字段）直接在 `/v1/` 内追加，不升版本号。
- **gRPC**：Protobuf package 版本化，如 `cats.task.v1`、`cats.translation.v1`；破坏性变更发布新 package `cats.task.v2`，旧 package 保留至确认无消费者依赖后下线。字段变更遵循 Protobuf 兼容性规则（不复用/不改变已分配的字段号，删除字段用 `reserved` 占位）。
- **Kafka Event Schema**：JSON 消息体内 `schema_version` 整数字段（架构设计书 §6.6），新增字段向后兼容，破坏性变更发布新 `event_type`（如 `task.completed.v2`）而非直接修改旧 schema。
- 所有 API 变更须在对应服务仓库的 `CHANGELOG.md` 登记，并在 CI 中跑 OpenAPI/Protobuf 兼容性校验（`buf breaking` / `openapi-diff`）作为合并门禁。

---

## 2. 服务间调用关系总览（文字化拓扑）

```
Tauri客户端 / Next.js控制台
        │ REST(JWT) / WebSocket / SSE
        ▼
   Envoy Gateway（JWT 校验、限流、灰度）
        │
        ├─ 同步 REST ──▶ auth-service ──▶ auth_db
        ├─ 同步 REST ──▶ user-service ──▶ user_db
        ├─ 同步 REST ──▶ project-service ──▶ project_db
        │                     │ 同步 gRPC（TM/术语查询）
        │                     ▼
        ├─ 同步 REST ──▶ task-service ──▶ task_db
        │                     │ 同步 gRPC AuthCheck
        │                     ▼
        │               auth-service（校验 Token 有效性，Valkey 缓存结果）
        │
        ├─ 同步 REST ──▶ file-service ──▶ file_db + 对象存储(MinIO)
        │
        ├─ WebSocket ──▶ notification-service ──▶ notification_db
        │
        ├─ 同步 REST(管理端) ──▶ report-service ──▶ report_db
        └─ 同步 REST(管理端) ──▶ audit-service ──▶ audit_db

异步 Kafka 事件面（task-service 为核心编排者，不直连各媒体处理服务）：

task-service ──Outbox+Debezium──▶ Kafka(task.media.*.requested)
        │
        ├─▶ ingestion-service（消费首个 requested 事件，判定媒体类型，派生子任务）
        ├─▶ asr-service      （消费 task.media.asr.requested，产出 .completed）
        ├─▶ ocr-service       （消费 task.media.ocr.requested，产出 .completed）
        ├─▶ subtitle-service  （消费 asr/翻译完成事件，产出字幕文件）
        ├─▶ office-converter-service（消费 task.media.office.requested）
        ├─▶ translation-core（同步 gRPC 被 task-service/subtitle-service/office-converter-service 调用做分段翻译，
        │                      同时消费 project.events 失效本地 TM/术语缓存）
        └─▶ render-writer-service（消费 task.media.render.requested，产出最终文件）
                    │
                    ▼ 同步 REST 调用 file-service 存最终产物
                    │
        各阶段完成 ──▶ task-service（同步 REST 上报进度，写 task_events_outbox）
                    │
                    ▼ Kafka(task.events)
        notification-service（消费，WebSocket/邮件推送）
        report-service（消费，用量/质量统计）
        audit-service（消费 audit.events，独立通道）
```

---

## 3. 核心业务服务 API 详细契约

### 3.1 auth-service

**职责与边界**：用户认证（账号密码/OIDC）、Session/JWT 签发与刷新、RBAC 角色与权限绑定定义。数据边界 `auth_db`，唯一权威写入方。

| Method | Path | 说明 | 认证 | 幂等性 |
|---|---|---|---|---|
| POST | `/v1/auth/login` | 账号密码登录，返回 JWT access/refresh token | 无（登录接口本身） | 否（每次生成新 Session） |
| POST | `/v1/auth/oidc/callback` | OIDC 回调换取 Token | 无 | 否 |
| POST | `/v1/auth/refresh` | 用 refresh_token 换取新 access_token | 无（refresh_token 即凭据） | 是（同一 refresh_token 短时间重复请求返回相同新 Token，防止刷新风暴） |
| POST | `/v1/auth/logout` | 吊销当前 Session | Bearer | 是 |
| GET | `/v1/auth/roles` | 查询角色/权限定义列表（管理端） | Bearer + `role:admin` | 是（只读） |
| POST | `/v1/auth/roles/{role_id}/bindings` | 绑定角色到用户 | Bearer + `role:admin` | 是（`Idempotency-Key`） |

**`POST /v1/auth/login` 请求/响应示例**：

```json
// 请求
{ "email": "user@org.internal", "password": "***" }
// 响应 200
{
  "access_token": "eyJhbGciOi...",
  "refresh_token": "eyJhbGciOi...",
  "expires_in": 7200,
  "user_id": "usr_01HZX...",
  "org_id": "org_01HZX..."
}
```

**gRPC 内部接口**（供其他服务同步校验 Token，架构设计书 §4.2）：

```protobuf
// package cats.auth.v1
service AuthInternal {
  rpc AuthCheck(AuthCheckRequest) returns (AuthCheckResponse);
}
message AuthCheckRequest { string access_token = 1; }
message AuthCheckResponse {
  bool valid = 1;
  string user_id = 2;
  string org_id = 3;
  repeated string roles = 4;
  int64 expires_at_unix = 5;
}
```
调用方（task-service/project-service 等）在 Valkey 以 `sess:{token_hash}` 缓存该结果，TTL 与 Token 剩余有效期一致，减少对 auth-service 的高频调用压力（架构设计书 §11.1）。

**异步事件**：`auth.events`（Topic 复用 `user.events`，`event_type = 'auth.login_succeeded' / 'auth.login_failed' / 'auth.password_changed'`），供 audit-service 消费落审计日志。

---

### 3.2 user-service

**职责**：用户/组织资料、成员邀请、订阅套餐。数据边界 `user_db`。

| Method | Path | 说明 | 认证 | 幂等性 |
|---|---|---|---|---|
| GET | `/v1/users/{id}` | 查询用户资料 | Bearer | 是 |
| PUT | `/v1/users/{id}` | 更新用户资料 | Bearer（本人或 org admin） | 是（整体覆盖式 PUT） |
| POST | `/v1/orgs` | 创建组织 | Bearer | 否 |
| GET | `/v1/orgs/{id}/members` | 组织成员列表 | Bearer | 是 |
| POST | `/v1/orgs/{id}/members/invite` | 邀请成员（发邮件邀请链接） | Bearer + `role:org_admin` | 否（`Idempotency-Key` 防重复发邮件） |
| GET | `/v1/orgs/{id}/subscription` | 查询订阅套餐/配额 | Bearer | 是 |
| PUT | `/v1/orgs/{id}/subscription` | 变更订阅套餐（内部管理端调用，非自助） | Bearer + `role:platform_admin` | 是 |

**响应示例**（`GET /v1/orgs/{id}/subscription`）：
```json
{
  "org_id": "org_01HZX...",
  "plan": "team",
  "seats_limit": 50,
  "seats_used": 12,
  "monthly_media_minutes_quota": 6000,
  "monthly_media_minutes_used": 812,
  "renews_at": "2026-09-01T00:00:00Z"
}
```

**异步事件**：Topic `user.events`（架构设计书 §6.2），Partition Key `org_id`。

| event_type | payload 关键字段 | 消费方 |
|---|---|---|
| `user.created` | `user_id, org_id, email` | audit-service |
| `org.member_added` | `org_id, user_id, role` | audit-service, notification-service |
| `org.subscription_changed` | `org_id, plan, seats_limit` | report-service（用量对账基线）, task-service（配额校验缓存失效） |

---

### 3.3 project-service

**职责**：项目、语言对、领域、术语库/TM 元数据、敏感策略配置。数据边界 `project_db`（与 translation-core 共享该库，见架构设计书 §5.1）。

| Method | Path | 说明 | 认证 | 幂等性 |
|---|---|---|---|---|
| POST | `/v1/projects` | 创建项目 | Bearer | 否（`Idempotency-Key` 支持） |
| GET | `/v1/projects/{id}` | 项目详情 | Bearer | 是 |
| GET | `/v1/projects/{id}/glossary` | 术语表列表（分页） | Bearer | 是 |
| POST | `/v1/projects/{id}/glossary` | 新增术语条目 | Bearer | 否 |
| PUT | `/v1/projects/{id}/glossary/{term_id}` | 更新术语条目（触发术语库版本号自增） | Bearer | 是 |
| GET | `/v1/projects/{id}/tm/search` | TM 检索（精确匹配 + 语义召回，query 参数 `q,source_lang,target_lang,top_k`） | Bearer | 是（只读） |
| PUT | `/v1/projects/{id}/compliance-policy` | 更新敏感策略配置（是否强制路由本地模型等） | Bearer + `role:org_admin` | 是 |

**gRPC 内部接口**（供 translation-core 同步查询，架构设计书 §4.2「读多写少」场景）：

```protobuf
// package cats.project.v1
service ProjectInternal {
  rpc GetGlossary(GetGlossaryRequest) returns (GetGlossaryResponse);
  rpc SearchTM(SearchTMRequest) returns (SearchTMResponse);
}
message SearchTMRequest {
  string project_id = 1;
  string source_lang = 2;
  string target_lang = 3;
  string source_text = 4;
  int32 top_k = 5;          // 语义召回候选数，默认 5
  double min_score = 6;      // 精确匹配最低分阈值，默认 75
}
message SearchTMResponse {
  string level = 1;          // L0(100%精确) / L1(模糊匹配) / MISS
  TMMatch best = 2;
  repeated TMMatch candidates = 3;
}
message TMMatch { double score = 1; string source = 2; string target = 3; string origin = 4; }
```

**异步事件**：Topic `project.events`，Partition Key `project_id`。

| event_type | payload 关键字段 | 消费方 |
|---|---|---|
| `project.glossary_updated` | `project_id, term_id, glossary_version` | translation-core（失效本地术语缓存） |
| `project.tm_updated` | `project_id, tm_entry_id` | translation-core（失效 TM 缓存） |
| `project.compliance_policy_changed` | `project_id, force_local_model` | translation-core、task-service |

---

### 3.4 task-service

**职责**：翻译任务生命周期管理与媒体处理编排入口，任务与媒体类型路由分发。数据边界 `task_db`。**是整个媒体处理管道的编排中枢**：媒体处理服务不互相直连，均通过消费 `task.media.*` 事件、再调用 task-service 的进度上报 API 完成协同。

| Method | Path | 说明 | 认证 | 幂等性 |
|---|---|---|---|---|
| POST | `/v1/tasks` | 创建翻译任务（`media_type` 决定后续处理链路） | Bearer | 否（`Idempotency-Key` 强制要求） |
| GET | `/v1/tasks/{id}` | 任务详情（当前状态、各阶段进度） | Bearer | 是 |
| GET | `/v1/tasks/{id}/events`（SSE） | 任务进度实时推送 | Bearer | 是（只读流） |
| POST | `/v1/tasks/{id}/cancel` | 取消任务 | Bearer | 是 |
| POST | `/v1/tasks/{id}/retry` | 重试失败阶段（不重跑已成功阶段，见架构设计书 §8.1） | Bearer | 是 |
| **内部** POST | `/internal/v1/tasks/{id}/stage-progress` | 媒体处理服务上报某阶段进度/完成/失败（mTLS 服务间调用，非对外） | mTLS+服务账号 | 是（`Idempotency-Key = event_id`） |

**`POST /v1/tasks` 请求示例**：
```json
{
  "project_id": "prj_01HZX...",
  "media_type": "video",
  "source_lang": "ja",
  "target_lang": "zh-CN",
  "source_file_id": "file_01HZX...",
  "output_formats": ["srt", "mp4_burned_in"],
  "priority": "normal"
}
```
`media_type ∈ {text, audio, video, pdf, docx, xlsx, pptx, odt, ods, odp, gif, webp}`。

**响应 201**：
```json
{ "task_id": "tsk_01HZX...", "status": "queued", "created_at": "2026-08-18T09:00:00Z" }
```

**`/internal/v1/tasks/{id}/stage-progress` 请求示例**（asr-service 上报 ASR 阶段完成）：
```json
{
  "event_id": "evt_9f2c...",
  "stage": "asr",
  "status": "completed",
  "result_ref": { "file_id": "file_02HZX...", "kind": "transcript" },
  "metrics": { "duration_seconds": 812, "process_seconds": 96 }
}
```

**任务状态机**：`queued → ingesting → processing(各阶段并行/串行子状态见 task_media_items 表) → rendering → completed | failed | canceled | partially_failed`

**异步事件**：Topic `task.events`（生命周期状态变更）与 `task.media.*.requested/.completed`（各媒体处理阶段），全部经 Outbox（`task_events_outbox` 表）+ Debezium 发布，详见架构设计书 §6.2、§7 与本文档 §6。

---

### 3.5 file-service

**职责**：原始文件/中间产物/最终产物的存取、预签名 URL、版本管理。数据边界 `file_db` + 对象存储（MinIO）。

| Method | Path | 说明 | 认证 | 幂等性 |
|---|---|---|---|---|
| POST | `/v1/files/presign-upload` | 申请预签名上传 URL（大文件走对象存储直传，不经过应用层） | Bearer | 否 |
| POST | `/v1/files` | 确认上传完成，登记文件元数据（小文件也可走 multipart 直接上传本接口） | Bearer | 否（`Idempotency-Key`） |
| GET | `/v1/files/{id}` | 文件元数据 | Bearer | 是 |
| GET | `/v1/files/{id}/download` | 预签名下载 URL（302 跳转或直接返回签名 URL JSON） | Bearer | 是 |
| POST | `/v1/files/{id}/versions` | 登记新版本（如渲染回写产出新文件版本） | mTLS服务间调用 | 否 |
| DELETE | `/v1/files/{id}` | 软删除（按保留策略延迟物理清理，见数据库设计书） | Bearer | 是 |

**`POST /v1/files/presign-upload` 请求/响应**：
```json
// 请求
{ "filename": "source.mp4", "content_type": "video/mp4", "size_bytes": 209715200, "project_id": "prj_01HZX..." }
// 响应 200
{
  "file_id": "file_01HZX...",
  "upload_url": "https://minio.cats.internal/cats-uploads/....?X-Amz-Signature=...",
  "expires_at": "2026-08-18T09:15:00Z"
}
```

**异步事件**：Topic `file.events`（复用 `task.events` 命名空间，`event_type='file.uploaded'/'file.version_added'`），Partition Key `file_id`，供 ingestion-service 触发处理链路（架构设计书 §2.2）。

---

### 3.6 notification-service

**职责**：站内信、WebSocket 推送、邮件通知。数据边界 `notification_db`。

| Method | Path | 说明 | 认证 | 幂等性 |
|---|---|---|---|---|
| WebSocket | `/v1/ws/notify` | 建立长连接，服务端推送通知事件 | Bearer（连接建立时校验一次） | — |
| GET | `/v1/notifications` | 站内信历史列表 | Bearer | 是 |
| POST | `/v1/notifications/{id}/read` | 标记已读 | Bearer | 是 |
| PUT | `/v1/notification-prefs` | 更新通知偏好（邮件开关等） | Bearer | 是 |
| POST | `/v1/notify/test`（内部管理端） | 测试通知渠道连通性 | Bearer + `role:platform_admin` | 是 |

**WebSocket 推送消息示例**：
```json
{ "type": "task.completed", "task_id": "tsk_01HZX...", "title": "视频翻译已完成", "ts": "2026-08-18T09:12:30Z" }
```

**异步事件消费**：消费 `notification.events`（Consumer Group `notification-service-group`），事件体：
```json
{
  "event_id": "evt_...", "schema_version": 1,
  "user_id": "usr_01HZX...", "channel_hint": ["ws", "email"],
  "template": "task_completed", "vars": { "task_id": "tsk_01HZX...", "task_title": "..." }
}
```

---

### 3.7 report-service

**职责**：用量统计、计费明细、质量报表。数据边界 `report_db`，只读消费其他服务事件，不对外提供写接口（管理端只读）。

| Method | Path | 说明 | 认证 | 幂等性 |
|---|---|---|---|---|
| GET | `/v1/reports/usage` | 用量报表（按 org/project/时间范围聚合） | Bearer + `role:org_admin` | 是 |
| GET | `/v1/reports/billing` | 计费明细 | Bearer + `role:org_admin` | 是 |
| GET | `/v1/reports/qa-stats` | QA 命中率/术语一致性统计 | Bearer + `role:org_admin` | 是 |

**异步事件消费**：`task.events`（统计任务量/媒体分钟数）、`user.events`（订阅变更基线）、`asr.metrics`/`ocr.metrics`（可选，若需精细成本核算，复用 `task.media.*.completed` 事件中的 `metrics` 字段即可，不新增 Topic）。

---

### 3.8 audit-service

**职责**：关键操作审计日志落库，供合规追溯，只消费不产出业务事件。数据边界 `audit_db`。

| Method | Path | 说明 | 认证 | 幂等性 |
|---|---|---|---|---|
| GET | `/v1/audit-logs` | 审计日志查询（分页，按 org/user/action/时间范围过滤，内部管理端） | Bearer + `role:platform_admin` | 是 |
| GET | `/v1/audit-logs/{id}` | 单条详情 | Bearer + `role:platform_admin` | 是 |

**异步事件消费**：统一消费 `audit.events` Topic（架构设计书 §4.2「各服务 → audit-service」），事件体：
```json
{
  "event_id": "evt_...", "schema_version": 1,
  "org_id": "org_...", "actor_user_id": "usr_...",
  "action": "glossary.term_updated", "resource_type": "project", "resource_id": "prj_...",
  "before": { "target_term": "旧译法" }, "after": { "target_term": "新译法" },
  "ip": "10.0.12.3", "occurred_at": "2026-08-18T09:00:00Z"
}
```
各业务服务在自身领域事件发生时**额外**产出一条对应 `audit.events` 事件（同一 Outbox 事务内多条 Outbox 记录，见架构设计书 §7.2），而非由 audit-service 反向订阅所有领域 Topic 再自行判定哪些需要审计——后者会让 audit-service 耦合全平台事件语义，前者职责更清晰。

---

### 3.9 worker-service

**职责**：通用异步任务执行框架/调度（Cron 型任务、批量导入清洗、对账补偿任务，如架构设计书 §8.1 提到的"定时对账补偿任务"）。无独立数据库，读写 `task_db` 的状态字段（通过 task-service 内部 API，不直连）。无对外 API。

| 内部机制 | 说明 |
|---|---|
| Cron 调度 | K8s `CronJob` 或进程内调度器（如 `tokio-cron-scheduler`/`APScheduler`，视实现语言），任务清单纳入 GitOps 配置仓 |
| 对账任务 | 定时扫描 `task_db.tasks` 中状态长时间未推进（如 `processing` 超过 SLA 阈值未变化）的任务，调用 task-service `/internal/v1/tasks/{id}/retry` 或标记失败 |
| 批量导入 | 消费 `file.events` 中的 `bulk_import.requested`（管理端触发批量项目/术语库导入），逐条清洗校验后调用 project-service API 写入 |

---

### 3.10 translation-core

**职责**：编排翻译管道（TM 匹配 → 术语匹配注入 → 标签保护 → 模型翻译 → 术语校验 → QA），延续 OFCAT LangGraph 编排逻辑，迁移为常驻服务。复用 `project_db` 内 TM/术语相关表，自身不新建独立库。

**gRPC 主接口**（供 task-service/subtitle-service/office-converter-service 同步调用做分段翻译）：

```protobuf
// package cats.translation.v1
service TranslationCore {
  rpc Translate(TranslateRequest) returns (stream TranslateEvent);  // 流式，事件序列同 OFCAT SSE 语义
  rpc TranslateBatch(TranslateBatchRequest) returns (TranslateBatchResponse); // 字幕/文档场景批量分段翻译
  rpc TMMatch(TMMatchRequest) returns (TMMatchResponse);
  rpc QACheck(QACheckRequest) returns (QACheckResponse);
}

message TranslateRequest {
  string segment = 1;
  string source_lang = 2;
  string target_lang = 3;
  string project_id = 4;
  string mode = 5;            // default(L2) / high_quality(L3)
  string task_id = 6;         // 用于 Trace 关联与合规策略查询
}
message TranslateEvent {
  oneof payload {
    TmHit tm_hit = 1;
    TermsMatched terms = 2;
    DeltaText delta = 3;
    QaResult qa = 4;
    Done done = 5;
  }
}

message TranslateBatchRequest {
  string project_id = 1;
  string source_lang = 2;
  string target_lang = 3;
  string task_id = 4;
  repeated Segment segments = 5;   // 字幕分段/文档段落，携带 segment_id 保证顺序回填
}
message Segment { string segment_id = 1; string text = 2; }
message TranslateBatchResponse { repeated TranslatedSegment results = 1; }
message TranslatedSegment { string segment_id = 1; string target_text = 2; string tm_level = 3; bool qa_pass = 4; }
```

**REST 内部管理接口**（非对外，管理端调试用）：

| Method | Path | 说明 |
|---|---|---|
| GET | `/internal/v1/translation-core/cache-stats` | 查看当前 TM/术语本地缓存命中率与缓存条目数 |
| POST | `/internal/v1/translation-core/cache/invalidate` | 手动强制失效指定 project_id 的本地缓存（排障用，正常路径由消费 `project.events` 自动完成） |

**异步事件消费**：`project.events`（失效 TM/术语本地缓存，架构设计书 §4.2）。

**错误码补充**：`COMPLIANCE_BLOCKED`（409）— 项目命中敏感策略且本地模型不可达时，`Translate`/`TranslateBatch` 直接以该错误终止，fail-closed，不降级调用云端模型（延续 OFCAT 原则，见架构设计书 §1.2 原则 3）。

---

## 4. 媒体处理服务 API 详细契约

> 以下 6 个服务统一约定：**无独立数据库**，无对外 REST（不经 Envoy Gateway 暴露），仅作为 Kafka 消费者+file-service/task-service 内部 API 调用方运行（架构设计书 §4.1 说明）。下表中的"REST"均为**内部管理端点**，供健康检查/排障使用，不代表业务主链路。

### 4.1 ingestion-service

**职责**：统一接收各媒体类型原始文件（经由消费 `file.events` 中的 `file.uploaded` 事件触发，而非直接暴露上传接口——上传本身由 file-service 承接），识别媒体类型并拆解为处理任务。

| 消费 Topic | event_type | 处理逻辑 |
|---|---|---|
| `file.events` | `file.uploaded`（且 `purpose=task_source`） | 探测文件真实类型（非仅信任扩展名，用 `file` magic number/ffprobe/PyMuPDF 判定），视频→ffprobe 探测音轨/内嵌字幕轨，PDF→判定文字层 vs 扫描件（是否需要 OCR），Office→判定具体格式与版本 |
| `task.events` | `task.created` | 触发对应 `task_media_items` 子任务的创建，并产出首批 `task.media.*.requested` 事件 |

**产出事件**：按探测结果产出 `task.media.asr.requested` / `task.media.ocr.requested` / `task.media.office.requested` 之一或组合（如视频需要 ASR 又需要 OCR 内嵌字幕轨时并行产出两条）。

**内部管理端点**：`GET /internal/health`、`GET /internal/metrics`（Prometheus scrape）。

---

### 4.2 asr-service

**职责**：语音转写（faster-whisper），输出带时间戳的转写文本。

**消费**：`task.media.asr.requested`
```json
{
  "event_id": "evt_...", "schema_version": 1,
  "task_id": "tsk_01HZX...", "source_file_id": "file_01HZX...",
  "source_lang": "ja", "model_hint": "medium"
}
```

**产出**：`task.media.asr.completed`
```json
{
  "event_id": "evt_...", "schema_version": 1,
  "task_id": "tsk_01HZX...", "status": "completed",
  "transcript_file_id": "file_02HZX...",
  "segments_count": 214,
  "metrics": { "audio_duration_seconds": 812, "process_seconds": 96, "model": "faster-whisper-medium" }
}
```
失败时 `status="failed"`，附 `error_code`/`error_message`，进入 §6.4 重试链路。

**幂等设计**：消费者在处理前以 `event_id` 查询 Valkey `dedup:{event_id}`（TTL 24h），命中则跳过重复推理（faster-whisper 单次推理成本高，双重防线尤为必要，见架构设计书 §6.7）；同时转写结果写入 file-service 时使用 `Idempotency-Key = event_id`，即使去重表失效也不会产生重复文件。

**内部同步调用**：处理完成后调用 `task-service` `/internal/v1/tasks/{id}/stage-progress` 上报（见 §3.4），而非仅依赖 Kafka 事件——双通道保证 task-service 状态及时更新（Kafka 事件供其他消费者广播，REST 上报供 task-service 自身状态机同步推进，二者携带同一 `event_id` 幂等）。

---

### 4.3 ocr-service

**职责**：图片/PDF 扫描件/视频帧/GIF-WebP 帧的文字检测识别（PaddleOCR 主，Tesseract 备选）。

**消费**：`task.media.ocr.requested`
```json
{
  "event_id": "evt_...", "schema_version": 1,
  "task_id": "tsk_01HZX...", "source_file_id": "file_01HZX...",
  "source_lang": "ja", "engine_hint": "paddleocr", "frame_sampling": { "fps": 1 }
}
```
`frame_sampling` 仅视频/GIF/WebP 场景需要（抽帧密度）。

**产出**：`task.media.ocr.completed`，`result` 含每个检测框的 `bbox/text/confidence/frame_index(视频场景)`，落盘为结构化 JSON 文件（file-service 存储），不直接塞进 Kafka 消息体（避免超大消息，Kafka 消息体建议 < 1MB，架构设计书未明确该约束但为通用最佳实践，此处补充说明）。

---

### 4.4 subtitle-service

**职责**：字幕格式解析生成（srt/vtt/ass）、时间轴对齐、字幕分段与翻译单元切分。

**消费**：`task.media.asr.completed`（触发字幕分段）、`task.media.subtitle.requested`（显式请求，如用户上传已有字幕文件仅需格式转换场景）

**处理流程**：解析 ASR 转写结果（含词级时间戳）→ 按标点/停顿规则切分为字幕分段（遵循单行字数上限、显示时长下限等字幕行业惯例约束）→ gRPC 同步调用 `translation-core.TranslateBatch`（§3.10）逐段翻译 → 产出目标语言字幕文件。

**产出**：`task.media.subtitle.completed`
```json
{
  "event_id": "evt_...", "schema_version": 1,
  "task_id": "tsk_01HZX...", "status": "completed",
  "subtitle_file_id": "file_03HZX...", "format": "srt", "segments_count": 198
}
```

---

### 4.5 office-converter-service

**职责**：Office 全家桶（docx/xlsx/pptx + ODF）结构化文本抽取与译文回填、老格式转换（LibreOffice Headless）。

**消费**：`task.media.office.requested`
```json
{
  "event_id": "evt_...", "schema_version": 1,
  "task_id": "tsk_01HZX...", "source_file_id": "file_01HZX...",
  "office_format": "docx", "source_lang": "ja", "target_lang": "zh-CN"
}
```

**处理流程**：老格式（.doc/.xls/.ppt）先经 LibreOffice Headless 转换为标准 OOXML；结构化库（python-docx/openpyxl/python-pptx/odfpy）抽取段落/单元格/文本框文本单元 → gRPC 同步调用 `translation-core.TranslateBatch` → 按原结构回填 → 产出译文文档。

**产出**：`task.media.office.completed`，含 `output_file_id`。

**失败重试策略**：LibreOffice Headless 进程超时（默认 120s）视为失败，走 §6.4 标准重试链路；进程池设置最大并发实例数（按节点 CPU 核数配置），超出排队而非无限拉起新进程（对应技术选型书 R-09）。

---

### 4.6 render-writer-service

**职责**："渲染回写"：把 translation-core 产出的译文，结合原始版面/时间轴信息，写回为原格式最终产物（字幕烧录、PDF 版面重排、Office 回填复用 office-converter-service 产出、GIF/WebP 重编码、预留 TTS 配音钩子）。

**消费**：`task.media.render.requested`
```json
{
  "event_id": "evt_...", "schema_version": 1,
  "task_id": "tsk_01HZX...", "source_file_id": "file_01HZX...",
  "render_kind": "subtitle_burn_in",
  "subtitle_file_id": "file_03HZX...",
  "output_format": "mp4"
}
```
`render_kind ∈ {subtitle_burn_in, pdf_relayout, office_passthrough, gif_reencode, webp_reencode, dub(预留未实现)}`。

**产出**：`task.media.render.completed`
```json
{
  "event_id": "evt_...", "schema_version": 1,
  "task_id": "tsk_01HZX...", "status": "completed",
  "output_file_id": "file_04HZX...", "output_format": "mp4"
}
```
该事件是任务整体完成判定的关键信号之一，task-service 消费后综合各阶段状态判定任务是否 `completed`/`partially_failed`。

**`dub`（TTS 配音）**：MVP 不实现，`render_kind=dub` 当前直接返回 `status=failed, error_code=NOT_IMPLEMENTED`，接口预留，符合技术选型书 ADR「MVP 只留接口钩子」。

---

## 5. gRPC Protobuf 版本化组织约定

```
proto/
  cats/
    auth/v1/auth.proto        (package cats.auth.v1)
    project/v1/project.proto  (package cats.project.v1)
    task/v1/task.proto        (package cats.task.v1)
    translation/v1/translation.proto (package cats.translation.v1)
```
- 统一存放于独立的 `cats-proto` 仓库（或 Monorepo 内 `proto/` 目录），各服务通过 `buf generate` 生成各自语言的 stub，纳入 CI 校验（`buf lint` + `buf breaking` 对比上一个 Git Tag，破坏性变更阻断合并）。
- TypeScript 类型（供 Next.js 控制台/Tauri 客户端）从同一 Protobuf/OpenAPI 源生成，保证前后端契约一致（技术选型书 ADR-15 已提及）。

---

## 6. 端到端示例流程："用户上传一段视频请求翻译"

以下按调用顺序列出每一步的服务、动作、同步/异步方式，以及 Outbox/CDC 的参与节点。

| # | 发起方 → 接收方 | 方式 | 动作 | Outbox/CDC 参与 |
|---|---|---|---|---|
| 1 | Tauri 客户端 → Envoy Gateway | 同步 REST | `POST /v1/auth/login`（若 Token 已过期，先刷新） | — |
| 2 | Envoy Gateway → auth-service | 同步 gRPC/REST | JWT 签发返回客户端 | — |
| 3 | Tauri 客户端 → Envoy Gateway → file-service | 同步 REST | `POST /v1/files/presign-upload` 申请预签名 URL | — |
| 4 | Tauri 客户端 → MinIO（对象存储） | 同步 HTTP PUT（直传，不经过应用层） | 上传视频文件二进制 | — |
| 5 | Tauri 客户端 → file-service | 同步 REST | `POST /v1/files`（确认上传完成，登记元数据），file-service 在**同一事务**内写 `files` 表 + `file_events_outbox` 表（`event_type=file.uploaded`） | ✅ Outbox 写入（file_db） |
| 6 | Debezium → Kafka | 异步 CDC | 捕获 `file_events_outbox` INSERT，发布到 `file.events` Topic | ✅ CDC 转发 |
| 7 | Tauri 客户端 → Envoy Gateway → task-service | 同步 REST | `POST /v1/tasks`（`media_type=video`，携带 `source_file_id`），task-service 同一事务写 `tasks` 表 + `task_events_outbox` 表（`event_type=task.created`），返回 `task_id` 给客户端 | ✅ Outbox 写入（task_db） |
| 8 | Debezium → Kafka | 异步 CDC | 捕获并发布 `task.events`（`task.created`） | ✅ CDC 转发 |
| 9 | Kafka → ingestion-service | 异步消费（`file.events`+`task.events`） | 探测媒体类型（ffprobe：有音轨+无内嵌字幕轨→需要 ASR），写回 `task_media_items` 子任务规划（经 task-service 内部 API，同一事务再写一条 Outbox 记录） | ✅ Outbox 写入（task_db，经 task-service API 间接完成） |
| 10 | Debezium → Kafka | 异步 CDC | 发布 `task.media.asr.requested` | ✅ CDC 转发 |
| 11 | Kafka → asr-service | 异步消费 | faster-whisper 转写，产出带时间戳文本，调用 file-service `POST /v1/files` 落盘（同步 REST，幂等键=event_id） | — |
| 12 | asr-service → task-service | 同步 REST | `POST /internal/v1/tasks/{id}/stage-progress`（`stage=asr,status=completed`），task-service 同事务写 Outbox | ✅ Outbox 写入（task_db） |
| 13 | Debezium → Kafka | 异步 CDC | 发布 `task.media.asr.completed` | ✅ CDC 转发 |
| 14 | Kafka → subtitle-service | 异步消费 | 解析转写结果切分字幕分段 | — |
| 15 | subtitle-service → translation-core | **同步 gRPC**（`TranslateBatch`） | 逐段翻译（内部走 TM 匹配→术语注入→模型翻译→QA），translation-core 同步 gRPC 调用 project-service `SearchTM`/`GetGlossary` | — |
| 16 | subtitle-service → file-service | 同步 REST | 落盘生成的目标语言字幕文件（srt） | — |
| 17 | subtitle-service → task-service | 同步 REST | 上报 `stage=subtitle,status=completed`，写 Outbox | ✅ Outbox 写入（task_db） |
| 18 | Debezium → Kafka | 异步 CDC | 发布 `task.media.subtitle.completed`，并由 task-service 状态机判定后续触发 `task.media.render.requested`（`render_kind=subtitle_burn_in`） | ✅ CDC 转发 |
| 19 | Kafka → render-writer-service | 异步消费 | ffmpeg 字幕烧录，产出最终 mp4 | — |
| 20 | render-writer-service → file-service | 同步 REST | 落盘最终产物文件 | — |
| 21 | render-writer-service → task-service | 同步 REST | 上报 `stage=render,status=completed`，task-service 综合各阶段状态判定整体任务 `completed`，写 Outbox（`event_type=task.completed`） | ✅ Outbox 写入（task_db） |
| 22 | Debezium → Kafka | 异步 CDC | 发布 `task.events`（`task.completed`） | ✅ CDC 转发 |
| 23 | Kafka → notification-service | 异步消费 | 生成通知，经 WebSocket `/v1/ws/notify` 推送给在线客户端；若客户端离线，落 `notification_db` 站内信 + 可选邮件 | — |
| 24 | Kafka → report-service / audit-service | 异步消费（各自独立 Consumer Group） | report-service 累加用量统计；audit-service 消费对应 `audit.events`（各步骤同步产出的审计事件）落审计日志 | — |
| 25 | Tauri 客户端 → task-service | 同步 REST/SSE | `GET /v1/tasks/{id}` 或已建立的 `GET /v1/tasks/{id}/events` SSE 连接，实时看到状态变为 `completed`，随后 `GET /v1/files/{output_file_id}/download` 下载成品 | — |

**关键点**：第 5、7、9、12(经12写Outbox)、17、21 步是"业务写 + Outbox 写"同事务发生的节点，Debezium 只负责**转发**（第 6/8/10/13/18/22 步），任何一步 Kafka 短暂不可用都不会丢事件（业务数据已落 PostgreSQL，恢复后 Debezium 补齐转发，架构设计书 §7.1）。第 15 步是全链路中唯一的"服务间同步 gRPC 长耗时调用"，因涉及流式翻译需要实时反馈，其余跨服务协同均走异步 Kafka + REST 状态上报的组合，避免长链路同步阻塞。
