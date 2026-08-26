# CATs 模块设计书

**系统名称:** CATs — 全媒体 AI 辅助翻译 SaaS 平台

---

## 文档管理信息

| 项目 | 内容 |
|---|---|
| 文档编号 | CATs-DD-MOD-002 |
| 文档名 | 模块设计书（微服务内部分层 / 客户端 / 控制台 / 媒体管线模块设计） |
| 版本 | 第 2.1 版 |
| 创建日 | 2026-08-26 |
| 作者 | 架构师 + Rust Lead + DBA（worker 代签 per DEC-008） |
| 状态 | 评审前草稿 |
| 密级 | 仅社内 |
| 上游文档 | [CATs 微服务架构设计书 v1.0](../../02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md)、[CATs 技术选型书 v2.0](../../02-基础设计/技术选型/CATs_技术选型书_v2.0.md)、[CATs 接口设计书 v2.0](../接口设计/CATs_接口设计书_v2.0.md)、[CATs 数据库设计书 v2.0](../数据库设计/CATs_数据库设计书_v2.0.md)、[CATs_技术基线 v1.0](../../02-基础设计/技术选型/CATs_技术基线_v1.0.md)（**§1 PostgreSQL 18.6 + pgvector 0.8.6 + Rust 1.98.0**）、[OFCAT 模块设计书 v1.0（历史/旧架构参考，格式沿用）](./OFCAT_模块设计书_v1.0.md) |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|---|---|---|---|
| 1.0 | 2026-06-25 | 架构师 | （OFCAT）浏览器扩展 + 本地引擎模块划分，见历史文档 |
| 2.0 | 2026-08-18 | 架构师 | 全面重做：微服务内部分层、Tauri 客户端、Next.js 控制台、媒体管线各阶段、公共基础库设计，承接《CATs 微服务架构设计书 v1.0》§21 |
| **2.1** | **2026-08-26** | **架构师 + Rust Lead + DBA** | **基线升级：PostgreSQL / Rust 引用统一引用 CATs_技术基线_v1.0 §1（PostgreSQL 18.6 + Rust 1.98.0 + pgvector 0.8.6）** |

### 审批栏

| 角色 | 姓名 | 审批日 | 签字 |
|---|---|---|---|
| 起草 | 架构师 + Rust Lead + DBA | 2026-08-26 |  |
| 评审 |  |  |  |
| 批准 |  |  |  |

---

## 0. 阅读指南

本书是《CATs 微服务架构设计书 v1.0》§21 承诺的模块详细设计文档，覆盖：①后端各微服务内部代码分层与目录/crate 结构，②Tauri 客户端内部模块，③Next.js Web 控制台目录结构，④媒体处理管线各阶段的输入输出契约与失败重试/幂等设计，⑤跨服务共享的公共基础库/SDK。接口契约见《CATs 接口设计书 v2.0》，数据表结构见《CATs 数据库设计书 v2.0》。涉及技术栈版本（PostgreSQL / pgvector / Rust toolchain 等）一律以 [CATs_技术基线_v1.0 §1](../../02-基础设计/技术选型/CATs_技术基线_v1.0.md) 为准。

---

## 1. 后端微服务内部分层通用规范

### 1.1 分层原则（不分实现语言，统一约束）

```
API 层（REST Handler / gRPC Service Impl / Kafka Consumer Handler）
   │  仅做：请求解析校验、鉴权上下文提取、调用领域层、组装响应/错误信封
   ▼
领域层（Domain / Service）
   │  业务规则、状态机迁移校验、跨聚合编排（如 task 状态机推进逻辑）
   │  不感知 HTTP/gRPC/Kafka 细节，不直接拼 SQL
   ▼
仓储层（Repository）
   │  唯一允许写 SQL/调用 sqlx-query 的层，向上暴露领域对象，不暴露行级 DB Row 结构
   ▼
基础设施层（Infra）
      DB 连接池、Kafka Producer/Consumer 客户端、外部 gRPC Client Stub、对象存储 SDK
```

- 领域层不得直接依赖任何 Web 框架类型（如 Rust 的 `axum::Json`、Python 的 `fastapi.Request`），保证领域逻辑可脱离框架单元测试。
- 仓储层方法签名以领域对象为出入参（如 `fn find_task(id: TaskId) -> Result<Task>`），不返回裸 `sqlx::Row`/ORM Model，避免领域层反向感知持久化细节。
- Outbox 写入固定放在仓储层的同一事务方法内（如 `TaskRepository::create_with_outbox_event(...)`），杜绝业务写与 Outbox 写分散在两次独立调用中破坏事务边界（架构设计书 §7.3 的事务流程必须在代码层面被这一模式强制收口，而非依赖开发者每次手写 BEGIN/COMMIT）。

### 1.2 Rust 服务的 crate 结构（适用于 render-writer-service 等对性能敏感的服务，以及未来考虑用 Rust 重写的核心服务）

以 `render-writer-service` 为例：
```
render-writer-service/
├── Cargo.toml                     # workspace 根
├── crates/
│   ├── api/                       # API 层：Kafka Consumer Handler、内部 gRPC/REST Client 调用封装
│   │   └── src/lib.rs
│   ├── domain/                    # 领域层：render_kind 分发逻辑、渲染策略接口 trait
│   │   └── src/{model.rs, service.rs, ports.rs}   # ports.rs 定义 trait，供 infra 实现（依赖倒置）
│   ├── infra/                     # 基础设施层：ffmpeg 子进程封装、file-service gRPC/REST Client、Kafka Producer/Consumer
│   │   └── src/{ffmpeg_runner.rs, file_client.rs, kafka.rs}
│   └── shared/                    # 引用公共基础库 cats-sdk-rs（见 §5）
└── src/main.rs                    # 组装依赖注入、启动 Consumer Loop + 健康检查 HTTP Server
```
`domain::ports` 定义 `trait Renderer { fn render(&self, job: RenderJob) -> Result<RenderOutput>; }`，`infra` 层提供 `SubtitleBurnInRenderer`/`PdfRelayoutRenderer`/`GifReencodeRenderer` 等具体实现，`main.rs` 按 `render_kind` 注入对应实现——这是媒体处理服务"可插拔处理器"设计（架构设计书 §2 拓扑图标注"媒体处理域"为可插拔）在代码层面的落地方式。

### 1.3 Python 服务的目录结构（适用于 translation-core、asr-service、ocr-service、subtitle-service、office-converter-service）

以 `translation-core` 为例（FastAPI）：
```
translation-core/
├── pyproject.toml
├── app/
│   ├── api/                       # API 层
│   │   ├── grpc/translation_service.py   # gRPC Servicer 实现，仅做 protobuf<->领域对象转换
│   │   └── http/internal_routes.py       # 内部管理端点（缓存统计等）
│   ├── domain/                    # 领域层：LangGraph 编排图定义（沿用 OFCAT 编排逻辑迁移）
│   │   ├── pipeline.py            # TM匹配→术语注入→标签保护→模型翻译→术语校验→QA 的 LangGraph StateGraph
│   │   ├── tm_matcher.py
│   │   ├── term_injector.py
│   │   ├── tag_protector.py
│   │   └── qa_checker.py
│   ├── repository/                # 仓储层：project_db 的 TM/术语查询（本服务只读，project-service 是写权威）
│   │   └── tm_repository.py
│   ├── infra/
│   │   ├── db.py                  # SQLAlchemy engine/session
│   │   ├── project_client.py      # gRPC 调用 project-service（架构设计书 §4.2 同步 gRPC）
│   │   ├── model_gateway.py       # LiteLLM AI 网关封装（沿用 OFCAT）
│   │   └── kafka_consumer.py      # 消费 project.events 失效本地缓存
│   └── shared/                    # 引用公共基础库 cats-sdk-py（见 §5）
└── migrations/                    # Alembic
```
`domain/pipeline.py` 是 OFCAT LangGraph 编排逻辑的直接迁移承接点（架构设计书 §18.4 步骤 S2），节点函数签名保持稳定，仅将原本直连 SQLite 的部分替换为调用 `repository/tm_repository.py`。

### 1.4 Worker/无状态处理器服务的目录结构（asr/ocr/office-converter，Python 实现示例）

```
asr-service/
├── app/
│   ├── consumer/                  # API 层：Kafka Consumer 入口，消费 task.media.asr.requested
│   │   └── asr_requested_handler.py
│   ├── domain/
│   │   ├── transcribe.py          # faster-whisper 推理封装（领域逻辑：模型选择/分段策略）
│   │   └── idempotency.py         # event_id 去重判定（Valkey SETNX + 业务表兜底，见 §4.2）
│   ├── infra/
│   │   ├── whisper_runtime.py     # CTranslate2/faster-whisper 底层调用
│   │   ├── file_client.py         # 调用 file-service 存取文件
│   │   ├── task_client.py         # 调用 task-service stage-progress 上报
│   │   └── kafka.py
│   └── shared/                    # cats-sdk-py
└── Dockerfile                     # 基于 GPU 基础镜像（cats-3rdparty 缓存），媒体处理服务统一约定见架构设计书 §12.1
```

---

## 2. Rust 原生前端客户端（Tauri）模块设计

### 2.1 整体分层

```
┌─────────────────────────────────────────────────────┐
│ WebView 前端（Svelte 5 + TypeScript）                  │
│  - 翻译工作台 UI（对照编辑器/字幕时间轴/文档预览）           │
│  - 通过 Tauri `invoke()` / `Channel` 调用 Rust 核心命令  │
└───────────────────────┬───────────────────────────────┘
                         │ Tauri IPC（invoke/emit，进程内，非网络调用）
┌───────────────────────▼───────────────────────────────┐
│ Rust 核心层（tauri::App 主进程）                          │
│  ┌─────────────┐ ┌──────────────┐ ┌──────────────────┐ │
│  │ commands/   │ │ api_client/   │ │ local_cache/      │ │
│  │ (IPC 命令入口)│ │ (REST/gRPC/  │ │ (SQLite 本地缓存/  │ │
│  │              │ │  WS 客户端)   │ │  离线队列)          │ │
│  └─────────────┘ └──────────────┘ └──────────────────┘ │
│  ┌─────────────┐ ┌──────────────┐ ┌──────────────────┐ │
│  │ system/     │ │ auth/         │ │ updater/          │ │
│  │ (托盘/文件监│ │ (Token 存储/  │ │ (签名校验自动更新)  │ │
│  │  控/通知)    │ │  刷新)        │ │                   │ │
│  └─────────────┘ └──────────────┘ └──────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

### 2.2 Rust 核心层职责细分

| 模块 | 职责 |
|---|---|
| `commands/` | 定义所有 `#[tauri::command]`，是 WebView 唯一可调用的入口，每个 command 做参数校验后转发给对应领域模块，不含业务逻辑本身 |
| `api_client/` | 封装对后端微服务的 REST（`reqwest`）/gRPC（`tonic`）/WebSocket（`tokio-tungstenite`）调用；统一注入 `Authorization` Header、`traceparent`（OpenTelemetry，架构设计书 §13.2 要求客户端也参与统一 Trace）、超时与重试策略 |
| `local_cache/` | 本地 SQLite（`rusqlite`/`sqlx-sqlite`）存储：最近查看的项目/任务列表缓存、术语库本地只读快照（供离线时对照编辑器仍可查词）、**离线操作队列**（见 §2.3） |
| `auth/` | Token 安全存储（依赖操作系统级密钥库：Windows Credential Manager / macOS Keychain / Linux Secret Service，通过 `keyring` crate），access_token 过期自动用 refresh_token 刷新 |
| `system/` | 系统托盘图标与菜单、本地文件系统监控（如"监控文件夹自动导入待翻译文档"功能）、原生桌面通知 |
| `updater/` | Tauri 内置 Updater 插件封装，校验发布签名后拉取更新包，对应技术选型书 ADR-14"客户端安全"要求 |

### 2.3 本地缓存 / 离线队列设计

**设计目标**：客户端断网（局域网内网络抖动/后端集群维护窗口）时，用户仍可继续编辑已加载的翻译任务，编辑操作先落本地队列，恢复联网后自动同步，不丢失用户输入。

```sql
-- 客户端本地 SQLite（非 PostgreSQL，与服务端数据库设计书无关，仅存于用户本机）
CREATE TABLE offline_queue (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    op_type         TEXT NOT NULL,          -- 'segment_edit' / 'term_add' / ...
    payload_json    TEXT NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,   -- 客户端生成 UUID，与后端接口的 Idempotency-Key 语义一致（接口设计书 §1.1）
    created_at      TEXT NOT NULL,
    sync_status     TEXT NOT NULL DEFAULT 'pending' CHECK (sync_status IN ('pending','syncing','synced','failed')),
    retry_count     INTEGER NOT NULL DEFAULT 0
);
```

**同步策略**：
1. 网络恢复检测（定时心跳 `GET /v1/health` 或 WebSocket 重连成功事件触发）后，按 `created_at` 顺序逐条重放 `offline_queue` 中 `pending`/`failed` 记录。
2. 每条记录携带其 `idempotency_key` 作为 `Idempotency-Key` 请求头，即使因网络问题重复提交也不会在服务端产生重复副作用（接口设计书 §1.1 幂等键约定）。
3. 单条重放失败（非网络原因，如 409 冲突——服务端数据已被其他端更新）标记 `failed` 并弹出冲突提示 UI，交由用户人工决策"保留本地版本/放弃本地版本/查看差异"，不做自动覆盖式合并（避免翻译内容被静默覆盖丢失）。
4. 成功同步的记录保留 7 天后本地清理（供排障回溯），非永久保留。

### 2.4 与后端 API 的通信层设计

- REST 调用统一走 `api_client::rest::RestClient`，内部用 `reqwest::Client` 单例（连接池复用），所有请求自动附加：`Authorization`、`X-Client-Version`、`traceparent`。
- 流式翻译（`translation-core.Translate`）与任务进度（`GET /v1/tasks/{id}/events` SSE）分别走 `api_client::grpc`（`tonic` 流式客户端，经 Envoy Gateway GRPCRoute）与 `api_client::sse`，二者产出统一转换为 Rust `mpsc::channel`，再经 `commands/` 用 Tauri `Channel`/`emit` 推给前端 WebView，前端以事件订阅方式消费，避免轮询。
- WebSocket（`notification-service`）常驻连接由 `system/` 模块在应用启动时建立，断线自动重连（指数退避，上限 30s），收到通知后调用 `system::notify` 弹桌面通知并 `emit` 给前端更新站内信角标。

---

## 3. Next.js 后台管理控制台模块设计

### 3.1 App Router 目录结构

```
web-console/
├── app/
│   ├── (auth)/
│   │   ├── login/page.tsx
│   │   └── oidc/callback/route.ts        # OIDC 回调 Route Handler，换取后端 JWT 后写入 Cookie
│   ├── (dashboard)/
│   │   ├── layout.tsx                    # 鉴权中间件保护的布局，含侧边导航
│   │   ├── projects/
│   │   │   ├── page.tsx                  # 项目列表（Server Component，SSR 首屏）
│   │   │   └── [projectId]/
│   │   │       ├── glossary/page.tsx     # 术语库管理
│   │   │       └── tm/page.tsx           # TM 检索/管理
│   │   ├── tasks/
│   │   │   ├── page.tsx                  # 任务列表
│   │   │   └── [taskId]/page.tsx         # 任务详情（Client Component，订阅 SSE 实时进度）
│   │   ├── org/
│   │   │   ├── members/page.tsx
│   │   │   └── billing/page.tsx
│   │   ├── reports/page.tsx
│   │   └── admin/                        # 仅 platform_admin 角色可见（中间件二次校验）
│   │       ├── audit-logs/page.tsx
│   │       └── dlq/page.tsx              # Kafka DLQ 消息查看/重放（架构设计书 §6.4 提及的管理页面）
│   └── api/                              # BFF Route Handlers（聚合层，见 §3.3）
│       └── bff/
│           ├── tasks/route.ts
│           └── projects/route.ts
├── middleware.ts                         # 鉴权中间件（见 §3.2）
├── lib/
│   ├── api-client/                       # 从 OpenAPI/Protobuf 生成的 TypeScript 类型 + fetch 封装（技术选型 ADR-15 契约共享）
│   ├── auth/session.ts                   # Cookie Session 读写（Auth.js 封装）
│   └── ws/notification-client.ts         # 复用与 Tauri 客户端相同的通知协议
└── components/                           # UI 组件库（Design Token 与 Tauri 客户端共享视觉规范，非强制共享组件代码，架构设计书 §19 风险 #9）
```

### 3.2 鉴权中间件

```ts
// middleware.ts（简化示意）
export async function middleware(req: NextRequest) {
  const session = await getSession(req);           // 读取 Cookie 中的 JWT
  if (!session && !isPublicPath(req.nextUrl.pathname)) {
    return NextResponse.redirect(new URL('/login', req.url));
  }
  if (req.nextUrl.pathname.startsWith('/admin') && !session?.roles.includes('platform_admin')) {
    return NextResponse.redirect(new URL('/403', req.url));
  }
  return NextResponse.next();
}
export const config = { matcher: ['/((?!_next|api/public).*)'] };
```
- Session 存储：Auth.js（NextAuth）JWT 策略，Cookie `httpOnly + secure + sameSite=lax`，Token 本体为 auth-service 签发的 JWT（与 Tauri 客户端使用同一套 auth-service，不做重复认证体系）。
- 角色/权限校验二次防线：中间件层做粗粒度路由保护（如 `/admin/*` 仅 `platform_admin`），页面/Route Handler 内部仍需对具体资源做细粒度校验（如"这个项目是否属于当前用户所在 org"），不能仅依赖中间件。

### 3.3 与后端 API 网关的对接方式

- **BFF 模式**：`app/api/bff/*` Route Handlers 作为服务端聚合层，Server Component 优先直接在服务端调用 Route Handler 内部逻辑（同进程函数调用，不产生额外网络跳转），Client Component 交互（如任务列表分页/筛选）则通过 `fetch('/api/bff/tasks?...')` 调用。
- **直连 vs BFF 的选择原则**：单一资源的简单读取（如项目详情）Server Component 直接 `fetch` 后端微服务 REST API（经内部集群 DNS 或专用 BFF-to-backend 出口，不经过 Envoy Gateway 面向公网的路径，减少一次网络跳转）；需要**聚合多个微服务**响应的场景（如任务详情页需要同时展示 task-service 状态 + file-service 文件信息 + report-service 相关用量）才经 BFF Route Handler 聚合，避免 Client Component 直接并发调用多个后端服务、暴露过多内部服务端点给浏览器。
- 鉴权透传：BFF Route Handler 从 Cookie 取出 JWT，转换为 `Authorization: Bearer` Header 转发给后端微服务（浏览器侧不直接持有可被 XSS 窃取的 Token，Token 只存在于 httpOnly Cookie，安全性优于客户端直存）。
- WebSocket 通知：浏览器侧直接与 `notification-service` 建立 WebSocket 连接（经 Envoy Gateway WS 升级支持），不经过 BFF 中转（长连接不适合走 Serverless/Route Handler 模式）。

---

## 4. 媒体处理管线各阶段模块设计

> 本节为架构设计书 §2.2「异步媒体处理」请求路径与接口设计书 §6 端到端示例流程的模块层落地细化，每阶段给出输入契约、输出契约、失败重试策略、幂等设计四要素。

### 4.1 ingestion（ingestion-service）

| 要素 | 设计 |
|---|---|
| 输入契约 | Kafka `file.events`(`file.uploaded`) + `task.events`(`task.created`)，见接口设计书 §4.1 |
| 输出契约 | `task.media.{asr,ocr,office}.requested` 之一或组合（按探测结果），写入 `task_media_items` 规划记录 |
| 失败重试 | 探测阶段失败（如文件损坏无法被 ffprobe/PyMuPDF 解析）**不进入标准 Kafka 重试链路**，直接标记该任务 `failed`，`error_code=UNSUPPORTED_OR_CORRUPTED_FILE`——因为这是确定性失败（重试不会改变结果），区别于下游服务的"资源暂时不可用"类瞬时失败 |
| 幂等设计 | 探测结果落库前先查 `task_media_items` 是否已存在该 `task_id` 的规划记录，存在则跳过（避免同一 `task.created` 事件因 Consumer Group Rebalance 等原因重复消费时重复规划子任务） |

### 4.2 ASR / OCR（asr-service / ocr-service）

| 要素 | 设计 |
|---|---|
| 输入契约 | `task.media.asr.requested` / `task.media.ocr.requested`（接口设计书 §4.2/§4.3 完整 schema） |
| 输出契约 | `task.media.asr.completed` / `task.media.ocr.completed`，结果落 `task_db.asr_transcripts` 表 / file-service 结构化 JSON 文件 |
| 失败重试 | 标准 Kafka Retry Topic 链路（架构设计书 §6.4）：重试 1(10s)→重试 2(1min)→重试 3(10min)→DLQ；GPU 显存不足（`CUDA_OOM`）类失败额外触发"降级到 CPU 推理"的应用层兜底（而非无限重试同一 GPU 资源竞争），降级逻辑写在 `domain/transcribe.py` 内，非 Kafka 重试机制职责 |
| 幂等设计 | `domain/idempotency.py`：处理前 `Valkey SETNX dedup:{event_id}`（TTL 24h）抢占执行权，业务表 `asr_transcripts` 以 `(media_asset_id, seq)` 唯一约束兜底（即使 Valkey 因故障丢失去重状态，重复插入也会被数据库唯一约束拒绝而非产生重复行），完全对应架构设计书 §6.7 双重防线设计 |

### 4.3 翻译（translation-core）

| 要素 | 设计 |
|---|---|
| 输入契约 | 上游服务（subtitle-service/office-converter-service）经 gRPC `TranslateBatch` 同步调用传入的分段数组，`segment_id` 由调用方生成保证顺序可回填 |
| 输出契约 | 逐段 `TranslatedSegment`（含 `tm_level`/`qa_pass`），**不落自己的独立数据库**，结果直接在同步响应中返回给调用方，调用方负责持久化（如 subtitle-service 写 `subtitle_segments` 表） |
| 失败重试 | 同步 gRPC 调用失败由**调用方**（subtitle-service 等）负责重试（指数退避，最多 3 次），translation-core 自身不维护重试队列——因为它是同步调用被调方，重试语义天然属于调用方职责 |
| 幂等设计 | 翻译计算本身是纯函数式（相同输入产出相同/确定性范围内的输出，TM 精确匹配部分严格幂等，模型生成部分允许合理的非确定性但不影响业务正确性），无需额外幂等表；`COMPLIANCE_BLOCKED` 判定基于项目当前策略实时查询，天然幂等 |

### 4.4 字幕 / 排版还原（subtitle-service / office-converter-service）

| 要素 | 设计 |
|---|---|
| 输入契约 | subtitle-service：`task.media.asr.completed` 事件；office-converter-service：`task.media.office.requested` 事件 |
| 输出契约 | `task.media.subtitle.completed` / `task.media.office.completed`，产物写 file-service，段落级明细写 `task_db.subtitle_segments` |
| 失败重试 | 标准 Kafka Retry+DLQ 链路；LibreOffice Headless 超时（office-converter-service）触发进程池强制 kill + 重启该 worker 进程（不影响其他并发转换任务），随后按重试链路重新消费 |
| 幂等设计 | 输出文件以 `Idempotency-Key = event_id` 调用 file-service `POST /v1/files`，`subtitle_segments`/结构化回填以 `(media_asset_id, seq)` / `(task_id, 文档内定位路径)` 唯一约束防重复写入 |

### 4.5 渲染写回（render-writer-service）

| 要素 | 设计 |
|---|---|
| 输入契约 | `task.media.render.requested`（`render_kind` 分发，接口设计书 §4.6） |
| 输出契约 | `task.media.render.completed`，最终产物写 file-service，`task-service` 据此判定任务整体状态 |
| 失败重试 | 标准 Kafka Retry+DLQ 链路；ffmpeg 子进程异常退出码非 0 视为失败，捕获 stderr 写入 `error_message` 供 DLQ 人工排查页面（Web 控制台 `admin/dlq`）展示 |
| 幂等设计 | `render_kind` 分发到的具体 `Renderer` 实现（§1.2 crate 结构中的 `ports::Renderer`）内部均以输出文件的确定性命名（基于 `task_id`+`stage`+`event_id` 派生）避免同一渲染任务重复执行产生的多份输出文件互相覆盖不一致；file-service 落盘同样以 `Idempotency-Key = event_id` 兜底 |

### 4.6 跨阶段共性：失败重试与幂等的统一模式总结

| 维度 | 统一约定 |
|---|---|
| 瞬时失败（资源暂不可用/网络抖动） | 走 Kafka Retry Topic 链路（架构设计书 §6.4），指数退避 3 次后 DLQ |
| 确定性失败（输入本身不合法/损坏） | 直接标记失败，不进入重试链路，避免无意义重试消耗资源 |
| 幂等主防线 | `event_id` 全局唯一 + Valkey `SETNX` 短期去重 |
| 幂等兜底防线 | 目标业务表唯一约束 + 文件落盘 `Idempotency-Key` |
| 进度上报 | 各阶段完成后统一调用 task-service `/internal/v1/tasks/{id}/stage-progress`（接口设计书 §3.4），保证 Kafka 事件与 task-service 状态机双通道一致 |

---

## 5. 公共基础库 / SDK 设计

### 5.1 是否做成内部 crate/npm 包/python package：结论

按实现语言拆分为三个内部共享库，**不做跨语言的单一超级 SDK**（避免为了"复用"引入不必要的跨语言 FFI/RPC 复杂度，符合架构设计书 §1.2 不过度设计原则——三种语言各自的生态内部复用收益远大于强行跨语言复用的成本）：

| 包 | 语言 | 发布方式 | 覆盖服务 |
|---|---|---|---|
| `cats-sdk-rs` | Rust | 内部 Cargo Registry（或 Git 依赖，视团队规模决定是否需要专用 Registry） | render-writer-service、Tauri 客户端 Rust 核心层、未来可能的 Rust 核心服务 |
| `cats-sdk-py` | Python | 内部 PyPI 镜像（复用 Harbor 或独立轻量 PyPI 代理） | translation-core、asr/ocr/office-converter-service、worker-service |
| `cats-sdk-ts` | TypeScript | 内部 npm Registry（或 Harbor 的 npm 支持） | Next.js Web 控制台、Tauri 客户端 Svelte 前端层 |

### 5.2 各 SDK 覆盖的公共能力

| 能力 | 说明 |
|---|---|
| 统一错误处理 | 实现接口设计书 §1.3/§1.4 的统一错误信封结构体/异常类型，各服务 API 层捕获领域层错误后统一转换为该结构，禁止裸抛原始异常/裸 HTTP 500 |
| 统一日志 | 结构化 JSON 日志封装（自动注入 `trace_id`/`service`/`level`，架构设计书 §13.3），屏蔽底层日志库差异（Rust `tracing`/Python `structlog`/TS `pino`） |
| OpenTelemetry 封装 | 统一初始化 OTel SDK、Trace Context 在 HTTP/gRPC Header 与 Kafka 消息 Header 间传播的封装函数（架构设计书 §13.2 的"Span Link"逻辑固化在 SDK 内，避免每个服务重复实现且容易出错） |
| Kafka Producer/Consumer 封装 | 统一的 `schema_version` 校验、Consumer Group 命名规范校验（架构设计书 §6.5）、Retry Topic/DLQ 自动路由逻辑（消费失败达到重试上限后自动发布到 `.dlq`，业务代码只需返回错误，不用手写重试计数与路由） |
| Outbox 写入助手 | 领域仓储层调用的统一 `write_with_outbox(tx, aggregate, event)` 辅助函数，强制同事务写业务表+Outbox 表（§1.1 已提及的"代码层面强制收口"） |
| 幂等去重助手 | 封装 Valkey `SETNX` + 业务表唯一约束兜底的标准双重防线模式（§4.6），供各媒体处理服务复用而非各自重复实现 |
| 鉴权上下文提取 | 从网关注入的 Header（`X-Cats-User-Id`等，接口设计书 §1.2）解析为强类型 `AuthContext`，供领域层使用，杜绝各服务各自 stringly-typed 解析 Header |

### 5.3 版本管理

三个 SDK 均语义化版本（SemVer），破坏性变更（如 Outbox 助手函数签名变更）升主版本号，各消费服务在自己的依赖清单锁定兼容版本区间，SDK 团队变更需在 CI 中跑消费方的集成测试矩阵（或至少发布 CHANGELOG 通知），避免"静默升级 SDK 导致下游服务行为变化"的隐性耦合风险。
