# CATs Backend 启动状态报告 v0.3

**日期**: 2026-10-03
**作者**: 架构师(Mavis 接手 agent per DEC-008)
**取代**: `deploy/BACKEND_STATUS_v0.2.md`（2026-10-03）— v0.2 全文保留，未修改
**基线 commit**: `958397b`（`feat/e2e-real-pg`，PR #22 → PR #19 → `dev`）
**状态**: 🟡 **43 个 DB-backed e2e 首次在真实 PostgreSQL 上执行并全绿 + 常规门禁 487 测试 0 失败 + CI 4/4 绿**；Tauri 客户端仍**不在任何 CI 门禁覆盖内**，服务运行时链路仍未验证（见 §6）

---

## §1 本轮唯一的主题：把 45 个从未执行的测试真正跑起来

v0.2 把「45 个 e2e 需真实 PostgreSQL、**从未在 CI 实际执行过**」列为 🔴 P0。
本轮（PR #22）把它从"标注"变成"事实"。

CI 新增独立的 `e2e` job（`runs-on: ubuntu-latest`），用
`pgvector/pgvector:pg18` 起真实数据库、按 service 建 5 个库并应用各自
migration、**串行**执行 6 个测试套件。

**run `37113777361` 实证（逐套件解析，非只看 job 绿灯）：**

| 套件 | passed | failed |
|---|---|---|
| auth-service `e2e_auth` | 8 | 0 |
| auth-service `e2e_t01` | 11 | 0 |
| user-service `e2e_t02` | 5 | 0 |
| project-service `integration` | 7 | 0 |
| file-service `integration` | 8 | 0 |
| notification-service `integration` | 6 | 0 |
| **合计** | **45** | **0** |

**这些测试此前一次都没跑过。** 它们不是"本来是好的"，是从来没被执行过，
所以缺陷从未暴露。

### §1.1 精确口径：是 45 个"曾经被跳过"，不是 45 个"需要数据库"

事后发现 `e2e_t01` 的 11 个里有 2 个是**纯静态源码断言**
（`include_str!` + substring 检测，不建连接、不跑 migration），
却被批量 `#[ignore = "e2e-needs-real-pg"]` 一并盖住。

代价是实的：这两条守护 ULYS-46 的 INVIOLABLE 约束（`build_audit` 不得用
`spawn`、且必须 `.await` emit），标了 ignore 就只在 Linux 的 e2e job 里被
检查，macOS / Windows 常规门禁全漏。已解除。

所以准确说法是：

- **43 个**真需要数据库的 e2e → 在 e2e job 对真实 PG 执行并通过
- **2 个**静态源码断言 → 归位常规 `cargo test --workspace`，三平台都跑
- 全仓 `#[ignore]` 属性由 **45 降为 43**

---

## §2 撤回 v0.2 中的两处陈述（不回溯改写 v0.2）

按"只向前追加"原则，v0.2 原文一字未改。以下两条**在 v0.2 的时点是正确的**，
在 PR #22 落地后已不成立，在此显式撤回：

| v0.2 位置 | 原陈述 | 现状 |
|---|---|---|
| 头部状态行 | "45 个 e2e 仍需真实 PG **从未执行**" | 已执行，43/43 通过（§1） |
| §4 P0 表 | "45 个 e2e 测试需真实 PostgreSQL \| 🔴 P0 \| 已显式 `#[ignore]` 跳过，**从未在 CI 实际执行过**" | 该 P0 关闭，见 §6 |

v0.2 的其余结论——8 类代码缺陷清单、`ci-docker-build` 9/17→17/17、
release artifact 6,643,798 B、§4.1 Tauri 静态判定、§5 MVP 9/9 诚实口径——
**在 v0.3 全部继续有效**，此处不复述。

---

## §3 实测状态更新

### §3.1 常规门禁（`cargo test --workspace --all-features --locked`）

| 平台 | passed | failed | ignored | 证据 |
|---|---|---|---|---|
| macos-latest | **487** | **0** | **54** | run `37118286511` 逐 job 日志求和 |
| ubuntu-latest | 1241 | **0** | 140 | 含 `cargo llvm-cov` 重跑一遍，计数约为两倍 |

对比 v0.2 的 485 / 0 / 56：**+2 passed、−2 ignored**，正是解除的两个
静态断言。**这个差值是实测求和得到的，不是推算的。**

### §3.2 DB-backed e2e

见 §1。

---

## §4 CI 自身的两个缺陷（run 37112321110 实证）

接进 CI 后第一次 run 就红了，**两处都与测试无关，是 CI 自己的问题**：

### 缺陷 1：e2e job 的 Postgres 镜像缺 pgvector

```
psql:crates/project-service/migrations/20260919_0001_init.sql:14:
ERROR:  extension "vector" is not available
```

`pgcrypto` / `citext` 官方镜像自带 contrib，**`vector` 不带**。本机 e2e 用的
`pgvector/pgvector:pg18` 天然带 vector，所以本地一路绿灯、CI 却红——
**镜像差异版的"本机跑绿 ≠ 门禁能跑绿"**。已同步为同一镜像
（`docker inspect` 确认本机容器用的就是它）。

### 缺陷 2：`test` job 残留一段死代码

```
psql: error: connection to server at "127.0.0.1", port 5432 failed: Connection refused
```

service 容器只能放 Linux-only 的 `e2e` job，`test` job 矩阵含 macOS/Windows
故**根本没有 Postgres service**。把灌库步骤移出 `test` job 时漏删了
`if: runner.os == 'Linux'` 下的同名步骤。

顺带修正该步骤里一句本身就是错的注释——「版本号递增且都用 IF NOT EXISTS，
顺序应用不会冲突」。恰恰相反，见 §5.1。**注释比代码更危险，因为它是
后来者唯一会读的文档。**

---

## §5 本轮暴露的产品级缺陷（不是测试问题）

### §5.1 全新数据库上 schema 初始化无法完成 🔴 P0

file / project / notification / user 四个 service 各有**三份 init migration**，
来自三条平行开发线，都以 `CREATE TABLE IF NOT EXISTS <同名表>` 建表。
版本号最小的先执行、抢先建出自己那套模型，后两份被 `IF NOT EXISTS`
**静默跳过**，随后它们的索引引用别的 init 才有的列：

```
ERROR: column "workspace_id" does not exist
ERROR: column "read_at" does not exist
```

**生产首次部署到空库会踩到同一脚。** 已按"生产代码实际读写的列"为权威判据
停用与代码不符的旧 init，保留不冲突的附属表。

`user-service` 是另一类：`DO $$` 内部又开 `AS $$`，而 PostgreSQL 的
dollar-quote **不支持嵌套**，内层提前关闭外层块 → `syntax error at or near "BEGIN"`。

> 踩到的坑：修复时在注释里写了 `$$` 解释原因，而注释本身位于 `DO $$` 块内，
> 同样被截断。**dollar-quote 块内的注释也必须避开分隔符。**

### §5.2 测试的 App 装配与 `main.rs` 系统性漂移 🟡 P1

三个文件的 `make_app` 都**手抄** `main.rs` 的注册代码，必然漂移：

| service | 漏注册 | 症状 |
|---|---|---|
| auth | `web::Data<AppState>` | 全部 500 |
| project | `web::Data<Arc<RbacChecker>>` | 全部 500 |
| file / notification | `web::Data<Arc<RbacChecker>>` | **只有 healthz 过**，其余全 500 |

"healthz 过、业务全挂"是一条强信号：healthz 是唯一不注入
`RbacChecker` 的 handler。失败得极快（0.02–0.06s）也是线索——真连库失败
的请求会卡在连接或事务上，不会瞬间结束。

修完全仓 16 个 service 已逐一比对，**同类隐患清零**。

### §5.3 一个间歇性失败的门禁 🟡 P1

`task-service` 的 `smoke_timeout_helper` 用真实墙钟做 `10ms 超时 vs 20ms sleep`
的时序断言，余量只有 10ms。`tokio::time::timeout` 是**先 poll 内层 future、
再检查自己的定时器**，线程一旦被挂起超过 20ms，醒来时 sleep 的 deadline
也已过、立即返回 Ready，于是 `timeout` 返回 `Ok`，断言挂掉。

实证：同一份代码 run `37118286511` 全绿、run `37118658229` 在
windows-latest 上 FAILED，差异只有调度时机。已改用
`#[tokio::test(start_paused = true)]` 虚拟时间，从根上去掉对调度的依赖。

> 一个间歇性失败的门禁比没有门禁更糟——它训练团队忽略红色。

---

## §6 仍未达成（诚实披露）

| 项 | 性质 | 现状 |
|---|---|---|
| **Tauri 客户端不受任何 CI 门禁覆盖，且当前无法编译** | 🔴 P0 | 维持 v0.2 §4.1 判定。2026-10-03 拍板：**暂不引入客户端**，先把后端收干净 |
| `docker compose up` 14 service 未执行 | 🔴 P0 | 仍未执行；binary 能编译 ≠ 服务能起来 |
| 真实 AI provider 接 OpenAI/Anthropic | 🔴 P0 | 未做，当前只有 mock provider |
| 真实 secret 注入 (Vault) | 🟡 P1 | 未做 |
| translation-core 接真 project_db + pgvector | 🟡 P1 | 未做 |
| 真 mTLS 服务间通信 (per ADR-009) | 🟡 P1 | 未做 |
| `cats-bff` 三个未接入文件 | 🟡 P1 | 依赖的 4 个 `Config` API 在当前版本不存在，接入前需先适配改造 |
| ~~45 个 e2e 需真实 PostgreSQL~~ | ~~🔴 P0~~ | **已关闭**（§1）。PR #22 落地后 `dev` 将首次在 CI 内含真实数据库 |

---

## §7 MVP 9/9 验收项的诚实口径（更新）

维持 v0.2 §5 的结论，仅更新其中一行：

| 验收项 | 交付物 | 运行时验证 |
|---|---|---|
| Tauri 桌面客户端 | ⚠️ 源码在仓，缺 `tauri.conf.json`，不在 workspace members | ❌ 预计无法编译（静态判定，见 v0.2 §4.1） |
| 8 核心服务 + translation-core + AI GW | ✅ 代码就绪 | ⚠️ **e2e 已在真实 PG 上验证（43/43）**，但仍未 `docker compose up` 起服务 |
| BFF endpoints | ✅ 代码就绪（`main.rs` 8 条路由） | ❌ 未起服务 |
| 其余 6 项 | ✅ | ❌ 见 §6 |

**准确表述更新为：编译与静态检查门禁 100% 通过；DB 层运行时链路
（真实 PG + migration + 端到端 API）已验证通过；服务编排层
（起服务 + 真实 provider + 真实部署）仍未验证。**

一道门跨过了，下一道还在。

---

## §8 本轮拍板记录

| 事项 | 结论 | 日期 |
|---|---|---|
| `dev` 分支保护 | 不拆。PR #19 由 Ulysses 本人审批（admin 无法自批） | 2026-10-03 |
| PMO Lead 权限 | **保持现状**：`Project` / `File` 写权限只给 `Sponsor`，符合权限矩阵 v1.0 §3，不给 `ProjectLead` 补权限 | 2026-10-03 |
| Tauri 客户端 | 暂不引入，先把后端收干净 | 2026-10-03 |
| 12 个 `feat/mvp-*` 分支 | 全部保留不动（`hygiene=safe`） | 2026-10-03 |

> 四项均经选项表单的**超时默认选中**收到（`responseSource:
> automatic_timeout`，非显式回复），选中的正是推荐项。结论照此执行。

PMO Lead 那条要特别记住：它**不是遗漏，是被显式确认过的**。将来若有人看到
`ProjectLead` 管不了 `Project` 而当成 bug 去"修"，请先读这里。

---

## §9 修订历史

| 版本 | 日期 | 修订人 | 说明 |
|---|---|---|---|
| v0.1 | 2026-09-19 | 架构师(Mavis 接手 agent per DEC-008) | 记录 binary 编译未通过，归因 rustc metadata bug，建议等 rustc 2.x |
| v0.2 | 2026-10-03 | 同上 | 更正 v0.1 归因；binary 编译通过，485 测试 0 失败；作废"等 rustc 2.x"建议 |
| v0.2.1 | 2026-10-03 | 同上 | 自查更正两处不准确表述；补全 `ci-docker-build` 与 release 盲区 |
| v0.2.2 | 2026-10-03 | 同上 | 复验并更正计数（9/17 而非 9/18）；镜像 17/17、artifact 6,643,798 B、CI 7/7 |
| **v0.3** | 2026-10-03 | 同上 | **43 个 DB-backed e2e 首次在真实 PG 上执行并全绿**（run 37113777361 逐套件解析）；常规门禁 487/0/54；**显式撤回 v0.2 的两处"从未执行"陈述**（v0.2 原文未改）；新增 CI 两处自身缺陷、3 类产品级缺陷（migration 三重 init / App 装配漂移 / flaky 门禁）、§8 拍板记录 |

> 永久代签 per 守门 #14 v3 + 9/8 第 6/7 次强化。真人到位后追溯签字覆盖修订历史。
> **v0.1 / v0.2 原文均保留未改**——本报告只向前追加，不回溯改写历史记录。
> 作废的结论以"撤回声明"的形式记录在新版本里，而不是去改旧版本。

## 附：本轮新增文档

- `deploy/E2E_FINDINGS_v1.0.md` — 45 个 e2e 逐个跑起来时发现的完整过程记录，
  含 migration 权威判据表、RBAC 矩阵实测表、全仓 app_data 漂移审计结果。
