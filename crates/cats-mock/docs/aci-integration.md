# CATs × ACI emitter 集成设计

> **ULYS-191 §4.3.2** — CATs 接入 `aci-emitter v0.1.0` 集成设计.
> Stage 3.1 最小可交付 (本笔).

## §1 背景

CATs (`D:/CATs`) 是 23+ crate monorepo (per `Cargo.toml`), 含 22 服务 crate + 1 mock crate (`cats-mock`).
`cats-mock` 是 CATs 测试 Mock 项目 (per 设计书 v1.0):

| 模块 | 子文件 | 职责 |
|---|---|---|
| `data/` | user/project/task/audit.rs | 业务对象 factory |
| `db/` | fixture/schema/seed.rs | PG 18.6 + pgvector fixture |
| `http/` | server/routes/response.rs | actix-web mock HTTP server |
| `infra/` | kafka/redis.rs | Kafka/Redis in-memory mock |
| `smoke.rs` | 2.4K | 现有 smoke 测试 |
| `scripts/` | add_devdep.ps1 / rewrite_smoke.ps1 / run_test.ps1 | 3 PowerShell 脚本 |

per ULYS-191 §4.3.2 brief v0.1:
- Stage 3.1 最小可交付 (本笔): 加 `.aci.json` + `aci-emitter` git dep + 1 helper 模块 + 3 IT + 1 smoke
- Stage 3.2+ 后续 brief: 与现有 4 大模块集成

## §2 集成路径

### 2.1 依赖声明 (2 处)

**`Cargo.toml` (workspace 根)** — 在 `[workspace.dependencies]` 段 (在 `cats-mock = { path = "crates/cats-mock" }` 前) 加:

```toml
# ACI 跨项目 emitter (per ULYS-191 §4.3.2 brief v0.1)
aci-emitter = { git = "https://github.com/UlyssesLeoLee/aci-emitter", rev = "df28c56" }
```

**`crates/cats-mock/Cargo.toml`** — 在 `[dependencies]` 段加:

```toml
# ACI 跨项目 emitter
aci-emitter = { workspace = true }
```

锁 `rev = df28c56` (= ULYS-191.1 main HEAD), 防止上游 master 推进时漂移.

### 2.2 字段 1:1 对齐

CATs 的 `.aci.json` 1:1 拷贝自 Star [`tools/star-flash-mock/.aci.json`](https://github.com/UlyssesLeoLee/Star/blob/main/tools/star-flash-mock/.aci.json).
所有字段名 (10 必填 + 2 条件)、4 测试层、5 严重度、4 状态、17 expect_value_types、6 scope_dimensions 全部一致.

### 2.3 CATs 端 Helper

`crates/cats-mock/src/aci_emitter_helper.rs` 暴露 1 个公开函数:

```rust
use cats_mock::aci_emitter_helper::emit_smoke_assertion;
let a = emit_smoke_assertion();
assert_eq!(a.assertion_id, "cats-mock:smoke:g-1");
```

### 2.4 集成 seam (Stage 3.2+ 后续 brief)

`emit_smoke_assertion()` 与 cats-mock 现有 4 大模块 (data/db/http/infra) 集成:
- `data::*` factory 配对: emit 后用 factory 验
- `http::MockServer` 配对: emit server 启动 + 用 emit_smoke 验
- `infra::{MockKafka, MockRedis}` 配对: emit 消息 + 用工厂验
- `db::DbFixture` 配对: emit DB 操作 + 用工厂验

## §3 CI 跨项目验证

### 3.1 当前 CI 范围 (CATs 仓)

CATs 仓 `.github/workflows/ci.yml` 配置不在本笔范围, 由后续 brief 处理.
本笔先本机 5 项验收.

### 3.2 aci-emitter 上游变更同步

CATs CI **不主动** 监听 aci-emitter upstream 变更, 但:

- (a) `aci-emitter = { git = "...", rev = "df28c56" }` 锁 rev, 上游 master 推进不影响 CATs
- (b) 上游 release tag 时人工 bump `rev`, 走 PR 流程
- (c) §4.5 跨项目 CI 落地后, 改 crates.io publish 或 path 依赖

## §4 未来 (per Stage 3 报告)

| 阶段 | brief | 范围 |
|---|---|---|
| §4.3.1 | ULYS-191.3 (ULYS-226) | RGS 集成 ✅ SHIPPED |
| §4.3.2 | **本笔 (ULYS-191.4 / ULYS-233)** | CATs 集成 (接 cats-mock) |
| §4.3.3 | ULYS-191.5 (ULYS-227) | IM1.0 集成 ✅ SHIPPED |
| §4.3.4 | ULYS-191.6 | Ada 集成 (接 ada-mock) |
| §4.3.5 | ULYS-191.7 | GitGit 集成 (TS emitter) |

## §5 风险

| # | 风险 | 缓解 |
|---|---|---|
| R-1 | **git dep aci-emitter 跨项目漂移** | 锁 rev + §4.5 跨项目 CI 监听 |
| R-2 | **CATs workspace 23+ crates 与 cats-mock 不共享 deps** | 本笔仅影响 cats-mock 子 crate, 后续 brief 给 CATs main 也加 |
| R-3 | **CATs main 分支已 ahead of origin/main** | worktree 明确基于 `main` 不是 `origin/main` |
| R-4 | **CATs compile error (per build2.log / build_err.log)**: CATs main 已知有 rustc 1.98 metadata bug | 本笔只验证 cats-mock 子 crate, 不动 main 22+ crates |

## §6 参考

- [ULYS-191 §4.3.2 brief v0.1](./regression-report-2026-09-24.md)
- [aci-emitter v0.1.0 (ULYS-224)](https://github.com/UlyssesLeoLee/aci-emitter)
- [Star `.aci.json` schema v0.1](https://github.com/UlyssesLeoLee/Star/blob/main/tools/star-flash-mock/.aci.json)
- [IDE1.0 §4.2.2 集成设计](https://github.com/UlyssesLeoLee/IDE1.0) — 参考 pattern
- [RGS §4.3.1 集成设计](https://github.com/UlyssesLeoLee/RustGameServer) — 参考 pattern
- [IM1.0 §4.3.3 集成设计](https://github.com/UlyssesLeoLee/IM1.0) — 参考 pattern
- cats-mock 设计书: `doc/02-基础设计/测试/CATs_测试Mock项目设计书_v1.0.md`
