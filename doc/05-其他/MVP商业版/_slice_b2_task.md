# 切片 B-2: task-service 业务 + SSE 进度推送

**Slice**: B-2 — 后端核心 4 服务
**目标 crate**: `crates/task-service/`
**基线**: `agent/minimaxm3/d11b0bce579f` (HEAD)

## 任务范围

task-service 缺失业务 endpoint + SSE 进度推送：

- `POST /v1/tasks` — 创建翻译任务（dispatch 到 translation-core）
- `GET /v1/tasks` — 列出任务（分页 + 按 project_id 过滤）
- `GET /v1/tasks/{id}` — 获取任务详情
- `PATCH /v1/tasks/{id}/status` — 状态变更（pending → running → completed/failed）
- `GET /v1/tasks/{id}/events` — **SSE** 推送任务进度（per memory: ULYS-45 SSE endpoint 已 ship 模式）

## 复用代码

- `crates/user-service/src/handlers.rs` — handlers 模式
- `crates/cats-rbac/src/lib.rs` — RBAC 中间件
- `crates/translation-core/src/db.rs` — task 相关表查询
- `crates/cats-ai-gateway/src/service.rs` — 看到 AI GW 怎么暴露 SSE / 流式

## 数据模型

tasks 表：
- id UUID PK
- project_id UUID FK → projects(id)
- task_type TEXT CHECK (task_type IN ('translate','review','export'))
- status TEXT DEFAULT 'pending' CHECK (status IN ('pending','running','completed','failed','cancelled'))
- input_payload JSONB
- output_payload JSONB
- progress INT DEFAULT 0 CHECK (progress BETWEEN 0 AND 100)
- error_message TEXT
- created_at / updated_at / started_at / completed_at TIMESTAMPTZ

## SSE 实现要点（per ULYS-45 模式）

```rust
use actix_web::Responder;
use tokio::sync::broadcast;

async fn sse_events(task_id: Path<Uuid>) -> impl Responder {
    let rx = state.subscribe_task_events(task_id.into_inner());
    sse::Sse::from_stream(ReceiverStream::new(rx).map(|ev| {
        Ok::<_, Infallible>(sse::Event::Data(sse::Data::new(ev.to_json())))
    }))
}
```

事件类型：task.progress / task.status_changed / task.completed / task.failed

## 交付清单

1. `crates/task-service/migrations/20260920_0001_init.sql` — tasks 表 DDL
2. `crates/task-service/src/db.rs` — PgPool + CRUD
3. `crates/task-service/src/models.rs` — DTO + SSE event
4. `crates/task-service/src/handlers.rs` — 4 REST + 1 SSE handlers
5. `crates/task-service/src/events.rs` — broadcast::channel 封装
6. `crates/task-service/src/lib.rs` + `src/main.rs` — 启动
7. `crates/task-service/tests/integration.rs` — sqlx::test + SSE 客户端测试
8. `crates/task-service/README.md`

## 验收标准

- 5 endpoint 全部实现
- SSE 用 actix-web 的 `actix_web_lab::sse` 或直接 stream
- RBAC 中间件挂在每个 endpoint
- 集成测试 ≥3 case
- cargo check -p task-service 通过

## 诚实披露

rustc 1.98 bug 仍在。cargo check 单 crate 应过。

## 作者署名

架构师(Mavis 接手 agent per DEC-008) <architecture@mavis.local>

## 引用

- 父: ULYS-125
- 审计: §4.2
- SSE 既有实现参考: 任务 `task-9d56a2010fec` 提交 af8abb6（ULYS-45）`GET /v1/tasks/{id}/events`
