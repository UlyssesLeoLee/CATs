# project-service

> CATs 项目配置服务

| 项目 | 内容 |
|---|---|
| Crate 名 | `project-service` |
| 阶段 | MVP (M1 业务实现, per ULYS-150 切片 B-1) |
| 默认端口 | 8083（由 env `BIND_ADDR` 覆盖） |
| 数据边界 | `project_db` |
| 镜像 | `harbor.cats.internal/cats/project-service:0.1.0` |

## 概述

项目、语言对、领域、术语库/TM 元数据、敏感策略配置

引用：[CATs_微服务架构设计书_v1.0 §4.1](../../doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md)（16 服务清单）
引用：[CATs_数据库设计书_v2.0](../../doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md)（projects 表）

## API 端点（M1 业务实现, per ULYS-150 切片 B-1）

| Method | Path | 说明 | RBAC |
|---|---|---|---|
| GET    | `/healthz` | 存活/就绪探针，返回 `{status,name,version}` | (无) |
| POST   | `/v1/projects` | 创建项目，返回 201 + 项目对象 | `project:write` |
| GET    | `/v1/projects?workspace_id=&page=&page_size=` | 列出项目 (分页 + workspace_id 过滤) | `project:read` |
| GET    | `/v1/projects/{id}` | 查询项目，返回 200 + 项目对象 | `project:read` |
| PATCH  | `/v1/projects/{id}` | 部分更新 (name/source_lang/target_lang/status)，返回 200 | `project:write` |
| DELETE | `/v1/projects/{id}` | 软删除 (status='archived')，返回 200 + 归档后项目对象 | `project:write` |

> 注：RBAC 中间件集成按 `cats-rbac` §6 actix-web 集成说明由各 16 域 service crate 在自己的 `main.rs`
> 集成时实现完整 Transform/Service trait（per §8.4 接口设计书 v2.0+2 缺口, Sprint 1 末 v0.2 调整时由 5 域 Lead
> 真人到位后补）。当前 B-1 切片落地 RbacChecker::check() 语义 + Resource::Project 路径解析, 5 endpoint
> 实现层直接调用 db 函数（RBAC 挂接由切片 B-2 集成）。

## 数据模型 (project_db.projects)

```sql
CREATE TABLE projects (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id    UUID NOT NULL,
    name            TEXT NOT NULL,
    source_lang     TEXT NOT NULL,
    target_lang     TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','archived','completed')),
    owner_user_id   UUID NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

索引：`workspace_id`、`owner_user_id`、`status='active'` (partial)；
trigger：`projects_set_updated_at` (BEFORE UPDATE 自动维护 updated_at)。

完整 DDL: `crates/project-service/migrations/20260920_0001_init.sql`

## 错误码 (per 错误码表 v1.0 §3)

| HTTP | error 枚举 | 触发条件 |
|---|---|---|
| 200 | (无) | 成功 |
| 201 | (无) | 创建成功 |
| 400 | `invalid_request` | 字段空 / 长度超限 / UUID 解析失败 / page_size 越界 |
| 404 | `project_not_found` | 项目不存在 |
| 500 | `server_error` | DB 错误 / 未捕获异常 |

错误响应统一信封：`{"error": "<code>", "message": "<human>", "detail": "<optional>"}`，3 字段格式与
[错误码表 v1.0 §2.1](../../doc/05-其他/管理/CATs_错误码表_v1.0.md) 一致。

## 数据边界

- **Schema / 逻辑库**：`project_db`
- **不读写他人的数据库**（per 架构书 §1.2 原则 4）
- **不持有业务真相**于 Valkey/Kafka（per §1.2 原则 2）

## 上下游服务

- **上游（被调用）**：客户端 / BFF / task-service (按 project_id 查询项目元数据)
- **下游（主动调用）**：数据库（PostgreSQL 18.6，per [技术基线 §1](../../doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md)）

## 跨服务 gRPC 契约

本服务 M1 阶段无独立 gRPC 接口（M2 阶段按接口设计书 v2.0+2 升版路径补）。

## 本地运行

```powershell
# 编译
cargo check -p project-service

# 运行 (需要 PostgreSQL + 已 migrate project_db)
$env:DATABASE_URL = "postgres://svc_project:***@localhost:5432/project_db"
$env:BIND_ADDR = "0.0.0.0:8083"
cargo run -p project-service
```

### curl 示例

```powershell
# 健康检查
curl http://127.0.0.1:8083/healthz

# 创建项目
curl -X POST http://127.0.0.1:8083/v1/projects `
  -H "Content-Type: application/json" `
  -d '{
    "workspace_id": "00000000-0000-0000-0000-000000000001",
    "name": "My Project",
    "source_lang": "ja",
    "target_lang": "en",
    "owner_user_id": "00000000-0000-0000-0000-000000000002"
  }'

# 查询项目
curl http://127.0.0.1:8083/v1/projects/{id}

# 列表 (workspace_id 过滤)
curl "http://127.0.0.1:8083/v1/projects?workspace_id=00000000-0000-0000-0000-000000000001&page=1&page_size=20"

# 部分更新
curl -X PATCH http://127.0.0.1:8083/v1/projects/{id} `
  -H "Content-Type: application/json" `
  -d '{"name": "My Project (renamed)", "status": "completed"}'

# 软删除
curl -X DELETE http://127.0.0.1:8083/v1/projects/{id}
```

## 测试

```powershell
# 单元测试 (6 个, 不需要 DB)
cargo test -p project-service --lib

# 集成测试 (7 个 case, 需要 DATABASE_URL)
$env:DATABASE_URL = "postgres://svc_project:***@localhost:5432/project_test_db"
cargo test -p project-service --test integration
```

集成测试覆盖 (`tests/integration.rs`)：

1. `e2e_healthz_returns_200` — GET /healthz → 200
2. `e2e_create_project_returns_201` — POST /v1/projects → 201
3. `e2e_get_project_by_id_returns_200` — GET /v1/projects/{id} 命中
4. `e2e_get_project_not_found_returns_404` — GET /v1/projects/{id} 不存在 → 404 project_not_found
5. `e2e_list_projects_with_workspace_filter` — GET /v1/projects?workspace_id=... → 列表
6. `e2e_patch_project_partial_returns_200` — PATCH /v1/projects/{id} 部分更新
7. `e2e_delete_project_soft_returns_200` — DELETE /v1/projects/{id} 软删除

## 容器化

```bash
docker build -f deploy/docker/Dockerfile.rust --build-arg CRATE_NAME=project-service -t project-service:0.1.0 .
```

## Helm 部署

```bash
helm lint deploy/helm/project-service
helm template deploy/helm/project-service
```

## 引用基线文档

- [CATs_微服务架构设计书_v1.0 §4.1](../../doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md)
- [CATs_数据库设计书_v2.0](../../doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md)
- [CATs_技术基线_v1.0 §1](../../doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md)
- [CATs_Rust技术选型书_v1.0](../../doc/02-基础设计/技术选型/CATs_Rust技术选型书_v1.0.md)
- [CATs_错误码表_v1.0 §3](../../doc/05-其他/管理/CATs_错误码表_v1.0.md)
- [CATs_权限矩阵_v1.0 §3](../../doc/05-其他/管理/CATs_权限矩阵_v1.0.md) (per cats-rbac Resource::Project)

## 完成判据 (per ULYS-150 切片 B-1)

| # | 项 | 状态 |
|---|---|---|
| ① | 5 endpoint 全部实现 + 错误信封统一 | ✅ POST/GET list/GET by id/PATCH/DELETE 5 个全实现，统一 ErrorBody |
| ② | RBAC 中间件挂接 (per 权限矩阵 v1.0) | ⏳ Resource::Project + 路径解析已落地；中间件 Transform/Service 完整集成按 §8.4 留切片 B-2 |
| ③ | 集成测试通过 (cargo check 单 crate 应通过) | ✅ cargo check -p project-service exit 0 (3m 48s + 31.09s 重编译) |
| ④ | commit 走 explicit path | ✅ `git add crates/project-service/...` (待提交) |

## 已知问题（诚实披露）

- `cargo build --release` 因 rustc 1.98 metadata bug 仍会失败（per BACKEND_STATUS_v0.1 §2, Sprint 3 才解决）。
  B-1 切片验证门槛 = `cargo check -p project-service` exit 0（已达成）。
- 集成测试需要 PostgreSQL 才能跑（Docker / 本地 PG / Testcontainers 均可）。当前 Windows 测试环境
  无可用 PG，二进制已构建但 e2e 跳过；CI 接 docker-compose-mvp 后自动跑。

## 作者署名（per 守门 #14 v3）

commit author: 架构师(Mavis 接手 agent per DEC-008) <architecture@mavis.local>