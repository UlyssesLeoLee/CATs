# 切片 A: cats-bff 业务 8 endpoint 串通

**Slice**: A — BFF 业务 endpoint 串通（依赖最小、最高优先级）
**目标 crate**: `crates/cats-bff/`
**基线**: `agent/minimaxm3/d11b0bce579f` (HEAD)

## 任务范围

把 cats-bff 从 `/healthz` 占位扩展到 OpenAPI v1.0.1 §paths 全部 8 endpoint：

| endpoint | 上游 | 备注 |
|---|---|---|
| POST /auth/login | auth-service:8081 | 转发 + 注入 JWT |
| POST /auth/refresh | auth-service:8081 | 转发 |
| POST /auth/logout | auth-service:8081 | 转发 |
| GET /auth/me | auth-service:8081 + user-service:8082 | 聚合 |
| GET /projects | project-service:8083 | 转发 + RBAC |
| POST /projects | project-service:8083 | 转发 + RBAC |
| POST /tasks | task-service:8084 + translation-core:8086 | 转发 + 关联 |
| GET /healthz | 本地 | ✅ 已有 |

## 复用代码（参考已有实现）

- `crates/cats-bff/src/main.rs` — 当前 /healthz 实现 + Config::from_env 模式
- `crates/auth-service/src/handlers.rs` — auth endpoint 调用模式
- `crates/common/src/lib.rs` — CatsError + 错误信封
- `crates/cats-rbac/src/lib.rs` — RBAC 中间件
- `apps/cats-client/src-tauri/src/api/client.rs` — BFF 调用模式（看上游怎么调 BFF）

## BFF 配置

```rust
struct Config {
    bind_addr: String,                    // 0.0.0.0:8097
    auth_service_url: String,             // http://localhost:8081
    user_service_url: String,             // http://localhost:8082
    project_service_url: String,          // http://localhost:8083
    task_service_url: String,             // http://localhost:8084
    translation_core_url: String,         // http://localhost:8086
}
```

env 读取：SERVICE_BIND_ADDR / AUTH_SERVICE_URL / ... 等。

## 交付清单

1. `crates/cats-bff/src/config.rs` — 多上游 URL 配置
2. `crates/cats-bff/src/upstream/` — 各上游服务的 reqwest 客户端封装
   - `auth.rs` (login/refresh/logout/me)
   - `projects.rs` (list/create)
   - `tasks.rs` (dispatch)
3. `crates/cats-bff/src/handlers/` — BFF 自己的 8 endpoint handlers
4. `crates/cats-bff/src/error.rs` — BFF 错误信封统一
5. `crates/cats-bff/src/lib.rs` — 模块导出
6. `crates/cats-bff/src/main.rs` — 路由注册 + RBAC
7. `crates/cats-bff/tests/bff_smoke.rs` — 用 mock upstream 的集成测试
8. `crates/cats-bff/README.md` — endpoint 表 + 配置 env 列表

## 验收标准

- 8 endpoint 全部实现（其中 /healthz 已有）
- 错误信封与接口设计书 v2.0 §1.3 一致
- RBAC 中间件挂在每个业务 endpoint
- 上游失败时 BFF 返回 502/504 包裹在统一错误信封里
- cargo check -p cats-bff 通过

## 诚实披露（per BACKEND_STATUS_v0.1 §2）

`cargo build --release` 因 rustc 1.98 metadata bug 仍会失败。cargo check 单 crate 应过。

## 作者署名

架构师(Mavis 接手 agent per DEC-008) <architecture@mavis.local>

## worktree

`C:/Users/leo19/multica_workspaces_desktop-api.multica.ai/ulysses-ca266aa77e9f/ulys-125-d11b0bce579f/workdir/CATs`

## 引用

- 父: ULYS-125
- 审计: `doc/05-其他/MVP商业版/CATs_FEATURE_AUDIT_v1.0.md` §4.1
- OpenAPI: `api/openapi/cats-openapi-v1.0.1.yaml`
- 接口设计: `doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md`
- 微服务架构: `doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md` §4.1
