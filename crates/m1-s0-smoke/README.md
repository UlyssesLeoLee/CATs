# cats-m1-s0-smoke

M1-S0 兼容性冒烟 crate。验证 CATs 技术栈在 **Rust 1.98.0** 下能编译 + 通过基础功能测试。

> 详见 `doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md` §2.3（OI-3 待办）。

## 验证目标（6 项库）

| 库 | 版本 | 验证内容 | 跑通状态 |
|---|---|---|---|
| actix-web | 4.x | `/healthz` HTTP 200 + JSON 响应 | ✅（仅编译期 + 单元测试，不绑端口） |
| sqlx | 0.8 | PgPool lazy build + PgRow 类型可见 | ✅（不连 DB，CI 无 PG） |
| tonic | 0.12 | tonic-build + service trait + Status/Request/Response | ✅（编译期，不启 server） |
| yrs | 0.18 | Doc + Text insert/get round-trip | ✅（内存测试） |
| tauri | 1.x | `cats-m1-s0-smoke/src/tauri_smoke.rs` type-check 编译通过 | ✅（仅编译期；UI 实施在 `apps/cats-client/`） |
| rdkafka | 0.36 | **不**进 smoke crate（cmake-build 需 librdkafka 系统依赖） | 移出 → 由 `crates/cats-bff/` + Kafka worktree 验证（K3s 阶段二） |

## 不在 smoke 范围

- **PG 18.6 + pgvector 0.8.6 实际部署**：走 `ci/scripts/dev-up.ps1` + RGS-PGV18-INSTALL-SOP（per OI-4）
- **tauri 2.x 客户端**：由 `apps/cats-client/` worktree 独立验证（M1-Sprint 1）
- **rdkafka 0.36**：由 `crates/cats-bff/` + Kafka worktree 验证（K3s 阶段二）

## 跑通命令

```bash
cargo test -p cats-m1-s0-smoke
cargo build -p cats-m1-s0-smoke --release
```

## 完成判据

- `cargo build -p cats-m1-s0-smoke` exit 0
- `cargo test -p cats-m1-s0-smoke` exit 0（含 4 子模块单元测试）
- `cargo clippy -p cats-m1-s0-smoke -- -D warnings` exit 0

## 不属于本 crate

- 业务逻辑、HTTP 端点、数据库 schema — 全部归各 service crate
- 网络层、鉴权、限流 — 走 `crates/cats-common/` 共享
- 真实 PG/Kafka 连接 — 走 deployment 实战
