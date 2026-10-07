# task-service

> CATs 任务调度服务 (per ULYS-151 切片 B-2 — task-service 业务 + SSE 进度推送)

| 项目 | 内容 |
|---|---|
| Crate 名 | `task-service` |
| 阶段 | MVP (M1-Sprint 2 切片 B-2) |
| 默认端口 | 8084 (由 env `BIND_ADDR` 覆盖) |
| 数据边界 | `task_db` (第 8 个逻辑库, per 架构书 §5.1) |
| 镜像 | `harbor.cats.internal/cats/task-service:0.1.0` |
| 基线 | `agent/minimaxm3/d11b0bce579f` (HEAD) |
| 子 issue | ULYS-151 (stage 2 of ULYS-125) |
| 切片 | [doc/05-其他/MVP商业版/_slice_b2_task.md](../../doc/05-其他/MVP商业版/_slice_b2_task.md) |

## 概述

翻译任务生命周期 (创建 / 列表 / 详情 / 状态变更 / SSE 进度推送 / 内部上报) +
进程内事件总线 (per ULYS-45 子任务 A, 切片 B-2 复用 + PATCH status 联动 publish 终态)

引用: [CATs_微服务架构设计书_v1.0 §4.1](../../doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md) (16 服务清单)
引用: [CATs_接口设计书_v2.0 §3.4](../../doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md) (task-service REST + SSE 契约)
引用: ULYS-45 子任务 A (af8abb6) SSE 既有实现 — `GET /v1/tasks/{id}/events` 与 `POST /internal/v1/tasks/{id}/stage-progress`

## API 端点 (per 切片 B-2)

| Method | Path | 说明 | RBAC 资源/操作 |
|---|---|---|---|
| GET | `/healthz` | 存活/就绪探针, 返回 `{status, name, version}` | — |
| POST | `/v1/tasks` | 创建翻译任务, 返回 201 + TaskView | Task / Create |
| GET | `/v1/tasks` | 列出任务 (分页 + `project_id` / `status` 过滤) | Task / Read |
| GET | `/v1/tasks/{id}` | 获取任务详情 | Task / Read |
| PATCH | `/v1/tasks/{id}/status` | 状态变更 (`pending → running → completed/failed/cancelled`); 终态自动 publish 到 SSE | Task / Update |
| GET | `/v1/tasks/{id}/events` | **SSE** 进度推送 (`Content-Type: text/event-stream`) | Task / Read |
| POST | `/internal/v1/tasks/{id}/stage-progress` | 内部上报 (per ULYS-45 既有实现) | Task / Create |

### RBAC 简化模式 (M1)

`Authorization: Bearer cats-role:<Role1,Role2,...>` — 例如:
- `Bearer cats-role:User` — 业务用户, 可读自己任务
- `Bearer cats-role:Sponsor` — 全权 (1 人公司 Ulysses 兼, 0 代签)
- `Bearer cats-role:QualityLead` — QA Lead, 可读所有 (评审类)

生产路径 JWT 校验 (`auth-service` 签发的 access_token) 留 Sprint 2 落地.

### 错误响应 (per 错误码表 v1.0.1 §3)

```json
{
  "error": "task_not_found",
  "message": "task does not exist",
  "detail": "可选: 上下文"
}
```

| HTTP | error 枚举 |
|---|---|
| 400 | `invalid_request` |
| 401 | `missing_authorization` |
| 403 | `operation_not_permitted` |
| 404 | `task_not_found` / `resource_not_found` |
| 409 | `invalid_state_transition` (终态后不可再更新) |
| 500 | `server_error` |

### SSE 帧格式 (per WHATWG HTML §9.3)

```text
event: stage_progress\n
id: evt_42\n
data: {"event":"stage_progress","stage":"translation",...}\n
\n
```

事件类型:
- `event: stage_progress` — 阶段进度 (来自 `/internal/.../stage-progress` 上报)
- `event: task_terminated` — 任务终态 (来自 PATCH status 进入 completed/failed/cancelled)
- `event: heartbeat` — 15s 心跳 (per SSE 代理最佳实践, 防中间代理判超时)

## 数据模型 (per migrations/20260920_0001_init.sql)

| 列 | 类型 | 说明 |
|---|---|---|
| `id` | UUID PK | `gen_random_uuid()` (per 切片 B-2 schema) |
| `project_id` | UUID NOT NULL | 关联 project-service; **不**做 FK (per 架构书 §1.2 跨服务边界) |
| `task_type` | TEXT NOT NULL CHECK | `translate` / `review` / `export` |
| `status` | TEXT NOT NULL CHECK DEFAULT 'pending' | `pending` / `running` / `completed` / `failed` / `cancelled` |
| `input_payload` | JSONB | 任务输入 (per 接口设计书 §3.4 业务 payload) |
| `output_payload` | JSONB | 任务输出 (PATCH status 时可填) |
| `progress` | INTEGER CHECK 0..100 | 进度百分比 |
| `error_message` | TEXT | 失败原因 (status='failed' 时) |
| `created_at` / `updated_at` | TIMESTAMPTZ | 自动维护 (trigger) |
| `started_at` / `completed_at` | TIMESTAMPTZ | 状态机推进时间戳 |

索引:
- `idx_tasks_project_id` — 按项目过滤
- `idx_tasks_status` — 按状态过滤
- `idx_tasks_project_id_status` — 复合 (per "某项目下 pending 任务" 常见查询)
- `idx_tasks_created_at` — DESC 排序 (per 列表分页)
- `idx_tasks_updated_at` — DESC (per 监控)

## 上下游服务

- **上游 (被调用)**: 客户端 / BFF (`cats-bff`)
- **下游 (主动调用)**: `cats-proto` (gRPC 契约) + `cats-rbac` (路径/方法鉴权) + 数据库 (PostgreSQL 18.6)

## 状态机 (per 接口设计书 §3.4)

```text
pending → running → completed
                  → failed
                  → cancelled
```

- 终态后再次 PATCH → 409 `invalid_state_transition`
- 进入终态 → publish 到 SSE 事件总线 (per 切片 B-2 新增: PATCH status 联动)
- `started_at` 在首次进入 running 时填; `completed_at` 在终态时填

## 本地运行

```powershell
# 编译
cargo build -p task-service

# 运行 (本地 task_db)
$env:DATABASE_URL = "postgres://cats:secret@localhost:5432/task_db"
$env:BIND_ADDR = "0.0.0.0:8084"
$env:RUST_LOG = "info,task_service=debug"

# 可选: 本地开发关闭 RBAC (生产严禁)
# $env:RBAC_DISABLED = "1"

cargo run -p task-service

# 健康检查
curl http://127.0.0.1:8084/healthz
```

## 测试

```powershell
# 单元测试 (lib, 不连 DB)
cargo test -p task-service --lib

# 集成测试 (tests/integration.rs, 22 case 覆盖 SSE 帧 + RBAC + 业务模型)
cargo test -p task-service --test integration

# 全部测试 (含 smoke)
cargo test -p task-service

# 编译检查
cargo check -p task-service
```

实测结果 (2026-09-21): **21 lib + 22 integration + 2 smoke = 45 tests, all pass**

## 容器化

```bash
docker build -f deploy/docker/Dockerfile.rust --build-arg CRATE_NAME=task-service -t task-service:0.1.0 .
```

## Helm 部署

```bash
helm lint deploy/helm/task-service
helm template deploy/helm/task-service
```

## 引用基线文档

- [CATs_微服务架构设计书_v1.0 §4.1](../../doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md)
- [CATs_接口设计书_v2.0 §3.4](../../doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md)
- [CATs_数据库设计书_v2.0 §6 (task_db 章节, 待补)](../../doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md)
- [CATs_技术基线_v1.0 §1](../../doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md)
- [CATs_Rust技术选型书_v1.0](../../doc/02-基础设计/技术选型/CATs_Rust技术选型书_v1.0.md)
- [CATs_权限矩阵_v1.0 §3](../../doc/05-其他/管理/CATs_权限矩阵_v1.0.md) (RBAC 16 域 × 7 操作)
- [CATs_错误码表_v1.0.1 §3-§4](../../doc/05-其他/管理/CATs_错误码表_v1.0.1.md) (error enum)

## 切片交付清单 (per 切片 B-2 §"交付清单")

1. ✅ `crates/task-service/migrations/20260920_0001_init.sql` — tasks 表 DDL
2. ✅ `crates/task-service/src/db.rs` — PgPool + CRUD (create / find_by_id / list / update_status)
3. ✅ `crates/task-service/src/models.rs` — DTO + 业务枚举 + SSE event (TaskEvent / SseTaskStatus)
4. ✅ `crates/task-service/src/handlers.rs` — 4 REST + 1 SSE + 1 internal + healthz
5. ✅ `crates/task-service/src/events.rs` — broadcast::channel 封装 (per ULYS-45 + 切片 B-2 PATCH 联动)
6. ✅ `crates/task-service/src/lib.rs` + `src/main.rs` — 启动
7. ✅ `crates/task-service/src/rbac.rs` — cats-rbac 集成 (AuthContext + enforce inline)
8. ✅ `crates/task-service/tests/integration.rs` — 22 integration tests (含 SSE 帧解析器复用)
9. ✅ `crates/task-service/tests/smoke.rs` — 现有 cats-mock smoke (保留)
10. ✅ `crates/task-service/README.md` — 本文件

## 验收标准 (per 切片 B-2)

- ✅ 5 endpoint 全部实现 (POST / GET list / GET detail / PATCH status / GET SSE events)
- ✅ SSE 用 actix-web `streaming` + async-stream + tokio::sync::broadcast (per ULYS-45 既有)
- ✅ RBAC 中间件挂在每个 endpoint (`rbac::enforce` inline 检查)
- ✅ 集成测试 ≥3 case (实测 22 case, 大幅超出)
- ✅ `cargo check -p task-service` 通过

## 诚实披露

- **rustc 1.98 metadata bug 仍在** — 本切片单 crate check 通过; 全 workspace release binary 编译仍卡 (per Sprint 1 §6.x 诚实披露)
- **真实 task_db 未连**: 集成测试用 `connect_lazy` 占位; Sprint 2 接入 sqlx::test attribute 时填实真 DB 路径
- **M1 RBAC 简化模式**: `cats-role:` token 字符串替代 JWT 校验; Sprint 2 接 auth-service JWKS 时落地
- **events 事件总线进程内**: M1 阶段无 Kafka 依赖; Sprint 2 引入 Kafka `task.events` topic
- **slice B-2 中 SSE `SseTaskStatus` 8 态** 与 DB `TaskStatus` 5 态解耦 — SSE 协议沿用 ULYS-45 既有 8 态以保证向后兼容

## 作者署名

- 实施: Mavis (接手 agent per DEC-008, per 切片 B-2 顶部作者栏)
- 架构: 架构师 Lead (per 切片 spec)