# 切片 B-1: project-service 业务实现

**Slice**: B-1 — 后端核心 4 服务
**目标 crate**: `crates/project-service/`
**基线**: `agent/minimaxm3/ce1e3b.../d11b0bce579f` (HEAD)

## 任务范围

把 project-service 从 `/healthz` 占位扩展到完整 CRUD：
- `POST /v1/projects` — 创建项目
- `GET /v1/projects` — 列出项目（分页 + 过滤 by workspace_id）
- `GET /v1/projects/{id}` — 获取详情
- `PATCH /v1/projects/{id}` — 更新（name/source_lang/target_lang/status）
- `DELETE /v1/projects/{id}` — 软删除（status='archived'）

## 复用代码（参考已有实现，不要重写）

- `crates/user-service/src/db.rs` — sqlx PgPool 构造 + migrate 模式
- `crates/user-service/src/handlers.rs` — handlers 写法（统一错误信封）
- `crates/user-service/src/models.rs` — 请求/响应模型 + ErrorBody
- `crates/cats-rbac/src/lib.rs` — RBAC 中间件使用方式
- `crates/common/src/lib.rs` — AppMeta + init_tracing + CatsError

## 数据模型（参考 doc/03-详细设计/数据库设计）

projects 表：
- id UUID PK
- workspace_id UUID FK → workspaces(id)
- name TEXT NOT NULL
- source_lang TEXT NOT NULL
- target_lang TEXT NOT NULL
- status TEXT DEFAULT 'active' CHECK (status IN ('active','archived','completed'))
- created_at TIMESTAMPTZ DEFAULT now()
- updated_at TIMESTAMPTZ DEFAULT now()
- owner_user_id UUID FK → users(id)

## 交付清单

1. `crates/project-service/migrations/20260920_0001_init.sql` — projects 表 DDL
2. `crates/project-service/src/db.rs` — PgPool + migrate + CRUD 函数
3. `crates/project-service/src/models.rs` — 请求/响应结构体
4. `crates/project-service/src/handlers.rs` — 5 endpoint handlers
5. `crates/project-service/src/lib.rs` — 模块导出
6. `crates/project-service/src/main.rs` — 启动 + route 注册 + RBAC 中间件
7. `crates/project-service/tests/integration.rs` — sqlx::test 集成测试（≥3 case）
8. `crates/project-service/README.md` — endpoint 清单 + curl 示例

## 验收标准

- 5 endpoint 全部实现 + 错误信封统一（per 接口设计书 v2.0 §1.3）
- RBAC 中间件挂在每个 endpoint（per 权限矩阵 v1.0 — project:read / project:write）
- 集成测试通过（即使 cargo build 因 rustc 1.98 bug 失败，cargo check -p project-service 应通过）
- commit 走 explicit path（`git add crates/project-service/...` 而非 `git add -A`）

## 诚实披露（per BACKEND_STATUS_v0.1 §2）

`cargo build --release` 因 rustc 1.98 metadata bug 仍会失败（已知问题，Sprint 3 才解决）。**cargo check 单 crate 应该过**——这是本切片的可验证门槛。

## 作者署名（per 守门 #14 v3）

commit author 固定为：
```
架构师(Mavis 接手 agent per DEC-008) <architecture@mavis.local>
```

修订人 / 审批人字段 commit body 写明。

## worktree 路径

`C:/Users/leo19/multica_workspaces_desktop-api.multica.ai/ulysses-ca266aa77e9f/ulys-125-d11b0bce579f/workdir/CATs` (已就绪，branch: agent/minimaxm3/d11b0bce579f)

## 引用

- 父 issue: ULYS-125 CATs 功能点罗列
- 审计: `doc/05-其他/MVP商业版/CATs_FEATURE_AUDIT_v1.0.md` §4.2
- 接口规范: `api/openapi/cats-openapi-v1.0.1.yaml` §/projects
- 模块设计: `doc/03-详细设计/模块设计/CATs_模块设计书_v2.2.md` §project-service
- 数据库: `doc/03-详细设计/数据库设计/CATs_SQL设计一览_v1.0.md`
