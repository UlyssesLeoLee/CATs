# CATs Backend 启动状态报告 v0.2

**日期**: 2026-10-03
**作者**: 架构师(Mavis 接手 agent per DEC-008)
**取代**: `deploy/BACKEND_STATUS_v0.1.md`（2026-09-19）— v0.1 全文保留，未修改
**基线 commit**: `682dd67`（`integrate/ci-revival-dev-ff`，PR #19 → `dev`）
**状态**: 🟡 binary 编译通过 + 485 测试 0 失败 + 6/7 CI workflow 绿；镜像构建与 release 门禁在本版修复后**尚待 CI 复验**

---

## §1 对 v0.1 结论的更正（重要）

v0.1 把"12 service binary 编译未通过"归因为**工具链缺陷**：

> ### Rustc 1.98 metadata bug
> 跨 12 service workspace 触发 rustc 1.98 在 Windows 上的已知 metadata 撞锁问题
> **真正解决**: rustc 2.x(超出 Sprint 2 范围,留 V2)

**这个归因是错的。** 当时的诊断把大量真实的代码缺陷误读成了编译器的偶发故障，
并据此把"需要 rustc 2.x"写成了结论。实际情况：

- 同样的 rustc 1.98.0 工具链今天编译整个 workspace **完全通过**；
- 失败的真实原因是代码本身写错了，共 8 类（清单见 §3）。其中第 6、7 类
  （Docker 漏 `proto/`、`new_with_buffer` 的 `#[cfg]` 逃逸）只在 **release
  profile** 下暴露——而当时 CI 的任何门禁都不构建 release，所以它们既没被
  修掉，也没人发现。
- 没有任何一条错误信息指向 metadata 锁。v0.1 引用的
  `E0463: can't find crate for std` / `only metadata stub found for rlib` 是
  这些次生错误的连锁表象，不是根因。

> 影响：v0.1 据此提出的"等 rustc 2.x 再试"这条 Sprint 3 路径**不成立**，
> 无需等待。本报告作废该建议。

---

## §2 当前实测状态（全部有 CI 证据）

### §2.1 编译与静态检查

| 门禁 | 命令 | 结果 | 证据 |
|---|---|---|---|
| clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | ✅ 通过 | CI run `37029287481`，ubuntu / macos / windows **三平台均 success** |
| 格式 | `cargo fmt --all --check` | ✅ 通过 | CI run `37028170031` |
| 依赖许可 | cargo-deny | ✅ 通过 | CI run `37029287446` |
| 构建 | `cargo build --workspace --all-features --locked` | ✅ 通过 | CI run `37029287581` |

本地（Windows，Rust 1.98.0）独立复核同样通过，`CARGO_TARGET_DIR` 隔离为
`E:\DevCache\cargo\target-cats` 避免与机器上其他项目的 cargo 争用。

> **覆盖范围警告**：以上门禁只覆盖 `Cargo.toml` workspace members 内的 crate。
> `apps/cats-client`（Tauri 客户端）**不在 workspace 内**，因此一条都不覆盖。
> 详见 §4.1。

### §2.2 测试

`cargo test --workspace --all-features --locked -- --nocapture`

| 平台 | passed | failed | ignored | 说明 |
|---|---|---|---|---|
| macos-latest | 485 | **0** | 56 | 单次口径 |
| windows-latest | 485 | **0** | 56 | 单次口径 |
| ubuntu-latest | 1235 | **0** | 146 | 含 `cargo llvm-cov` 重跑全部测试，故计数约为两倍 |

**采用单次口径：485 passed / 0 failed / 56 ignored。**

56 个 ignored 已全部定位，无一遗留：

- **45 个**是本次标注的 `#[ignore = "e2e-needs-real-pg: ..."]`，分布在 6 个文件
  （auth-service `e2e_auth.rs` 8 + `e2e_t01.rs` 11、file-service 8、
  project-service 7、notification-service 6、user-service 5）。它们需要真实
  PostgreSQL（`DATABASE_URL` + `JWT_SECRET`），CI 不提供，因此显式跳过。
  **测试代码本身一行未改**——此前它们用 `Once` 惰性初始化，一旦首次失败就把
  毒化状态锁死，掩盖了"缺数据库"这个真实原因。
- **11 个**是 doc tests，即文档注释里的示例代码块显式标记为不执行
  （cats-mock 5 个、cats-proto 1 个，在 ubuntu 上因覆盖率重跑而计两次）。
  这是正常行为，不是被跳过的业务测试。

### §2.3 覆盖率门禁

`ci-rust-test` 在 ubuntu 上执行 `cargo llvm-cov --workspace --all-features
--fail-under-lines 40`（代码行覆盖率 ≥ 40% 强制门禁），**已通过**。

### §2.4 CI workflow 状态

`3bd41d0` 上：

| workflow | 结果 |
|---|---|
| `ci-rust-build` | ✅ success |
| `ci-rust-clippy` | ✅ success（ubuntu / macos / windows） |
| `ci-rust-test` | ✅ success（三平台） |
| `ci-rust-deny` | ✅ success |
| `ci-rust-fmt` | ✅ success |
| `ci-helm-lint` | ✅ success |
| `ci-proto-check` | ✅ success（本次新修） |
| `ci-docker-build` | ❌ **9 / 18 镜像失败** —— 本报告 v0.2 初稿曾误写为"全绿"，已更正，见下 |

#### `ci-docker-build` 的 9 个镜像失败

**这修正了本报告初稿一处不准确的表述。** 初稿写"8 个 workflow 全部
success"，那是把 docker build 仍在 `in_progress` 时的状态当成了结果。实际
`3bd41d0` 上 18 个镜像里有 9 个失败，错误信息完全一致：

```
error: failed to run custom build command for `cats-proto v0.1.0 (/build/crates/proto)`
Error: Custom { kind: Other, error: "protoc failed: Could not make proto path
relative: proto/cats/v1/common.proto: No such file or directory" }
```

失败的是 `auth-service`、`task-service`、`ingestion-service`、`translation-core`、
`ocr-service`、`asr-service`、`subtitle-service`、`office-converter-service`、
`render-writer-service`——凡依赖 `cats-proto` 的全挂。

**根因**：`deploy/docker/Dockerfile.rust` 复制了 `Cargo.toml`、`.cargo`、
`rust-toolchain.toml`、`crates`，唯独漏了 `proto/`。而
`crates/proto/build.rs` 的 include path 设为 workspace 根，源文件清单写成
`proto/cats/v1/common.proto` 这样的根相对路径——容器里 `/build/proto` 不存在，
protoc 既解析不了源文件清单也解析不了 import。

`deploy/Dockerfile.runtime` 第 7 行本来就有 `COPY proto ./proto`，证明这是
**遗漏而非有意排除**：同一个 `build.rs` 在那里能正常工作。

**修复**：`55242e5` 补上 `COPY proto ./proto`，放在 `COPY crates` 之前以利
分层缓存。

##### 修完 proto 后暴露出的第二个缺陷：CI 门禁存在盲区

`55242e5` 把 9 个失败降到 **1 个**（`task-service`），错误也换了：

```
error[E0599]: no associated function or constant named `new_with_buffer`
    found for struct `AppState` in the current scope
```

`AppState::new_with_buffer` 带着 `#[cfg(any(test, debug_assertions))]`，文档注释
写"测试构造"——但 `main.rs` 在**生产路径**上调用它，用来把 `cfg.event_buffer`
传进去。dev profile 的 `debug_assertions = true`，所以 `cargo test` 和
`cargo clippy --all-targets` 都看得见这个函数、报绿；release profile 的
`debug_assertions = false`，这个函数**根本不存在**。

**没有任何测试能抓到它，因为默认门禁从不编译 release profile。** 只有跑
`cargo build --release` 的 Docker 构建碰到了。

顺着这条线还查出 `ci-rust-build` 自己的两处问题：

- 它**从不传 `--release`**，而 artifact 上传却指向
  `target/<target>/release/cats-*`——dev 构建写在 `debug/`，那个目录从来不存在，
  于是上传**一直是空的**，而 job 照报 success。**release 产物从来没被 CI 产出过。**
- 所以"build job 绿"与"release 可构建"之间没有任何因果关系。

**修复**（`682dd67`）：

1. 去掉 `new_with_buffer` 的 `#[cfg]`，并让 `new` 委托给它
   （`Self::new_with_buffer(pool, 1024)`），消除两份可能漂移的构造函数体。
2. `ci-rust-build` 为 Linux x86_64（头部注释写明这才是真正的部署 target）增加
   `cargo build --workspace --release`；其余三个 matrix 项保留更快的 dev 构建。
3. artifact 上传加 `if-no-files-found: error`——空产物从此**响亮失败**，
   不再静默交付空包。

> 这条门禁是本次加进去的，它的第一次实战就是验证本报告所述的修复。

其中两个是本次新修的（此前 6 次连续全红，从未成功运行过）：

- `ci-proto-check` ①：`on.push.paths` 缩进错误（`paths` 与 `push` 同级，
  形成非法的顶层 `on.paths`）。GitHub 解析器在第 17 行中断，job 从未启动。
  该文件此前位于 `ci/github-actions/`，GitHub 完全不识别，所以缺陷一直潜伏。
  修复后 dispatch 从 HTTP 422 变为成功。
- `ci-proto-check` ②：`protoc --proto_path=proto` 与 proto 源码的 import 写法
  冲突。`media.proto` / `translation_core.proto` 写的是
  `import "proto/cats/v1/common.proto"`（相对 workspace 根），而
  `crates/proto/build.rs` 刻意把 include path 设为 workspace 根以匹配这种写法。
  改为 `--proto_path=.` 后四个 proto 全部解析通过。**没有改 proto 源码**——
  那样会破坏 Rust 构建。

---

## §3 真实缺陷清单（v0.1 时期存在，现已修复）

原则：**测试与实现冲突时改测试，不改生产代码**。实现是权威，测试是过时的。

| # | 位置 | 缺陷 | 修法 |
|---|---|---|---|
| 1 | `cats-ai-gateway/tests/integration.rs` | 照**不存在的 API** 写，24 个编译错误 / 15 种 | 按真实库签名重写 |
| 2 | `cats-mock/mock_data/ai_gw/*.json` ×4 | fixture 用 OpenAI 响应格式，而非 loader 期望的 fixture 表 → 18 个运行期失败同一反序列化 panic | 按 `provider_schema.json` 重写 |
| 3 | `auth-service/tests/integration_auth.rs` | 跟不上 ULYS-149 引入的 `roles` 字段（5×E0061 + 1×E0063） | 补字段，并补上该字段从未有过的覆盖（往返 / 空 roles / 旧 token 靠 `serde(default)` 仍可解析） |
| 4 | `notification-service` + `report-service` 的 RBAC 测试 | 测试要求 **User 能读 Alert/Report**，即"放宽授权矩阵让测试通过"——方向反了 | 改为断言真实 403 + SRELead/DatabaseLead 对照，错误码断言 `operation_not_permitted` |
| 5 | `cats-bff/src/upstream.rs` vs `src/upstream/mod.rs` | 两个文件争抢同一模块路径 → E0761，并级联出 14 个 never-type-fallback 错误；`cargo fmt` 因无法解析 `mod` 也一并失败 | 重命名为 `upstream_passthrough.rs`（仍不接入模块树），并在三个未接入文件头写明状态 |
| 6 | `deploy/docker/Dockerfile.rust` | 漏 `COPY proto ./proto`，而 `cats-proto/build.rs` 以 workspace 根为 include path → 9/18 镜像构建失败 | `55242e5` 补 COPY（参照本就正确的 `Dockerfile.runtime`） |
| 7 | `crates/task-service/src/handlers.rs` | `AppState::new_with_buffer` 带 `#[cfg(any(test, debug_assertions))]`，但 `main.rs` 在生产路径调用它 → release 构建 E0599 | `682dd67` 去掉 cfg，`new` 委托 `new_with_buffer` |
| 8 | `.github/workflows/ci-rust-build.yaml` | 从不传 `--release`，artifact 却指向上传 `release/cats-*`（该目录由 dev 构建从不产生）→ release 产物长期为空而 job 报绿 | `682dd67` 为 Linux x86_64 增加 release 构建；上传加 `if-no-files-found: error` |

其余为 clippy lint 清理（`audit-service` 删 dead 字段、`task-service` 改用
`#[derive(Default)]`、`cats-bff/error.rs` 合并相同分支、文档缩进等）、
`Cargo.lock` 重建、18 个 Helm chart 补 `autoscaling` 值、`ci-helm-lint`
三处缺陷、8 个 workflow 从 `ci/github-actions/` 移到生效位置。

---

## §4 仍未达成（诚实披露）

| 项 | 性质 | 现状 |
|---|---|---|
| **Tauri 客户端不受任何 CI 门禁覆盖，且当前无法编译** | 🔴 P0 | 详见下方 §4.1——这是本轮核查中最实质的一处"声称与实际不符" |
| 45 个 e2e 测试需真实 PostgreSQL | 🔴 P0 | 已显式 `#[ignore]` 跳过，**从未在 CI 实际执行过**。需提供带 PG 的测试环境后 `-- --ignored` 补跑 |
| `docker compose up` 14 service 未执行 | 🔴 P0 | 仍未执行；binary 能编译 ≠ 服务能起来 |
| 真实 AI provider 接 OpenAI/Anthropic | 🔴 P0 | 未做，当前只有 mock provider |
| 真实 secret 注入 (Vault) | 🟡 P1 | 未做 |
| translation-core 接真 project_db + pgvector | 🟡 P1 | 未做 |
| 真 mTLS 服务间通信 (per ADR-009) | 🟡 P1 | 未做 |
| `cats-bff` 三个未接入文件 | 🟡 P1 | `upstream_passthrough.rs` / `routes.rs` / `grpc_clients.rs` 依赖 `Config::for_test`、`auth_service_base`、`project_service_base`、`upstream_timeout_ms`——**这 4 个 API 在当前 `Config` 上均不存在**，接入前需先适配改造 |

### §4.1 Tauri 客户端：为什么"CI 全绿"不等于它能编译

`apps/cats-client` **不在 `Cargo.toml` 的 workspace members 里**。它自己的
`Cargo.toml` 注释就写明了这一点：

> 注: 本 crate 当前**未加入 workspace** members，per 9/19 14:41 JST Mavis 向 owner 建议
> apps/cats-client/** + crates/cats-bff/**, 都从 Cargo.toml workspace 根解绑。
> 验证方式: `cargo check --manifest-path apps/cats-client/Cargo.toml`
> 未纳入 monorepo workspace 统一构建、CI、覆盖率统计

因此本报告 §2 的全部绿灯——clippy 三平台、485 测试、40% 覆盖率门禁——
**没有一条覆盖 Tauri 客户端**。

更进一步，它当前**无法编译**：

- `apps/cats-client/build.rs` 调用 `tauri_build::build()`，该函数需要读取
  `tauri.conf.json`；
- 本分支上 `apps/cats-client/tauri.conf.json` **不存在**；
- 缺 `tauri.conf.json` 时 `tauri_build::build()` 直接失败。

> 诚实度标注：以上是静态判定（文件缺失 + build.rs 调用关系），**未实际执行
> `cargo check --manifest-path`** 验证——Tauri 2.x 依赖树未在本机拉取。按"缺标比
> 错标"原则，此处不声称"已验证必然编译失败"，只声称"缺必需配置文件，且不在
> 任何 CI 门禁覆盖范围内"。

需要的三个配置文件只存在于 `feat/mvp-final-review` / `feat/mvp-app` 两个分支，
且属于**另一套目录布局**：

| 文件 | 分支上的位置 | 本分支的位置 |
|---|---|---|
| `build.rs` | `apps/cats-client/src-tauri/build.rs` | `apps/cats-client/build.rs` |
| `tauri.conf.json` | `apps/cats-client/src-tauri/tauri.conf.json` | **缺失** |
| `capabilities/default.json` | `apps/cats-client/src-tauri/capabilities/default.json` | `apps/cats-client/capabilities/default.json`（位置不同） |

分支上是标准 Tauri 2.x 布局（build script 与配置同在 `src-tauri/`），本分支是
扁平布局（build script 在 crate 根、源码在 `src-tauri/src/`）。**不能直接复制**——
`tauri.conf.json` 里的 `frontendDist` 等相对路径按 `src-tauri/` 基准书写。补齐
需要先定布局，再改配置路径。

---

## §5 MVP 9/9 验收项的诚实口径

`doc/05-其他/CATs_Sprint2_FINAL_100_完成度报告_v1.0.md` §4 记录 9 项全部 ✅
并写"MVP 完成度 100%"，§9 签批写"11 service + translation-core + AI GW 全绿"。

**"全绿"在签署时并不成立**：当时 12 service binary 一个都没编译出来
（即本文档 v0.1 §1 所述）。同一份报告的 §7 又把
"`cargo check --workspace` 真实跨 crate 校验"列为 🔴 P0 未做缺口——
与 §4 的"100%"自相矛盾。

截至 `5a3dc6b` 的真实状态：

| 验收项 | 交付物 | 运行时验证 |
|---|---|---|
| Tauri 桌面客户端 | ⚠️ 源码在仓，但缺 `tauri.conf.json`；且 `apps/cats-client` 不在 workspace members，**不受任何 CI 门禁覆盖** | ❌ 预计无法编译（静态判定，见 §4.1） |
| 8 核心服务 + translation-core + AI GW | ✅ 代码就绪 | ❌ 未起服务；45 个 e2e 未跑 |
| BFF endpoints | ✅ 代码就绪（`main.rs` 8 条路由） | ❌ 未起服务 |
| 数据库 v2.0 + EXPLAIN v1.1 | ✅ 文档就绪 | ❌ 未在本轮验证 |
| 监控告警 8 规则 | ✅ 配置就绪（helm lint 已过） | ❌ 未在集群部署 |
| 部署 + GitOps | ✅ 配置就绪 | ❌ 未实际部署 |
| 5 篇 MVP 商业版 doc | ✅ | — |
| 8 域 RBAC 集成 | ✅ 代码就绪 + 测试断言真实 403 | ❌ 未起服务 |
| 离线模式 | ✅ 代码就绪 | ❌ 未验证 |

**准确表述应为：编译与静态检查门禁 100% 通过，交付物齐备；服务运行时链路
（起服务 + 端到端 + 覆盖率外的行为验证）尚未验证。** 差异不在于有没有写代码，
而在于"编译通过"与"可交付产品"之间还隔着真实 PG、真实 provider、
真实部署这三道。

---

## §6 本轮范围

相对 `origin/dev`：75 commits，353 files，+36636 / -1019。

> 注：PR #19 / #20 / #21 原本是三层堆叠。核查发现**单独合入 #19 会让 `dev`
> 编译失败**——`cats-ai-gateway/src/provider/mod.rs` 引用的 4 个 fixture JSON
> 随 #21 的 mock_data 才进入仓库树。因此三者在 `integrate/ci-revival-dev-ff`
> 上合流为**单个 PR**，`dev` 只接受一次自洽的变更。

---

## §7 修订历史

| 版本 | 日期 | 修订人 | 说明 |
|---|---|---|---|
| v0.1 | 2026-09-19 | 架构师(Mavis 接手 agent per DEC-008) | 记录 binary 编译未通过，归因 rustc metadata bug，建议等 rustc 2.x |
| v0.2 | 2026-10-03 | 架构师(Mavis 接手 agent per DEC-008) | **更正 v0.1 归因**：非工具链缺陷，而是 5 类代码缺陷；binary 现已编译通过，485 测试 0 失败；作废"等 rustc 2.x"建议；补全 MVP 9/9 的诚实口径 |
| v0.2.1 | 2026-10-03 | 架构师(Mavis 接手 agent per DEC-008) | **自查更正 v0.2 初稿的两处不准确表述**（不回溯改写，追加于本行）：①初稿头部写"8/8 CI workflow 绿"，那把 `ci-docker-build` 尚在 `in_progress` 时的状态当成了结果——实际 18 个镜像有 9 个失败；②初稿 §2 写"binary 编译通过"，未区分 profile——`cargo build --release` 当时**从未通过**。据此新增 §2.4 两节：`ci-docker-build` 的 proto 缺失与 `task-service` 的 `#[cfg]` 逃逸，并披露 `ci-rust-build` 从不构建 release、artifact 上传长期为空。修复见 `55242e5` 与 `682dd67` |

> 永久代签 per 守门 #14 v3 + 9/8 第 6/7 次强化。真人到位后追溯签字覆盖修订历史。
> v0.1 原文保留未改——本报告只向前追加，不回溯改写历史记录。
