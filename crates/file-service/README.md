# file-service

> CATs 文件存取服务

| 项目 | 内容 |
|---|---|
| Crate 名 | `file-service` |
| 阶段 | MVP (ULYS-152 切片 B-3) |
| 默认端口 | 8085（由 env `BIND_ADDR` 覆盖） |
| 数据边界 | `file_db` |
| 镜像 | `harbor.cats.internal/cats/file-service:0.1.0` |

## 概述

文件上传 / 下载 / 元数据 / 软删除。

引用：[CATs_微服务架构设计书_v1.0 §4.1](../../doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md)、[ULYS-152 切片 B-3](#)

## API 端点

| Method | Path | 说明 |
|---|---|---|
| GET    | `/healthz` | 存活/就绪探针 |
| POST   | `/v1/files` | 上传（JSON + base64 content；M1 简化，Sprint 2 升 multipart） |
| GET    | `/v1/files` | 列表（按 `workspace_id` + 分页 `limit`/`offset`） |
| GET    | `/v1/files/{id}` | 下载（返回 JSON 包装 `{file, content_base64}`） |
| GET    | `/v1/files/{id}/metadata` | 元数据 |
| DELETE | `/v1/files/{id}` | 软删除（`status='deleted'`） |

### POST /v1/files 请求示例

```json
{
  "workspace_id": "11111111-2222-3333-4444-555555555555",
  "filename": "doc.txt",
  "content_type": "text/plain",
  "content_base64": "aGVsbG8gd29ybGQ="
}
```

### POST /v1/files 响应示例（201）

```json
{
  "file": {
    "id": "...",
    "workspace_id": "...",
    "owner_user_id": "...",
    "filename": "doc.txt",
    "content_type": "text/plain",
    "size_bytes": 11,
    "sha256": "...",
    "status": "active",
    "created_at": "2026-09-20T...",
    "updated_at": "2026-09-20T..."
  },
  "deduplicated": false
}
```

## 错误码（per `CATs_错误码表_v1.0.md` §3）

| HTTP | error 枚举 | 触发条件 |
|---|---|---|
| 400 | `invalid_request` | filename 空 / 长度超限 / base64 解码失败 / UUID 非法 |
| 404 | `file_not_found` | 文件不存在或已删除 |
| 413 | `file_too_large` | size > 100 MiB |
| 500 | `server_error` | DB / IO 错误 |

## 数据边界

- **Schema / 逻辑库**：`file_db`
- **存储**：本地磁盘（`FILE_STORAGE_ROOT` env，默认系统 temp 目录 `file-storage/` 子目录；Sprint 3 切 S3）
- **不直连** `auth_db` / `user_db` / 其他服务数据库（per 架构书 §1.2 原则 4）
- **软删除**：`status='deleted'`，物理文件保留（审计追溯）

## 上下游服务

- **上游（被调用）**：客户端 / BFF
- **下游（主动调用）**：仅 `file_db`（per Sprint 1 数据边界）

## RBAC

per `CATs_权限矩阵_v1.0.md` §3：`/v1/files` 路径已纳入 `cats_rbac::Resource::File` 解析。

M1 简化：endpoint handler 未强制注入 RbacChecker（per 切片 B-3 任务范围仅实现 CRUD）；
Sprint 2 在 main.rs 中间件链挂 `RbacChecker::check(...)`。

## 本地运行

```powershell
# 编译
cargo check -p file-service

# 运行
$env:DATABASE_URL = "postgres://svc_file:***@localhost:5432/file_db"
$env:BIND_ADDR = "0.0.0.0:8085"
$env:FILE_STORAGE_ROOT = "D:\cats\file-storage"
cargo run -p file-service

# 健康检查
curl http://127.0.0.1:8085/healthz

# 上传
$content = [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes("hello"))
curl -X POST http://127.0.0.1:8085/v1/files `
  -H "Content-Type: application/json" `
  -d '{"workspace_id":"11111111-2222-3333-4444-555555555555","filename":"hello.txt","content_type":"text/plain","content_base64":"'$content'"}'
```

## 测试

```powershell
# 单元测试 (lib + tests/integration.rs)
# 注: integration 测试需要 file_test_db 在线 (per DATABASE_URL env)
cargo test -p file-service
```

## 引用基线文档

- [CATs_微服务架构设计书_v1.0 §4.1](../../doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md)
- [CATs_技术基线_v1.0 §1](../../doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md)
- [CATs_Baseline一览_v1.0 §5.1](../../doc/05-其他/管理/CATs_Baseline一览_v1.0.md)
- [CATs_错误码表_v1.0 §3](../../doc/05-其他/管理/CATs_错误码表_v1.0.md)
- [CATs_权限矩阵_v1.0 §3](../../doc/05-其他/管理/CATs_权限矩阵_v1.0.md)
- [ULYS-152 切片 B-3](../../.multica/issues/ULYS-152.md)