# CATs Rust 技术选型书

**系统名称:** CATs — 全媒体 AI 辅助翻译 SaaS 平台

---

## 文档管理信息

| 项目 | 内容 |
|---|---|
| 文档编号 | CATs-TS-RUST-001 |
| 文档名 | Rust 技术选型书（含 ADR 决策记录，Rust 生态深度） |
| 版本 | 第 1.0 版（草稿） |
| 创建日 | 2026-08-19 |
| 作者 | 架构师 |
| 状态 | 评审前草稿 |
| 密级 | 仅社内 |
| 适用标准 | Rust API Guidelines / Rust Standard Library / RFC 标准化 / [Rust 官方 ISRG 选择清单](https://www.rust-lang.org/) |
| 上游文档 | [CATs 技术选型书 v2.0](./CATs_技术选型书_v2.0.md)（横向补充：本书专门深化 Rust 生态） |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|---|---|---|---|
| 1.0 | 2026-08-19 | 架构师 | 初版。Rust 生态深度选型：① 工具链（rustc/cargo/clippy/rustfmt/cargo-audit 等）；② 异步运行时（Tokio）；③ Web 框架（Axum / Tauri）；④ 数据库（sqlx）；⑤ 消息（rdkafka / redis-rs）；⑥ 序列化（serde / prost）；⑦ 可观测性（tracing / OpenTelemetry / metrics）；⑧ 安全（ring / jsonwebtoken / argon2）；⑨ 测试（cargo test / mockall / testcontainers / proptest）；⑩ 媒体处理（ffmpeg-next / image） |

### 审批栏

| 角色 | 姓名 | 审批日 | 签字 |
|---|---|---|---|
| 起草 |  |  |  |
| 评审 |  |  |  |
| 批准 |  |  |  |

---

## 0. 阅读指南

本书是 **CATs 技术选型书 v2.0 的横向补充**——v2.0 覆盖了 Tauri/Svelte/Next.js/K3s/PostgreSQL/Kafka/Debezium 等整套技术基线，**但对 Rust 内部生态（编译器版本、异步运行时、Web 框架、序列化、可观测性、错误处理、安全、测试等）仅有粗粒度提及**。本书专门做 **Rust 生态深度选型**，逐类给出 ADR 决策记录（背景/候选/决策/理由/取舍/结论）。

配套 Excel `CATs_Rust技术选型书_v1.0.xlsx` 包含可筛选的选型总览表、ADR 决策记录、依赖版本锁定清单、风险与缓解表。

---

## 1. 前言

### 1.1 目的

- 明确 CATs 项目所有 **Rust 代码**（Tauri 客户端核心、Rust 服务、`render-writer-service` 等）使用的工具链与生态库选型
- 提供 ADR 决策记录，便于后续维护、升级、替换决策
- 锁定依赖版本，避免生态快速迭代引入兼容性风险
- 降低团队学习成本与维护负担

### 1.2 范围

**在范围内（Rust 生态）：**

- 工具链：rustc、cargo、clippy、rustfmt、cargo-audit、cargo-deny、cargo-outdated、cargo-nextest、cargo-flamegraph
- 异步运行时：Tokio
- Web 框架：Axum（服务）+ Tauri（客户端）+ tower（中间件）
- 序列化：serde、prost
- 数据库：sqlx、tokio-postgres、deadpool
- 消息：rdkafka、redis-rs、deadpool-redis
- 可观测性：tracing、tracing-subscriber、tracing-opentelemetry、metrics、metrics-exporter-prometheus
- 错误处理：thiserror、anyhow
- 安全：ring、jsonwebtoken、argon2
- 测试：cargo test、mockall、wiremock、testcontainers-rs、proptest、rstest、criterion
- 配置：config / figment
- 时间：time / chrono
- 容器/部署：bollard、kube-rs

**在范围外：**

- Tauri 前端（WebView 中的 Svelte 5 / TypeScript）：见 `CATs_技术选型书_v2.0` 与 `CATs_模块设计书_v2.0 §2`
- 其他语言生态（Python / Node.js / TypeScript）：不在本书范围
- 商业 LLM、模型服务：不在本书范围

### 1.3 与 v2.0 技术选型书的关系

| 维度 | v2.0 技术选型书 | 本书（Rust 选型） |
|---|---|---|
| 范围 | 全部技术基线 | 仅 Rust 生态 |
| 颗粒度 | 顶层技术选型（如 Tauri、K3s、PostgreSQL） | Rust 库选型（如 axum、tokio、sqlx） |
| 决策 | 30+ 个 ADR | 20+ 个 ADR（Rust 专用） |
| 适用读者 | 全员 | Rust 开发 / 架构师 |
| 版本锁定 | 顶层版本 | 库版本（带 semver） |

### 1.4 术语定义

| 术语 | 定义 |
|---|---|
| ADR（Architecture Decision Record） | 架构决策记录，结构化记录背景/候选/决策/理由 |
| 编译时依赖（build dependency） | 仅编译时需要的依赖（如 protoc、bindgen） |
| 异步运行时（async runtime） | 异步任务的执行器（Tokio / async-std） |
| Tokio | Rust 主流异步运行时，单线程/多线程可配 |
| WASM | WebAssembly，浏览器中运行的二进制格式 |
| MSRV | Minimum Supported Rust Version，最小支持 Rust 版本 |
| SemVer | 语义化版本（major.minor.patch） |
| `unsafe` | Rust 中绕过编译器安全检查的代码，需手动保证正确性 |
| FFI | Foreign Function Interface，调用 C/C++ 库 |

### 1.5 关联文档

- 横向（项目内）：[CATs 技术选型书 v2.0](./CATs_技术选型书_v2.0.md)、[CATs 微服务架构设计书 v1.0](../架构设计/CATs_微服务架构设计书_v1.0.md)、[CATs 接口设计书 v2.0](../../03-详细设计/接口设计/CATs_接口设计书_v2.0.md)、[CATs 数据库设计书 v2.0](../../03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md)、[CATs 模块设计书 v2.0](../../03-详细设计/模块设计/CATs_模块设计书_v2.0.md)
- 外部参考：
  - [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
  - [The Cargo Book](https://doc.rust-lang.org/cargo/)
  - [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
  - [Rust Performance Book](https://nnethercote.github.io/perf-book/)
  - [Effective Rust](https://www.lurklurk.org/effective-rust/)

---

## 2. 选型总原则

| 原则 | 说明 | 评估权重 |
|---|---|---|
| 主流活跃 | 优先选择 Rust 生态主流、社区活跃的库 | 25% |
| 内存安全 | 默认选择不依赖 `unsafe` 或依赖极少 `unsafe` 的库 | 20% |
| 性能 | 同类库中性能优者（基准测试支持） | 20% |
| 可维护性 | 文档齐全、API 稳定、升级路径清晰 | 15% |
| 兼容性 | 与 CATs 其他选型（Tokio/serde/axum）兼容 | 10% |
| 团队熟悉度 | 团队已有经验或学习曲线平缓 | 5% |
| 许可证 | 兼容商用部署 | 5% |

**许可证要求**：MIT / Apache-2.0 / BSD 优先；GPL 仅用于不重新分发的内部工具；禁止 AGPL（如 MongoDB 驱动某些版本）。

---

## 3. 工具链

### 3.1 Rust 编译器与工具链

| 选型 | 决策 | 版本 | 理由 |
|---|---|---|---|
| Rust 稳定版（stable） | **采纳** | 1.83+ | 生产环境不用 nightly/beta |
| MSRV | **锁定 1.75+** | — | 让依赖库可使用 2024 edition 特性；CI 中 `cargo +1.75 check` 兜底 |
| Edition | **2021** | — | 2024 edition 仍部分库未支持，待生态稳定后评估 |
| 组件（rustup） | `rustc`, `cargo`, `clippy`, `rustfmt`, `rust-src`, `rust-analyzer` | 全部 stable 组件 | — |

**ADR-R-01 锁定 Rust 1.75+ MSRV**

- **背景**：CATs 项目周期 12+ 月，生态快速迭代，需要确定 MSRV
- **候选**：
  1. 1.75（保守）
  2. 1.80（中等）
  3. 不锁定（用最新稳定）
- **决策**：1.75
- **理由**：
  - 1.75 是 2024 年初的稳定版，覆盖所有核心需求（含 `let-else`、`impl Trait` 改进）
  - Tokio / axum / sqlx 主流 crate 都支持 1.75
  - 留 1-2 年升级缓冲（每半年升一次小版本）
- **取舍**：无法用 1.80+ 引入的少数新特性（影响小）

### 3.2 构建工具

| 选型 | 决策 | 版本 | 理由 |
|---|---|---|---|
| `cargo` | **采纳** | 1.83+ | Rust 官方，事实标准 |
| `cargo-make` | **可选** | 0.37+ | 复杂构建任务编排；CATs 主要用 Makefile + cargo |
| `just` | **备选** | 1.34+ | Makefile 替代，跨平台更友好 |
| `cargo-binstall` | **采纳** | 1.10+ | 快速安装预编译二进制（节省 CI 时间） |
| `cargo-nextest` | **采纳** | 0.9+ | 并行测试运行器，比 `cargo test` 快 2-3 倍 |

**CARGO 构建配置**（`.cargo/config.toml`）：

```toml
[build]
# 增量编译默认开
incremental = true

[target.x86_64-unknown-linux-gnu]
# 链接器优化
linker = "lld"
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

# 全局 codegen units 调整
[profile.dev]
opt-level = 0
debug = true
incremental = true

[profile.release]
opt-level = 3
lto = "fat"          # 链接时优化
codegen-units = 1    # 优化二进制大小/速度
strip = "symbols"    # 去除符号
debug = "line-tables-only"
```

### 3.3 静态分析与代码风格

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `clippy` | **强制** | 随 stable | Lint，CI 中 `cargo clippy --all-targets -- -D warnings` 阻断 |
| `rustfmt` | **强制** | 随 stable | 格式化，CI 中 `cargo fmt --check` 阻断 |
| `cargo-deny` | **强制** | 1.16+ | 依赖许可证/安全/重复检查 |
| `cargo-machete` | **采纳** | 0.7+ | 检测未使用的依赖 |

**Clippy 配置**（`clippy.toml`）：

```toml
# 复杂度阈值
cognitive-complexity-threshold = 25
too-many-arguments-threshold = 8
type-complexity-threshold = 250

# 风格
avoid-breaking-exported-api = true
```

### 3.4 测试工具

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `cargo test` | **强制** | — | Rust 内置测试运行 |
| `cargo-nextest` | **强制** | 0.9+ | 并行测试，CI 默认 |
| `mockall` | **强制** | 0.13+ | 通用 mock 库（自动 mock trait） |
| `wiremock` | **采纳** | 0.6+ | HTTP/gRPC mock server |
| `mockito` | **备选** | 1.5+ | HTTP mock（与 wiremock 互补） |
| `testcontainers-rs` | **强制** | 0.20+ | 真实 PG/Kafka 容器测试 |
| `proptest` | **采纳** | 1.5+ | 基于属性的测试（property-based） |
| `rstest` | **采纳** | 0.21+ | 参数化测试夹具（fixture） |
| `criterion` | **采纳** | 0.5+ | 性能基准（benchmark） |
| `tokio-test` | **采纳** | 0.4+ | 异步测试辅助 |
| `pretty_assertions` | **采纳** | 1.4+ | 漂亮的断言输出 |
| `wiremock` | **采纳** | 0.6+ | mock 外部 HTTP |

### 3.5 性能分析

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `cargo flamegraph` | **强制** | 0.6+ | 生成火焰图（基于 perf） |
| `perf` (Linux) | **强制** | — | Linux 性能采样 |
| `pprof` (Rust) | **采纳** | 0.13+ | CPU profile 输出 |
| `heaptrack` | **采纳** | 1.5+ | 堆内存分析 |
| `valgrind` / `callgrind` | **采纳** | 3.22+ | 内存错误 + 调用栈分析 |
| `cargo-bloat` | **采纳** | 0.11+ | 二进制大小分析 |

### 3.6 安全审计

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `cargo-audit` | **强制** | 0.21+ | 已知漏洞扫描（RustSec） |
| `cargo-deny` | **强制** | 1.16+ | 许可证/重复依赖检查 |
| `cargo-geiger` | **采纳** | 0.11+ | `unsafe` 代码使用统计 |
| `cargo-bloat` | **采纳** | 0.11+ | 体积/链接分析 |
| `semgrep` | **强制** | — | SAST（Python/Rust/TS 跨语言） |

**ADR-R-02 默认禁止 `unsafe` 代码**

- **背景**：Rust 安全保证是核心价值，但 `unsafe` 是生态不可避免的一部分（如 `Vec::push`、FFI）
- **决策**：默认 `cargo geiger` 检测 `unsafe` 数量
  - 库依赖中 `unsafe` 数量 > 阈值时需评估（不阻断）
  - **应用代码中 `unsafe` 必须有 `// SAFETY:` 注释 + code review 双签**
  - 内部封装的 `unsafe` 应集中到独立 crate（如 `cats-unsafe-ffi`）并 100% 覆盖测试

### 3.7 依赖管理

**ADR-R-03 依赖版本锁定策略**

- **决策**：
  - `Cargo.toml` 用 SemVer caret（`^1.0`）——允许 minor/patch 升级
  - 仓库内提交 `Cargo.lock` ——保证可复现构建
  - CI 中 `cargo update --dry-run` 每周 review 一次新版本
  - 重大升级（major）需 ADR + 评审
- **理由**：平衡灵活性与稳定性
- **取舍**：某些 crate 升级可能引入行为变更，需充分测试

---

## 4. 核心运行时

### 4.1 异步运行时

| 选型 | 决策 | 版本 | 理由 |
|---|---|---|---|
| `tokio` | **强制** | 1.40+ | Rust 异步运行时事实标准 |
| `async-std` | **不采用** | — | 维护放缓，生态远不如 Tokio |
| `smol` | **不采用** | — | 适合嵌入式/简单场景，不适合服务端 |
| `monoio` | **备选** | — | CloudWeGo 出品，io_uring 支持；如需极致性能可评估 |
| `glommio` | **备选** | — | 线程绑定的 io_uring 运行时；适合特定场景 |

**ADR-R-04 选 Tokio 作为唯一异步运行时**

- **背景**：CATs 服务都是 I/O 密集型（HTTP、DB、Kafka）
- **候选**：
  1. tokio（主流，生态最完整）
  2. async-std（API 友好但生态弱）
  3. monoio（io_uring 高性能）
- **决策**：tokio
- **理由**：
  - 生态最完整：axum、tonic、sqlx、reqwest、rdkafka-rs 全部基于 tokio
  - 文档齐全、性能稳定、生产验证充分
  - 团队熟悉度高
- **取舍**：相比 monoio，io_uring 利用较少（但 TCP 场景差距不大）

**Tokio 配置**：

```rust
// main.rs
#[tokio::main]
async fn main() -> Result<()> {
    // 启用所有 features
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(num_cpus::get())
        .thread_name("cats-worker")
        .thread_stack_size(2 * 1024 * 1024)  // 2MB stack
        .build()?
        .block_on(async_main())
        .await
}
```

### 4.2 并发原语

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `tokio::sync::Mutex/RwLock/Semaphore/Notify` | **强制** | — | 跨 async task 同步 |
| `parking_lot::Mutex/RwLock` | **强制** | 0.12+ | 同步锁（性能优于 std） |
| `arc-swap` | **采纳** | 1.7+ | 无锁读多写少（如配置） |
| `crossbeam` | **采纳** | 0.8+ | 高性能 channel/epoch |
| `dashmap` | **采纳** | 6.1+ | 高并发 HashMap |
| `tokio::task::JoinSet` | **强制** | — | 任务组管理 |
| `rayon` | **采纳** | 1.10+ | CPU 密集型并行（data parallelism） |

### 4.3 错误处理

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `thiserror` | **强制** | 1.0+ | 库/领域层定义结构化错误 |
| `anyhow` | **强制** | 1.0+ | 应用/胶水层简化错误传播 |
| `eyre` | **备选** | 0.6+ | anyhow fork，更好的报告 |

**ADR-R-05 thiserror 与 anyhow 分工**

- **背景**：Rust 错误处理有 2 个主流选择
- **决策**：
  - 库代码（领域逻辑、trait 实现）→ `thiserror`，定义显式错误枚举
  - 应用代码（main.rs、胶水层、CLI）→ `anyhow`，简化 `?` 传播
  - 公开 API（REST/gRPC handlers）→ 把 `thiserror` 错误映射到统一错误码（接口设计书 §1.4）
- **理由**：
  - 库需要明确错误种类（调用方可 match 处理）
  - 应用层只关心"出错 vs 成功"
  - 公开契约需要可控的错误信息（不泄露内部细节）

```rust
// 库层（用 thiserror）
#[derive(thiserror::Error, Debug)]
pub enum TranslationError {
    #[error("TM match failed: {0}")]
    TmMatchFailed(String),
    #[error("compliance blocked: project={project} requires local model")]
    ComplianceBlocked { project: String },
}

// 应用层（用 anyhow）
use anyhow::Result;
async fn main() -> Result<()> {
    let task = task_service.create(input).await?;  // 任意错误自动转换
    Ok(())
}
```

---

## 5. 网络与 API

### 5.1 HTTP 服务器框架

| 选型 | 决策 | 版本 | 理由 |
|---|---|---|---|
| `axum` | **强制** | 0.7+ | Tokio 团队官方，类型安全，中间件生态完整 |
| `actix-web` | **备选** | 4.9+ | 性能高，但生态与 Tokio 不完全兼容 |
| `warp` | **不采用** | 0.3+ | API 灵活但维护放缓 |
| `rocket` | **不采用** | 0.5+ | 同步为主，不适合微服务 |

**ADR-R-06 选 axum 作为 HTTP 服务框架**

- **背景**：所有 Rust 微服务（auth/user/project/task 等）需要 HTTP 框架
- **候选**：
  1. axum（Tokio 团队官方）
  2. actix-web（性能领先）
  3. tonic（仅 gRPC）
- **决策**：axum
- **理由**：
  - 与 tokio 完美集成（`tokio::main` + `axum::serve`）
  - tower 生态（超时/限流/熔断/trace）
  - 类型安全的路由（编译期校验）
  - 团队熟悉度高
- **取舍**：纯性能略低于 actix-web（差异 < 10%，业务场景可忽略）

**核心中间件**：

```toml
[dependencies]
axum = { version = "0.7", features = ["macros", "matched-path"] }
tower = { version = "0.5", features = ["util", "timeout", "limit"] }
tower-http = { version = "0.6", features = ["trace", "cors", "compression-gzip", "request-id", "util"] }
```

### 5.2 HTTP 客户端

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `reqwest` | **强制** | 0.12+ | 同步/异步 HTTP 客户端 |
| `ureq` | **备选** | 2.10+ | 轻量同步客户端（无需 tokio） |
| `isahc` | **不采用** | 1.7+ | curl 绑定，复杂场景备用 |

```rust
// 共享客户端
use once_cell::sync::Lazy;
static HTTP: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(5))
        .pool_idle_timeout(Duration::from_secs(60))
        .build()
        .unwrap()
});
```

### 5.3 gRPC

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `tonic` | **强制** | 0.12+ | gRPC 客户端/服务器（基于 hyper） |
| `tonic-build` | **强制** | 0.12+ | Protobuf → Rust 代码生成 |
| `prost` | **强制** | 0.13+ | Protobuf 运行时（tonic 依赖） |
| `prost-types` | **强制** | 0.13+ | Protobuf 标准类型 |
| `tonic-reflection` | **采纳** | 0.12+ | gRPC reflection（调试用） |
| `tonic-health` | **采纳** | 0.12+ | gRPC health checking |

**ADR-R-07 选 tonic 作为 gRPC 实现**

- **背景**：CATs 内部服务间同步调用走 gRPC（接口设计书 §3.1/§3.3/§3.10）
- **候选**：
  1. tonic（hyper + prost）
  2. grpc-rs（基于 C 绑定）
  3. 自研（不推荐）
- **决策**：tonic
- **理由**：
  - 纯 Rust 实现，无 FFI 依赖
  - 与 axum 共享 tower 中间件
  - 支持 HTTP/2、streaming、reflection
  - 活跃维护、性能领先
- **取舍**：某些高级特性（如 metadata 扩展）需要 workaround

### 5.4 WebSocket

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `tokio-tungstenite` | **强制** | 0.24+ | WebSocket 客户端/服务器 |
| `axum::extract::ws` | **强制** | 0.7+ | axum 内置 WebSocket 支持 |
| `tungstenite` | **采纳** | 0.24+ | 底层库（tokio-tungstenite 依赖） |
| `async-tungstenite` | **不采用** | 0.28+ | 通用异步（不如 tokio-tungstenite 优化） |

### 5.5 Tauri 框架（客户端）

| 选型 | 决策 | 版本 | 理由 |
|---|---|---|---|
| `tauri` | **强制** | 2.1+ | 桌面客户端框架 |
| `tauri-build` | **强制** | 2.0+ | 构建时支持 |
| `tauri-plugin-*` | **强制** | 2.0+ | 官方插件（fs / http / dialog / shell / store 等） |
| `tao` | 间接 | — | 窗口管理（tauri 内部） |
| `wry` | 间接 | — | WebView 抽象（tauri 内部） |
| `keyring` | **强制** | 3.6+ | OS 密钥库集成（Token 安全存储） |

**ADR-R-08 选 Tauri 2.x（详见技术选型书 v2.0 ADR-14）**

补充：
- 用 `tauri::generate_handler!` 注册命令
- 前端用 `invoke()` / `Channel` 与 Rust 核心通信
- WebView2 必须在 Windows 打包时内嵌（局域网离线场景）

### 5.6 中间件（tower）

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `tower` | **强制** | 0.5+ | 中间件抽象 |
| `tower-http` | **强制** | 0.6+ | HTTP 专用中间件（trace / cors / compression / request-id） |
| `tower::limit::RateLimitLayer` | **强制** | — | 限流 |
| `tower::timeout::TimeoutLayer` | **强制** | — | 超时 |
| `tower::load_shed` | **采纳** | — | 过载保护 |
| `tower::buffer` | **采纳** | — | 请求缓冲 |

---

## 6. 数据持久化

### 6.1 PostgreSQL 驱动

| 选型 | 决策 | 版本 | 理由 |
|---|---|---|---|
| `sqlx` | **强制** | 0.8+ | 异步、编译时 SQL 校验、原生 tokio 集成 |
| `tokio-postgres` | **采纳** | 0.7+ | 底层驱动（sqlx 依赖） |
| `diesel` | **不采用** | 2.2+ | 同步为主，宏 DSL 学习曲线 |
| `sea-orm` | **不采用** | 1.1+ | 基于 sqlx 的 ORM，复杂场景不灵活 |
| `deadpool-postgres` | **强制** | 0.14+ | 连接池（pgbouncer 后端的客户端池） |

**ADR-R-09 选 sqlx 作为 PostgreSQL 驱动**

- **背景**：CATs 唯一权威存储是 PostgreSQL（架构设计书 §1.2）
- **候选**：
  1. sqlx（async + 编译时 SQL 校验）
  2. diesel（同步 + 宏 DSL）
  3. sea-orm（async + ORM）
- **决策**：sqlx
- **理由**：
  - 异步原生，匹配 tokio 生态
  - `query!` / `query_as!` 宏编译时校验 SQL（重大优势：bug 提早发现）
  - 不强制宏，可 `query()` 动态 SQL
  - 文档齐全、社区活跃
- **取舍**：相比 ORM，关联查询要手写 SQL

**关键使用模式**：

```rust
// 编译时校验
let user = sqlx::query_as!(
    User,
    r#"SELECT id, email, org_id, status FROM users_credential WHERE id = $1"#,
    user_id
)
.fetch_one(&pool)
.await?;

// 动态 SQL
let sql = format!("SELECT * FROM tasks WHERE project_id = $1 AND status = ANY($2)");
sqlx::query_as::<_, Task>(&sql)
    .bind(project_id)
    .bind(&statuses)
    .fetch_all(&pool)
    .await?;
```

### 6.2 迁移工具

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `sqlx-migrate` | **强制** | 0.8+ | 与 sqlx 配套的迁移子命令 |
| `refinery` | **备选** | 0.8+ | 独立迁移库（支持多 DB） |

```toml
# Cargo.toml
[[bin]]
name = "cats-auth-migrate"
path = "src/bin/migrate.rs"

[dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "postgres", "migrate"] }
```

### 6.3 连接池

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `sqlx::PgPool` | **强制** | 0.8+ | sqlx 内置池 |
| `deadpool-postgres` | **采纳** | 0.14+ | 独立池（PgBouncer 透传场景） |
| `bb8-postgres` | **备选** | 0.8+ | 备选池实现 |
| `r2d2-postgres` | **不采用** | 0.18+ | 同步池，不适合 tokio |

### 6.4 对象存储 SDK

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `aws-sdk-s3` | **强制** | 1.55+ | 通用 S3 兼容（MinIO 兼容） |
| `s3` | **备选** | 0.13+ | 轻量 S3 客户端 |

```rust
// MinIO 接入
let s3_config = aws_config::defaults(BehaviorVersion::latest())
    .endpoint_url("http://minio.cats.internal:9000")  // MinIO endpoint
    .region("us-east-1")
    .credentials_provider(/* MinIO 凭据 */)
    .load().await;
let client = aws_sdk_s3::Client::new(&s3_config);
```

### 6.5 缓存客户端

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `redis-rs` | **强制** | 0.27+ | 通用 Redis/Valkey 客户端（支持集群/cluster/sentinel） |
| `deadpool-redis` | **强制** | 0.18+ | 异步连接池 |
| `bb8-redis` | **备选** | 0.17+ | 备选池 |
| `fred` | **备选** | 9.x | 高性能集群客户端 |

**ADR-R-10 redis-rs + deadpool-redis 作为 Valkey 客户端**

- **背景**：CATs 缓存层是 Valkey（Redis 协议）
- **决策**：redis-rs + deadpool-redis
- **理由**：
  - redis-rs 是 Rust 生态最广泛使用的客户端
  - deadpool-redis 提供稳定异步连接池
  - 支持 Cluster / Sentinel / TLS

---

## 7. 消息系统

### 7.1 Kafka 客户端

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `rdkafka` | **强制** | 0.36+ | C 绑定 librdkafka，功能最完整 |
| `kafka-rust` | **备选** | 0.10+ | 纯 Rust 实现（功能不完整） |
| `rskafka` | **备选** | 0.5+ | 纯 Rust 异步 |

**ADR-R-11 选 rdkafka 作为 Kafka 客户端**

- **背景**：CATs 事件总线是 Kafka（架构设计书 §6）
- **候选**：
  1. rdkafka（C 绑定）
  2. kafka-rust（纯 Rust）
  3. rskafka（纯 Rust 异步）
- **决策**：rdkafka
- **理由**：
  - librdkafka 是 Confluent 维护的工业级 C 库，性能/可靠性领先
  - 完整支持 SASL/SSL/ACL/事务
  - 与 Kafka 所有高级特性兼容
- **取舍**：需要安装 librdkafka 系统库（Docker 镜像中 apt-get install librdkafka-dev）

```toml
[dependencies]
rdkafka = { version = "0.36", features = ["cmake-build", "ssl-vendored", "sasl"] }
```

**核心模式**：

```rust
// Producer
let producer: FutureProducer = ClientConfig::new()
    .set("bootstrap.servers", "kafka.cats.internal:9092")
    .set("message.timeout.ms", "5000")
    .set("compression.type", "zstd")
    .set("enable.idempotence", "true")
    .create()?;

producer.send(
    FutureRecord::to("task.task.created.v1")
        .key(&task_id)
        .payload(&payload),
    Duration::from_secs(0),
).await?;

// Consumer
let consumer: StreamConsumer = ClientConfig::new()
    .set("group.id", "asr-service")
    .set("bootstrap.servers", "kafka.cats.internal:9092")
    .set("enable.auto.commit", "false")  // 手动 commit（at-least-once + 幂等）
    .set("auto.offset.reset", "earliest")
    .create()?;
```

### 7.2 消息序列化

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `serde_json` | **强制** | 1.0+ | JSON（事件 schema 默认格式） |
| `prost` | **强制** | 0.13+ | Protobuf（gRPC + 内部高效序列化） |
| `rmp-serde` | **备选** | 1.3+ | MessagePack（紧凑） |
| `bincode` | **备选** | 2.0+ | 二进制（紧凑、无 schema） |

---

## 8. 序列化与数据

### 8.1 JSON / 一般序列化

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `serde` | **强制** | 1.0+ | 序列化框架事实标准 |
| `serde_json` | **强制** | 1.0+ | JSON 格式 |
| `serde_yaml` | **采纳** | 0.9+ | YAML（配置文件） |
| `serde_with` | **强制** | 3.11+ | 自定义序列化辅助（如 ISO8601） |
| `simd-json` | **采纳** | 0.13+ | 高性能 JSON（生产可选） |

### 8.2 Protobuf

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `prost` | **强制** | 0.13+ | Protobuf runtime（tonic 依赖） |
| `prost-build` | **强制** | 0.13+ | Protobuf → Rust 代码生成 |
| `tonic-build` | **强制** | 0.12+ | gRPC 服务代码生成（包装 prost-build） |

```rust
// build.rs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["../proto/cats/auth/v1/auth.proto"], &["../proto"])?;
    Ok(())
}
```

### 8.3 数据验证

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `validator` | **强制** | 0.18+ | derive 宏 + 类型化校验 |
| `garde` | **备选** | 0.20+ | 类似 validator，API 更现代 |
| `url` | **强制** | 2.5+ | URL 类型 |
| `uuid` | **强制** | 1.10+ | UUID v4/v7 |
| `chrono` / `time` | **强制** | 0.4+ | 时间处理（见 §13） |

```rust
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateTaskRequest {
    #[validate(length(min = 1, max = 100))]
    pub project_id: String,
    #[validate(length(min = 2, max = 16))]
    pub source_lang: String,
    #[validate(length(min = 2, max = 16))]
    pub target_lang: String,
    #[validate(url)]
    pub source_url: Option<String>,
}

request.validate()?;
```

---

## 9. 可观测性

### 9.1 日志

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `tracing` | **强制** | 0.1+ | 结构化日志 + span 追踪 |
| `tracing-subscriber` | **强制** | 0.3+ | subscriber 实现 |
| `tracing-bunyan-formatter` | **采纳** | 0.3+ | Bunyan 风格 JSON 输出 |
| `tracing-log` | **采纳** | 0.2+ | log → tracing 桥接 |
| `log` | **不推荐（用 tracing）** | — | 旧式日志框架 |

**ADR-R-12 选 tracing 作为统一日志框架**

- **背景**：CATs 需要结构化日志 + 与 OTel Trace 关联
- **决策**：tracing（替代 log）
- **理由**：
  - 原生支持 span（嵌套追踪）
  - 与 OpenTelemetry 无缝集成（`tracing-opentelemetry`）
  - 结构化字段（key=value）
  - 性能优于 log（编译时过滤）
- **取舍**：部分老库用 log，需要 `tracing-log` 桥接

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(pool))]
async fn create_task(pool: &PgPool, input: CreateTaskInput) -> Result<TaskId> {
    info!(project_id = %input.project_id, "creating task");
    let task_id = sqlx::query_scalar!(/* ... */).fetch_one(pool).await?;
    Ok(task_id)
}
```

### 9.2 分布式追踪

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `opentelemetry` | **强制** | 0.27+ | OTel API |
| `opentelemetry_sdk` | **强制** | 0.27+ | OTel SDK |
| `opentelemetry-otlp` | **强制** | 0.27+ | OTLP 导出器（gRPC/HTTP） |
| `tracing-opentelemetry` | **强制** | 0.28+ | tracing → OTel 桥接 |
| `opentelemetry-semantic-conventions` | **强制** | 0.27+ | OTel 语义约定 |

```rust
use opentelemetry::trace::TracerProvider as _;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

opentelemetry::global::set_text_map_propagator(
    TraceContextPropagator::new()
);

// 初始化 OTLP exporter
let tracer = opentelemetry_otlp::new_pipeline()
    .tracing()
    .with_exporter(opentelemetry_otlp::new_exporter().tonic().with_endpoint("http://tempo:4317"))
    .with_trace_config(trace::Config::default().with_resource(Resource::new(vec![
        KeyValue::new("service.name", "translation-core"),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
    ])))
    .install_batch(opentelemetry_sdk::runtime::Tokio)?;
```

### 9.3 指标

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `metrics` | **强制** | 0.23+ | 抽象指标 API |
| `metrics-exporter-prometheus` | **强制** | 0.15+ | Prometheus 导出 |
| `axum-prometheus` | **采纳** | 0.7+ | axum HTTP 指标自动埋点 |
| `autometrics` | **采纳** | 1.0+ | 函数级指标宏 |

```rust
use metrics::{counter, gauge, histogram, describe_counter, describe_histogram};

describe_histogram!("translation_duration_seconds", "Translation duration");
histogram!("translation_duration_seconds", "mode" => "l2").record(elapsed_secs);
```

---

## 10. 错误处理（已部分在 §4.3 详述）

补充：
- 公开 API（REST/gRPC）错误码统一映射（接口设计书 §1.4）
- 错误必须含 `trace_id` 字段（便于跨服务追踪）
- `Error::source()` 链式追踪内部错误

```rust
#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("validation error: {0}")]
    Validation(#[source] ValidationError),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("compliance blocked: {project}")]
    ComplianceBlocked { project: String, trace_id: String },
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            ApiError::Validation(_) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR"),
            ApiError::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            ApiError::ComplianceBlocked { .. } => (StatusCode::CONFLICT, "COMPLIANCE_BLOCKED"),
        };
        (status, Json(json!({
            "error": {
                "code": code,
                "message": self.to_string(),
                "trace_id": /* from current span */,
            }
        }))).into_response()
    }
}
```

---

## 11. 配置管理

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `figment` | **强制** | 0.10+ | 多源配置合并（env + file + 默认） |
| `config` | **备选** | 0.14+ | 老牌配置库 |
| `dotenvy` | **采纳** | 0.15+ | .env 文件加载 |
| `secrecy` | **强制** | 0.8+ | 敏感值包装（防误打印） |
| `envy` | **采纳** | 0.4+ | 环境变量 → 结构体 |

```rust
use figment::{Figment, providers::{Format, Toml, Env, Serialized}};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database_url: Secret<String>,
    pub kafka_brokers: Vec<String>,
    pub server_port: u16,
    pub log_level: String,
}

impl Config {
    pub fn load() -> Result<Self, figment::Error> {
        Figment::new()
            .merge(Serialized::defaults(serde_json::json!({
                "server_port": 8080,
                "log_level": "info",
            })))
            .merge(Toml::file("config/default.toml"))
            .merge(Toml::file(format!("config/{}.toml", env::var("ENV").unwrap_or_default())))
            .merge(Env::prefixed("CATS_"))
            .extract()
    }
}
```

---

## 12. 安全与加密

### 12.1 JWT / OAuth

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `jsonwebtoken` | **强制** | 9.3+ | JWT 签发/校验（HS256/RS256/ES256） |
| `oauth2` | **采纳** | 5.0+ | OAuth 2.0 客户端 |
| `openidconnect` | **采纳** | 3.5+ | OpenID Connect |

```rust
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation};

let token = encode(
    &Header::new(Algorithm::RS256),
    &claims,
    &EncodingKey::from_rsa_pem(include_bytes!("private.pem"))?,
)?;
```

### 12.2 加密 / 哈希

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `argon2` | **强制** | 0.5+ | 密码哈希（OWASP 推荐） |
| `ring` | **强制** | 0.17+ | 通用加密原语（AES/SHA/HMAC/RSA/ECDSA） |
| `aws-lc-rs` | **备选** | 1.10+ | ring 替代（AWS 维护） |
| `rustls` | **强制** | 0.23+ | TLS 实现（替代 OpenSSL） |
| `openssl` | **不推荐** | — | 仅在 librdkafka 等 C 库透传时用 |

**ADR-R-13 选 rustls 作为 TLS 实现**

- **决策**：rustls（替代 OpenSSL）
- **理由**：
  - 纯 Rust，内存安全（无 C 漏洞面）
  - Apache-2.0 / ISC 许可
  - 性能与 OpenSSL 相当
  - 主流云原生项目（Linkerd、Consul）已采用
- **取舍**：某些老旧 CipherSuite 不支持（TLS 1.0/1.1）

### 12.3 密钥管理

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `keyring` | **强制** | 3.6+ | OS 密钥库（Credential Manager / Keychain / Secret Service） |
| `secrecy` | **强制** | 0.8+ | 内存中敏感值包装 |
| `zeroize` | **强制** | 1.8+ | 内存清零（防止 swap 泄露） |

```rust
use secrecy::{Secret, ExposeSecret};
use zeroize::Zeroize;

pub struct DbCredentials {
    pub username: String,
    pub password: Secret<String>,  // 不会自动 Debug/Display
}

// 使用
let conn = connect(&cfg.database_url.expose_secret()).await?;
```

---

## 13. 时间处理

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `time` | **强制** | 0.3+ | 现代时间库（编译期类型化） |
| `chrono` | **备选** | 0.4+ | 成熟但 API 不够安全 |
| `timeago` | **采纳** | 0.4+ | 相对时间显示 |
| `cron` | **强制** | 0.12+ | Cron 表达式解析（worker-service 调度） |

**ADR-R-14 选 time 作为时间库**

- **决策**：`time` 而非 `chrono`
- **理由**：
  - `time` 编译期类型化（`OffsetDateTime` vs `NaiveDateTime`）
  - 性能优于 chrono
  - API 更现代（trait-based formatting）
- **取舍**：生态迁移成本（多数教程用 chrono）

```rust
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

let now = OffsetDateTime::now_utc();
let formatted = now.format(&Rfc3339)?;  // "2026-08-19T09:00:00Z"
```

---

## 14. 跨平台

### 14.1 操作系统支持

| 平台 | 客户端（Tauri） | 服务端 | 备注 |
|---|---|---|---|
| Windows 10/11 | ✓ | ✓ | 优先支持；WebView2 内嵌 |
| macOS 12+ (M1+Intel) | ✓ | ✓ | WKWebView |
| Ubuntu 22.04 LTS | ✓ | ✓ | glibc 2.35+ |
| Fedora 最新 | ✓ | ✓ | 滚动更新，跟踪测试 |
| Alpine | × | ✗ | musl 工具链支持但不主推（K8s 用 scratch/distroless） |

### 14.2 容器化

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| 多阶段构建 | **强制** | — | builder + runtime 镜像 |
| distroless/cc-debian12 | **强制** | — | 生产运行时基础镜像 |
| scratch | **备选** | — | 静态二进制场景 |
| Debian 12 slim | **备选** | — | 需要 glibc 兼容性 |

**Dockerfile 模板**（Rust 服务）：

```dockerfile
# ============ Builder ============
FROM rust:1.83-slim-bookworm AS builder
WORKDIR /build
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev librdkafka-dev cmake build-essential && rm -rf /var/lib/apt/lists/*

# Layer 缓存：先仅复制依赖描述
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src target/release/deps/cats_*

# 实际构建
COPY src ./src
RUN cargo build --release --locked

# ============ Runtime ============
FROM gcr.io/distroless/cc-debian12:nonroot
WORKDIR /app
COPY --from=builder /build/target/release/cats-service /app/cats-service
COPY config /app/config
USER nonroot:nonroot
EXPOSE 8080
ENTRYPOINT ["/app/cats-service"]
```

### 14.3 静态二进制（特殊场景）

```bash
# 完全静态二进制（Alpine / scratch）
RUSTFLAGS="-C target-feature=+crt-static" cargo build --release --target x86_64-unknown-linux-musl
```

---

## 15. Kubernetes 集成

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `kube` | **强制** | 0.95+ | K8s 客户端（Rust operator/客户端） |
| `k8s-openapi` | **强制** | 0.22+ | K8s OpenAPI 类型 |
| `bollard` | **采纳** | 0.15+ | Docker API 客户端 |

**用途场景**：
- 自研 K8s Operator（如 Feature Bundle Operator）
- 运维工具（CLI/服务）通过 K8s API 操作集群
- 测试环境用 bollard 起测试容器

---

## 16. 媒体处理

### 16.1 ffmpeg 绑定

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `ffmpeg-next` | **强制** | 7.1+ | FFmpeg Rust 绑定（视频/音频解码编码） |
| `ffmpeg-sidecar` | **备选** | — | 通过 sidecar 进程方式（更安全） |

**决策**：`ffmpeg-next` 在 `render-writer-service` 中使用；其他服务不直接调用 ffmpeg，而是经 `render-writer-service` 或子进程调用。

```rust
use ffmpeg_next as ffmpeg;

ffmpeg::init().unwrap();
let mut ictx = ffmpeg::format::input(&path)?;
let stream = ictx.streams().best(ffmpeg::media::Type::Video).unwrap();
```

### 16.2 图像处理

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `image` | **强制** | 0.25+ | 图像处理（PIL 替代） |
| `gif` | **采纳** | 0.13+ | GIF 解码 |
| `webp` | **采纳** | 0.3+ | WebP 解码 |

### 16.3 PDF 处理

| 选型 | 决策 | 版本 | 用途 |
|---|---|---|---|
| `lopdf` | **采纳** | 0.34+ | PDF 解析与生成 |
| `pdf` | **备选** | 0.14+ | 高层 PDF API |

**注**：CATs 中 PDF 处理主要在 Python 服务（PyMuPDF），Rust 端仅做轻量操作；如需纯 Rust 流程可扩展。

---

## 17. 关键依赖版本锁定

| Crate | 版本 | 用途 | 锁定期 |
|---|---|---|---|
| tokio | 1.40+ | 异步运行时 | 6 月（minor） |
| axum | 0.7+ | Web 框架 | 6 月 |
| tonic | 0.12+ | gRPC | 6 月 |
| sqlx | 0.8+ | 数据库 | 6 月 |
| rdkafka | 0.36+ | Kafka | 6 月 |
| serde | 1.0+ | 序列化 | 永久（1.x） |
| prost | 0.13+ | Protobuf | 12 月 |
| tracing | 0.1+ | 日志/追踪 | 12 月 |
| opentelemetry | 0.27+ | OTel | 6 月 |
| reqwest | 0.12+ | HTTP 客户端 | 6 月 |
| redis | 0.27+ | 缓存 | 6 月 |
| thiserror | 1.0+ | 错误处理 | 永久（1.x） |
| anyhow | 1.0+ | 错误处理 | 永久（1.x） |
| validator | 0.18+ | 数据验证 | 12 月 |
| jsonwebtoken | 9.3+ | JWT | 12 月 |
| argon2 | 0.5+ | 密码哈希 | 永久（0.x 稳定） |
| ring | 0.17+ | 加密 | 永久 |
| rustls | 0.23+ | TLS | 6 月 |
| tauri | 2.1+ | 桌面框架 | 6 月 |
| criterion | 0.5+ | 基准 | 12 月 |
| proptest | 1.5+ | 属性测试 | 12 月 |
| mockall | 0.13+ | mock | 12 月 |
| testcontainers | 0.20+ | 容器测试 | 6 月 |

---

## 18. 风险与缓解

| 编号 | 风险 | 概率 | 影响 | 缓解措施 |
|---|---|---|---|---|
| RUST-01 | Tokio 升级引入行为变更 | 中 | 高 | 锁 MSRV + 6 月升级窗口 + 充分测试 |
| RUST-02 | `unsafe` 误用导致内存安全问题 | 低 | 极高 | `cargo geiger` 检测；`unsafe` 集中到独立 crate + 100% 测试覆盖 |
| RUST-03 | C 依赖（librdkafka）系统兼容问题 | 中 | 中 | Docker 镜像固定系统库版本；CI 多 OS 测试 |
| RUST-04 | 生态快速迭代导致依赖频繁 breaking | 中 | 中 | SemVer caret 锁 + 6 月 review + ADR for major 升级 |
| RUST-05 | 编译时间长影响 CI 效率 | 高 | 中 | cargo-nextest 并行 + sccache 缓存 + Docker layer 缓存 |
| RUST-06 | 团队 Rust 熟练度不足 | 中 | 中 | 内部培训（书籍：[Rust Book](https://doc.rust-lang.org/book/)）+ code review 双签 + 资深人员 review |
| RUST-07 | 异步测试不稳定（flaky） | 中 | 中 | `tokio-test` + 超时控制 + CI 重试 1 次 + flaky 标记 |
| RUST-08 | Tauri 跨平台兼容性（尤其 Linux） | 中 | 中 | 3 OS 平台持续集成 + 兼容性测试矩阵 |
| RUST-09 | sqlx 编译时 SQL 校验在 CI 与本地 DATABASE_URL 不匹配时失败 | 中 | 低 | `.env` 模板 + CI 注入 + 文档 |
| RUST-10 | 性能退化难以发现 | 中 | 中 | criterion 基准 + cargo flamegraph + 性能基线对比（见测试设计书 §4.5） |

---

## 19. 选型总结表

### 19.1 必选依赖

| 类别 | Crate | 版本 | 替代方案（不采用） |
|---|---|---|---|
| 异步运行时 | tokio | 1.40+ | async-std / smol / monoio |
| Web 框架（服务） | axum | 0.7+ | actix-web / warp / rocket |
| Web 框架（客户端） | tauri | 2.1+ | — |
| HTTP 客户端 | reqwest | 0.12+ | ureq / isahc |
| gRPC | tonic | 0.12+ | grpc-rs |
| WebSocket | tokio-tungstenite | 0.24+ | async-tungstenite |
| 数据库 | sqlx | 0.8+ | diesel / sea-orm |
| 迁移 | sqlx-migrate | 0.8+ | refinery |
| Kafka | rdkafka | 0.36+ | kafka-rust / rskafka |
| 缓存 | redis | 0.27+ | — |
| 缓存池 | deadpool-redis | 0.18+ | bb8-redis |
| S3 | aws-sdk-s3 | 1.55+ | s3 |
| 序列化 | serde | 1.0+ | — |
| JSON | serde_json | 1.0+ | simd-json |
| Protobuf | prost | 0.13+ | — |
| 验证 | validator | 0.18+ | garde |
| 错误（库） | thiserror | 1.0+ | snafu |
| 错误（应用） | anyhow | 1.0+ | eyre |
| 日志 | tracing | 0.1+ | log |
| 追踪 | opentelemetry | 0.27+ | — |
| 指标 | metrics | 0.23+ | prometheus（client_rust） |
| JWT | jsonwebtoken | 9.3+ | — |
| 密码 | argon2 | 0.5+ | bcrypt |
| 加密 | ring | 0.17+ | aws-lc-rs / openssl |
| TLS | rustls | 0.23+ | openssl / native-tls |
| 密钥 | keyring | 3.6+ | — |
| 敏感 | secrecy | 0.8+ | — |
| 清零 | zeroize | 1.8+ | — |
| 时间 | time | 0.3+ | chrono |
| Cron | cron | 0.12+ | — |
| 配置 | figment | 0.10+ | config / dotenv |
| 并行 | rayon | 1.10+ | — |
| HashMap | dashmap | 6.1+ | — |
| 锁 | parking_lot | 0.12+ | — |
| K8s | kube | 0.95+ | — |
| FFmpeg | ffmpeg-next | 7.1+ | ffmpeg-sidecar |
| 图像 | image | 0.25+ | — |
| 测试 | cargo-nextest | 0.9+ | cargo test |
| Mock | mockall | 0.13+ | — |
| 容器测试 | testcontainers | 0.20+ | — |
| 属性测试 | proptest | 1.5+ | — |
| 基准 | criterion | 0.5+ | — |
| 火焰图 | cargo-flamegraph | 0.6+ | — |
| 审计 | cargo-audit | 0.21+ | — |
| 许可证 | cargo-deny | 1.16+ | — |

### 19.2 可选依赖

| 类别 | Crate | 版本 | 用途 |
|---|---|---|---|
| HTTP mock | wiremock | 0.6+ | HTTP 集成测试 |
| 测试断言 | pretty_assertions | 1.4+ | 友好错误输出 |
| 测试夹具 | rstest | 0.21+ | 参数化测试 |
| 异步测试 | tokio-test | 0.4+ | 异步测试辅助 |
| 监控 | axum-prometheus | 0.7+ | axum HTTP 指标 |
| 函数级指标 | autometrics | 1.0+ | 自动埋点 |
| OpenFeature | openfeature-rs | 0.2+ | 特性开关 |
| OAuth | oauth2 / openidconnect | 5.0+ / 3.5+ | OAuth/OIDC 客户端 |
| WebP | webp | 0.3+ | WebP 解码 |
| GIF | gif | 0.13+ | GIF 解码 |
| PDF | lopdf | 0.34+ | PDF 处理 |
| Docker | bollard | 0.15+ | Docker API 客户端 |
| 模板 | tera / handlebars | 1.20+ / — | 通知模板 |
| HTTP 客户端(轻) | ureq | 2.10+ | 同步 HTTP（无需 tokio） |

### 19.3 不采用的依赖

| Crate | 不采用原因 |
|---|---|
| `diesel` | 同步为主，不适合异步微服务 |
| `sea-orm` | 复杂抽象，灵活性不足 |
| `async-std` | 维护放缓 |
| `smol` | 适合嵌入式/简单场景 |
| `actix-web` | 与 Tokio 不完全兼容 |
| `warp` | 维护放缓 |
| `rocket` | 同步为主 |
| `kafka-rust` | 功能不完整 |
| `rskafka` | 生态较小 |
| `chrono` | API 不够安全（time 更现代） |
| `log` | tracing 更现代 |
| `anyhow::Error` 在公开 API | 信息泄露（应用层用即可） |
| `tokio::spawn_blocking` 滥用 | 阻塞 tokio 调度 |
| `lazy_static` | `OnceCell` 已稳定，优先用 |
| `dotenv` | `dotenvy` 是维护 fork |
| `serde_derive` 自定义序列化 | `serde_with` 提供更安全的方案 |

---

## 20. 引用与参考

- **Rust 官方**：[The Rust Programming Language](https://doc.rust-lang.org/book/)、[Cargo Book](https://doc.rust-lang.org/cargo/)、[Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- **Rust 性能**：[The Rust Performance Book](https://nnethercote.github.io/perf-book/)、[Criterion.rs](https://github.com/bheisler/criterion.rs)
- **Tokio**：[Tokio Tutorial](https://tokio.rs/tokio/tutorial)、[Async Rust in Production](https://www.lurklurk.org/effective-rust/)
- **Axum**：[Axum Docs](https://docs.rs/axum/latest/axum/)
- **Tonic**：[Tonic gRPC](https://github.com/hyperium/tonic)
- **sqlx**：[sqlx Docs](https://docs.rs/sqlx/latest/sqlx/)
- **rdkafka**：[rdkafka-rs](https://github.com/fede1024/rust-rdkafka)
- **OpenTelemetry**：[OTel Rust SDK](https://github.com/open-telemetry/opentelemetry-rust)
- **OpenFeature**：[OpenFeature SDK](https://openfeature.dev/)

---

> **本书的下一步**: 评审通过后，① 锁定 CI 镜像与依赖版本（写入 `.cargo/config.toml` 与 `Cargo.lock`）；② 培训团队关键库的使用（Tokio/Axum/sqlx/tonic/OTel）；③ 与现有 `CATs_技术选型书_v2.0` 交叉引用，补充 v2.0 中 Rust 相关的 ADR；④ 落地 §17 依赖版本锁定表为 Renovate 自动 PR 规则。
