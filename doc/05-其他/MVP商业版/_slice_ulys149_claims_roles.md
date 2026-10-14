# ULYS-149 第二层缺口: Claims 加 roles 字段 spec

**任务**: ULYS-149 (cats-bff) 第二层缺口修复
**目标**: 让 RBAC 真正生效 — auth-service Claims 加 roles, login 时注入, BFF 解析时拿到真值
**基线**: `agent/minimaxm3/d11b0bce579f` 分支 (已含 PR #12 三 commits)

## 问题 (per reviewer cf3de69f 9/23 复审)

PR #12 修了 JWT 验签 (`a3dcdc0`), 但 `auth-service/src/models.rs:59-66` 的 Claims struct 没有 `roles` 字段:
```rust
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub exp: i64,
    pub iat: i64,
    pub jti: String,
    pub token_type: String,
}
```

`BFF/src/principal.rs` 解析 `roles` 时恒空 (token payload 里没有这字段), 所以所有 RBAC 检查都拒绝. 8 endpoint 业务目标仍未达成.

## 任务清单

1. 创建 worktree:
   ```
   cd C:/Users/leo19/multica_workspaces_desktop-api.multica.ai/.repos/d83598db-3bfe-4386-b86d-ca266aa77e9f/github.com+UlyssesLeoLee+CATs.git
   git worktree add "C:/Users/leo19/multica_workspaces_desktop-api.multica.ai/ulysses-ca266aa77e9f/ulys-149-claims-roles/workdir/CATs" -b agent/minimaxm3/u149-roles agent/minimaxm3/d11b0bce579f
   ```
2. **auth-service** 改造:
   - `models.rs` `Claims` 加 `#[serde(default)] pub roles: Vec<String>`
   - `handlers.rs` `login` 函数: 从 DB 查 user_id 对应的 roles (per `crates/cats-rbac` Role enum 9 个), 注入到 login 返回的 JWT payload 里 (`Claims.roles` 反序列化后存在)
   - 加测试: `Claims { ..., roles: vec!["User".to_string()] }` roundtrip
3. **cats-bff** 微调:
   - `principal.rs` `Claims.roles` 字段已存在, 不需要改
   - 但加测试: `Claims { roles: vec!["User"] }` → `Principal { roles: vec![Role::User] }` 完整链
4. **集成验证**:
   - 用两个 crate 都过
   - 加 e2e: 模拟 login → JWT 带 roles → BFF 解析 → RBAC 允许
5. commit author = `架构师(Mavis 接手 agent per DEC-008) <architecture@mavis.local>`
6. push origin + 报 commit hash

## 验收

- cargo check -p auth-service -p cats-bff exit 0
- cargo test -p auth-service --lib 全过
- cargo test -p cats-bff --lib 全过
- Claims.roles roundtrip 测试 + Principal.roles 解析测试存在

## 守门合规

#1 禁回溯 / #5 不动 rust-toolchain.toml / #9 v19 explicit path / #14 v3 代签 / L11 cargo 1 次