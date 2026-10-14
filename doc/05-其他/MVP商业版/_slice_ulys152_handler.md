# ULYS-152 handler RBAC 实际接入 spec

**任务**: ULYS-152 (切片 B-3 + B-4, file + notification)
**目标**: 把已经写好的 rbac.rs 模块实际接到 handlers.rs, 替换 Uuid::nil() 占位
**基线**: `agent/minimaxm3/d11b0bce579f` 分支

## 问题 (per reviewer cf3de69f)

file-service + notification-service handlers 当前用 `Uuid::nil()` 占位 owner_user_id / user_id:
```rust
let owner_user_id = Uuid::nil(); // M1 简化: 占位, 真实从 JWT 解析
```

身份由调用方自己声明, 跨用户越权可随意伪造.

## 已有基础设施 (本批已落地)

- `crates/file-service/src/rbac.rs` (300+ 行, 含 10 单测)
- `crates/notification-service/src/rbac.rs` (300+ 行, 含单测)
- `cats-rbac.workspace = true` 已加 Cargo.toml

## 任务清单

1. 创建自己的 worktree:
   ```
   cd C:/Users/leo19/multica_workspaces_desktop-api.multica.ai/.repos/d83598db-3bfe-4386-b86d-ca266aa77e9f/github.com+UlyssesLeoLee+CATs.git
   git worktree add "C:/Users/leo19/multica_workspaces_desktop-api.multica.ai/ulysses-ca266aa77e9f/ulys-152-handler-rbac/workdir/CATs" -b agent/minimaxm3/b152-handler-rbac agent/minimaxm3/d11b0bce579f
   ```
2. **file-service handlers** 改造 (4 endpoint):
   - upload_file (POST /v1/files): 加 req: HttpRequest + rbac_checker: web::Data<Arc<RbacChecker>>, 调 rbac::enforce, owner_user_id 改用 auth.user_id.unwrap_or(Uuid::nil()) (实际应该总有 user_id, 兜底 nil 是历史)
   - list_files (GET /v1/files): 同上
   - get_file_metadata (GET /v1/files/{id}): 同上
   - delete_file (DELETE /v1/files/{id}): 同上
   - download_file (GET /v1/files/{id}): 同上
3. **notification-service handlers** 改造 (3 endpoint):
   - list_notifications (GET /v1/notifications): 加 RBAC, user_id 从 AuthContext 取
   - create_notification (POST /v1/notifications): 同上
   - mark_notification_read (PATCH /v1/notifications/{id}/read): 同上 (并且 DB mark_read 已有 user_id 校验)
   - notification_stream (GET /v1/notifications/ws): SSE endpoint 也挂 RBAC
4. **main.rs** 改造: 两个 crate 都加 Arc<RbacChecker> 注入到 AppState
5. 加 integration test: 每个 endpoint 加 401 (无 token) 用例
6. commit author = `架构师(Mavis 接手 agent per DEC-008) <architecture@mavis.local>`
7. push origin + 报 commit hash

## 验收

- cargo check -p file-service -p notification-service exit 0
- cargo test -p file-service --lib 全过
- cargo test -p notification-service --lib 全过
- 401 无 token / 403 错 role 测试存在

## 守门合规

#1 禁回溯 / #5 不动 rust-toolchain.toml / #9 v19 explicit path / #14 v3 代签 / L11 cargo 1 次