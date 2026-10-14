# ULYS-153 report + audit RBAC 集成 spec

**任务**: ULYS-153 (切片 C, report + audit)
**目标**: 给 report-service 加 cats-rbac + RBAC, 修复 audit-service RBAC 孤儿代码
**基线**: `agent/minimaxm3/d11b0bce579f` 分支

## 问题 (per reviewer cf3de69f 9/21 + 9/23 复审)

- **report-service**: 3 endpoint (usage / translation-volume / audit-summary) 零鉴权 + 跨租户越权
- **audit-service**: RBAC 集成是孤儿代码 (声明了但没接, 编译错误)

## 任务清单

1. 创建 worktree:
   ```
   cd C:/Users/leo19/multica_workspaces_desktop-api.multica.ai/.repos/d83598db-3bfe-4386-b86d-ca266aa77e9f/github.com+UlyssesLeoLee+CATs.git
   git worktree add "C:/Users/leo19/multica_workspaces_desktop-api.multica.ai/ulysses-ca266aa77e9f/ulys-153-report-audit-rbac/workdir/CATs" -b agent/minimaxm3/b153-rbac agent/minimaxm3/d11b0bce579f
   ```
2. **report-service** 改造:
   - Cargo.toml 加 cats-rbac dep
   - 新建 src/rbac.rs (per project-service rbac.rs 模板, Resource::Report)
   - 改 handlers.rs 3 endpoint 都挂 rbac::enforce
   - 改 main.rs 注入 RbacChecker
   - 加测试
3. **audit-service** 改造:
   - 修 audit.rs 编译错误 (per reviewer: RBAC 集成为孤儿代码)
   - 把孤儿代码接到真实 endpoint (per `crates/audit-service/src/handlers.rs` 现有 GET /v1/audit)
   - 加 RBAC 测试
4. commit author = `架构师(Mavis 接手 agent per DEC-008) <architecture@mavis.local>`
5. push origin + 报 commit hash

## 验收

- cargo check -p report-service -p audit-service exit 0
- cargo test -p report-service --lib 全过
- cargo test -p audit-service --lib 全过 (含 5 个 reviewer 提到的 audit 测试)
- audit.rs 编译错误修复 (0 error / 0 warning)

## 守门合规

#1 禁回溯 / #5 不动 rust-toolchain.toml / #9 v19 explicit path / #14 v3 代签 / L11 cargo 1 次