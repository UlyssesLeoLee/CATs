# 切片 B-3 + B-4: file-service + notification-service

**Slice**: B-3 + B-4 合并 — 后端核心 4 服务中的 2 个
**目标 crate**: `crates/file-service/` + `crates/notification-service/`
**基线**: `agent/minimaxm3/d11b0bce579f`

## 切片 B-3: file-service

### 任务范围

文件上传/下载/元数据管理：

- `POST /v1/files` — multipart upload，存到本地磁盘或返回二进制内容（BFF 透传）
- `GET /v1/files/{id}` — 下载
- `GET /v1/files/{id}/metadata` — 元数据
- `DELETE /v1/files/{id}` — 软删除

### 数据模型

files 表：
- id UUID PK
- workspace_id UUID
- owner_user_id UUID
- filename TEXT
- content_type TEXT
- size_bytes BIGINT
- storage_path TEXT  -- 本地磁盘路径 / S3 key
- sha256 TEXT
- status TEXT DEFAULT 'active'
- created_at / updated_at

### 复用

- `crates/user-service/src/handlers.rs`
- actix-multipart 或 axum extract

## 切片 B-4: notification-service

### 任务范围

通知列表 + WebSocket 实时推送：

- `GET /v1/notifications` — 列出通知（按 user_id）
- `PATCH /v1/notifications/{id}/read` — 标记已读
- `WS /v1/notifications/ws` — WebSocket 实时推送

### 数据模型

notifications 表：
- id UUID PK
- user_id UUID
- type TEXT  -- task_completed / task_failed / project_invited / etc
- title TEXT
- body TEXT
- payload JSONB
- read_at TIMESTAMPTZ NULL
- created_at TIMESTAMPTZ DEFAULT now()

### 复用

- `actix-ws` 或 `actix-web-actors` for WebSocket
- `tokio::sync::broadcast` for fan-out

## 交付清单（两服务）

1. 各 crate migrations/20260920_0001_init.sql
2. 各 crate src/{db,models,handlers,lib,main}.rs
3. 各 crate tests/integration.rs
4. 各 crate README.md

## 验收标准

- file-service 4 endpoint + notification-service 3 endpoint 全部实现
- 错误信封 + RBAC + sqlx::test
- cargo check -p file-service -p notification-service 通过

## 诚实披露

rustc 1.98 bug 仍在。cargo check 单 crate 应过。

## 作者署名

架构师(Mavis 接手 agent per DEC-008) <architecture@mavis.local>

## 引用

- 父: ULYS-125
- 审计: §4.2
