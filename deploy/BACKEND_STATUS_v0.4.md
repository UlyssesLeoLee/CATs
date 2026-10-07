# CATs Backend 启动状态报告 v0.4

> **本文件不回改 v0.3。** 发现 v0.3 写错的地方，在 §2 显式撤回并说明理由。
> 本轮主题：给 `docker compose up` 这条路径继续加闸门，闸门反过来抓出了
> **3 类新缺陷**，其中 1 个是 P0，而且**它躲过了上一轮那套 12 路 healthz 全绿的验证**。

---

## §1 本轮做了什么

给 `deploy/scripts/lint-compose.py` 加了两类静态不变量：

| 规则 | 挡什么 | 本轮抓到 |
|---|---|---|
| 规则 6 | compose 注入的 env 变量必须真被源码读过（含仓内 path 依赖） | 8 项，其中 1 项 P0 |
| 规则 7 | `src/` 下每个 `.rs` 必须真被 `mod` 声明引用（否则永不编译） | 12 crate / 5731 行 |

两条规则都不需要构建镜像、不需要起容器，各几秒。
**关键不是它们抓到了什么，是它们证明了上一轮的"全绿"覆盖不全。**

---

## §2 撤回 v0.3 §4 的一处陈述

v0.3 与 `COMPOSE_UP_DEFECTS_v1.0.md` §四 写的"12 路 `/healthz` 全 HTTP 200"
**在字面上没错，但覆盖不全，且漏掉的恰好是唯一坏掉的那个 service。**

那 12 路是：envoy / auth / user / project / task / file / notification /
report / audit / worker / ai-gateway / cats-bff。

**`translation-core` 不在其中。** 它当时实际监听 8090，而 compose 映射的是
50051，所有依赖方都指着 `translation-core:50051` —— 也就是说，那次验证
**恰好绕开了唯一坏掉的 service**。

这不是数值问题，是**验证手段本身的覆盖漏洞**：探针清单手写，漏一个目标
不会让脚本失败，只会让报告少一行；而报告与"17 容器 Up"并排呈现，读者
会合理地以为覆盖了全部。

**已修**：`mvp-backend-up.sh` 的探针清单改为**从 compose 编排推导**
（`docker compose config --format json` → 只取 command 指向
`/usr/local/bin/*` 的应用 service + envoy → 取其宿主端口），
并断言"探针目标数 > 0"，避免 0 失败被当成通过。

实测推导结果 **13 个目标 = 原 12 个 + translation-core(50051)**，
postgres / kafka 因不提供 `/healthz` 被正确排除。

> 同族：v0.3 §4 缺陷 1（e2e job 的 Postgres 镜像缺 pgvector）与本文档
> §1 抓到的问题同源——**检查通过了，但它没在看那一个**。

---

## §3 三类新缺陷

### §3.1 🔴 P0：translation-core 监听 8090，全链路指着 50051

compose 与架构书 §4.1 给 `GRPC_BIND_ADDR=0.0.0.0:50051`，
而 `main.rs` 只读 `BIND_ADDR`（默认 `0.0.0.0:8090`）。

**实证**（真实镜像 + compose 的 env）：

```
INFO translation_core: starting translation-core bind_addr=0.0.0.0:8090
INFO actix_web::server: starting service: "actix-web-service-0.0.0.0:8090"
```

**修法**：改源码，不改配置。`GRPC_BIND_ADDR` > `BIND_ADDR` > 默认
`0.0.0.0:50051`。默认值一并从 8090 改成 50051——8090 是 ai-gateway 的端口，
留着等于给这个服务预埋第二个同类 bug。配 3 个单测锁住优先级
（直接改 process env 在并行测试里是全局竞态，故拆成纯函数测）。

**修复后实证**（新二进制 + compose 的 env）：

```
INFO translation_core: starting translation-core bind_addr=127.0.0.1:50160
$ curl http://127.0.0.1:50160/healthz
HTTP 200: {"status":"ok","app":{"name":"cats-common","version":"0.1.0"}}
```

> 响应里 `name` 是 `cats-common`，这是 §4.3 记的另一个缺陷，一并说明。

### §3.2 5 个 service 配了 7 个源码从不读取的环境变量

| service | 死变量 | 源码实际读的 |
|---|---|---|
| audit-service | `KAFKA_BROKER` | `KAFKA_REST_URL` / `KAFKA_CONSUMER_GROUP` / `KAFKA_AUDIT_TOPIC` |
| file-service | `FILES_DIR` | `DATABASE_URL` / `BIND_ADDR`（无 blob 存储实现） |
| notification-service | `KAFKA_BROKER`、`KAFKA_NOTIFICATIONS_TOPIC` | 无任何 Kafka 代码 |
| translation-core | `DATABASE_URL`、`AI_GATEWAY_URL` | 仅绑定地址 |
| worker-service | `DATABASE_URL` | `BIND_ADDR` / `TRANSLATION_CORE_URL` |

外加 `file-service` 挂着的 `cats-mvp-files` 卷——无人读写，一并移除。

这类缺陷不报错、不告警、不阻止容器启动，只是**安静地不生效**。

### §3.3 12 个 crate 共 5731 行 `.rs` 从未被编译

`lib.rs` 只声明了 `version()` / `name()`，磁盘上却躺着完整业务实现。

| crate | 行数 | 内容 |
|---|---|---|
| cats-mock | 2899 | 12 文件；`lib.rs` 文档宣称提供 `http`/`db`/`infra`/`data` |
| cats-bff | 809 | `routes.rs` / `upstream_*.rs`（依赖 4 个不存在的 Config API，v0.3 §6 已记） |
| common | 550 | `error.rs`；`lib.rs` 里另有一份 inline 的 `CatsError` |
| translation-core | 550 | `service.rs`(174 行编排) / `qa.rs` / `db.rs` / `tm.rs` / `glossary.rs` / `ai_gateway.rs` |
| 其余 8 crate | 423 | 各自 `state.rs` / `models.rs` / `consumer.rs` 等 |

**实证**（不是推断）：往 `crates/common/src/error.rs` 注入一行语法错误后，
`cargo check -p cats-common` 仍然 `Finished in 15.55s`、退出码 0。
rustc 根本没看这个文件。

**后果**：`translation-core` 与 `worker-service` 的 `main.rs` 至今是 M0 占位
（只注册 `/healthz`），从不构造 `PgPool`、从不启动 `run_scheduler_loop`。
`worker-service/scheduler.rs` 里完整的抢占→派发→回写逻辑一行没跑过。

**闸门**：lint 规则 7 做死文件检测，已知存量列入 `ORPHAN_BASELINE`，
新增即 FAIL。**存量如实报在提醒行里（12 crate / 5731 行），不假装它不存在。**

---

## §4 仍未达成（诚实披露）

### §4.1 孤儿代码：已接线 4 个 crate，剩 8 个待拍板

> 2026-10-04 更新：§3.3 里那张表里最要紧的几项已经处理，原「拍板选项」
> 里的 **B（接 translation-core + worker-service）已执行**。下面先说做了什么，
> 再说剩下的。

#### 已接线（约 1450 行从"写在磁盘上"变成"真的在编译"）

| crate | 行数 | 接线时暴露的真实问题 |
|---|---|---|
| `common` | 550 | `error.rs` 依赖 `actix-web` / `tonic` / `sqlx`，Cargo.toml 三个都没有；且全仓 `use cats_common::CatsError` 的**每一个**使用者都在未编译模块里 |
| `cats-rbac` | 178 | `service_helpers.rs` 依赖 `actix-web` / `uuid`，同样没声明 |
| `translation-core` | 550 | 依赖缺 `async-trait` / `uuid`；**proto 类型名全错**（见下） |
| `worker-service` | 170 | 依赖缺 `cats-rbac` / `uuid`；`tick` 是私有函数但 `handlers.rs` 要调它；`mark_qa_blocked` 返回类型与声明不符 |

**最值得记的一条：prost 会把 proto3 类型名规范化。**
`TMMatchItem` 生成成 `TmMatchItem`、`MatchTMRequest` → `MatchTmRequest`、
`QAViolation` → `QaViolation`、`RunQARequest` → `RunQaRequest`。
而那段代码是照着 **`.proto` 里的名字**写的，不是照着 **prost 生成的名字**写的。
这类错误在"代码写完但没编译"的仓库里可以潜伏任意久 —— 它只在
`cargo build` 的那一刻存在。

第二类：prost 把 proto3 枚举字段生成为 `i32`，而代码把
`req.source_lang` 当成 `LanguageCode` 枚举直接用（4 处类型不匹配）。
现在用一个显式 `lang(i32) -> LanguageCode` 转换。

第三类：`CatsError` 上没有 `error_code()` 方法，实际叫 `code()`。

**这一切只有真正去编译才会暴露。** 从「接线」到「编译通过」中间是
9 → 1 → 0 的三轮，每一轮都是新信息。

#### 剩余（5 crate / 255 行，2026-10-05 cats-bff 全部接完后）

**这个数字在本轮被推翻过两次**，见 §4.1c（lint 假账）与 §4.1d（cats-bff 接线）。
> **【已落实 · 2026-10-05】**下面这张表已经过时。逐项调查后删掉了 6 个文件，
> 孤儿存量现为 **1 crate / 75 行**（仅 `notification-service/consumer.rs`）。证据与两条技术断言的撤回
> 见 **§4.1f**。

| crate | 行数 | 状态 |
|---|---|---|
| file-service | 2 文件 | `state.rs` + `storage.rs` |
| notification-service | 2 文件 | `consumer.rs` + `state.rs` |
| project / report / task-service | 各 1 文件 | 各自的 `state.rs` |

剩下的 5 个 crate 共 255 行，**是 9 个 `state.rs` 里还没接的那几个**。
`audit-service` 那种"四个文件互相引用、缺一不可"的情况已经没有了 ——
剩下的要么是没人引用的类型（接上去只会产生 `dead_code`），要么需要先决定
哪个 `AppState` 是权威（`task-service` 的 `handlers.rs` 自己又定义了一个）。

##### 6 个 `state.rs` 的三种不同情况（2026-10-05 逐 crate 核对）

这三个 crate 的 `state.rs` 都是 19~27 行的 `AppState { pool, checker }`，
看着像同一个模板，**实际不能一把接**：

| crate | 现状 | 接 `mod state;` 的后果 |
|---|---|---|
| **audit-service** | ✅ **已于 2026-10-05 接完**（见下节）。原本 `handlers.rs`（124 行完整 HTTP handler）引用 `crate::db` / `crate::models` / `crate::state::AppState`，而 `lib.rs` **只声明了 `pub mod consumer;`** | 这不是"接一个 state"，而是**接一整组**：`db` / `models` / `state` / `handlers` 四个文件互相引用，缺一个都编译不过 |
| **task-service** | `handlers.rs` **自己又定义了一个 `AppState`**（12 处引用），`state.rs` 里还有另一个 | **接上去会造出两个竞争的 `AppState`**，编译期就冲突。这需要先决定哪个是权威，不能顺手做 |
| **project / report / file / notification** | `state.rs` 无人引用，其余文件也不提 `AppState` | 只加 `mod` 会多出一个没人用的类型，workspace 严格 lint 下是 `dead_code`（可能直接是错误）。**不构成进展** |

##### audit-service 是下一块最合理的阵地

它的 `handlers.rs` 依赖：

```rust
use crate::db;
use crate::models::{AuditLogListResponse, AuditLogResponse, KafkaAuditEvent};
use crate::state::AppState;
use cats_common::{cats_error_to_response, CatsError, ErrorCode};
use cats_rbac::service_helpers::{extract_user_id_and_roles, require_roles};
```

后两条**正好是本轮刚接上的两个共享模块**（`common/src/error.rs` 与
`cats-rbac/src/service_helpers.rs`）。也就是说：这两个模块当年写好了、
却因为自己没被 `mod` 声明而不可用，于是依赖它们的 audit handler
也一起卡死。**接上共享模块等于一次性解锁了这条链。**

它接完之后 audit-service 才有真正的 HTTP 端点（接之前只有 `/healthz`），
这是**补功能**而不只是清死代码。

> 这正是「一个 27 行的小文件看起来该有多简单」和「它其实是设计冲突」的差别。
> 接线 translation-core 时遇到的 prost 类型名问题同理：**动手前先确认
> 它接的是谁、接上之后有没有人用**。

### §4.1b audit-service 接线结果（2026-10-05，commit `18c577c`）

上面 §4.1 的判断**成立**：四个文件互相引用，缺一不可；接完之后该服务第一次
有了业务端点。

```
GET  /healthz             存活探针
GET  /readyz              就绪探针（含 DB 探活）
GET  /v1/audit-logs       按 org 分页列出（RBAC: Audit Read）
POST /v1/audit-logs/test  手动 ingest（RBAC: Audit Audit）
```

**从"编译过"到"能用"之间又翻出 4 个真问题**，每一个都是因为这段代码
从未被编译过：

| # | 问题 | 后果 |
|---|---|---|
| 1 | `test_ingest` 签名是 `_req: HttpRequest`，**全程零校验** | 任何能连到 8088 的人都能往 `audit_logs` 写行 —— 审计记录可被伪造 |
| 2 | handler 内嵌一份静态角色表，其中的 `Role::ReviewLead` / `Role::PlatformLead` **在 cats-rbac 里不存在** | 就算能编译，也是一套过时的角色表；且语义反了（把解出来的调用方角色丢成 `_roles`）|
| 3 | 6 个未用 import（`handlers.rs` 2 个 + `consumer.rs` 4 个，后者是删掉重复 `KafkaAuditEvent` 定义后新产生的）| `clippy -D warnings` 直接红 |
| 4 | `models.rs` / `db.rs` 用 `sqlx::types::ipnetwork::IpNetwork`，而 workspace 的 sqlx **没开该 feature** | E0433 编译不过。开 feature 要给 `Cargo.lock` 加包，而 CI 跑 `--locked` |

第 4 条的修法值得记一笔：**没有开 feature，而是按仓库既有约定改**——
写侧绑 `&str` + `$9::inet` 让 PG 自己转换（与 `consumer.rs` 完全同一套），
读侧 `ip::text AS ip`。依据是 `auth-service` 的 `source_ip` 早就显式改成了
`TEXT`，注释写着「改 TEXT 简化 bind」。

#### 写测试时被实测打脸两次，两次都是**断言错、代码对**

第一版测试只直接调 `require_roles` 测权限矩阵，7 条全绿，但鉴别力接近零：
把 handler 里的 `Resource::Audit` 改成 `Project`、或者整个删掉
`require_roles`，它照样全绿——它验证的是 cats-rbac，与 audit-service 的
handler 无关。改成用 `actix_web::test` 真打 HTTP 之后，才有信息进来：

1. **Guest 单独访问是 401 不是 403。** `Role::Guest` 的定义注释就写着
   「未登录」（`cats-rbac/src/lib.rs:53`），全仓库一致（5 个 service 的
   `is_authenticated()` 都是 `!roles.contains(&Role::Guest)`）。

2. **更要紧：Guest 不是集合级否决。** `require_roles` 把角色集合**拆开、
   逐个单独判定**（`check_roles(&[*role], ...)`），所以
   `Guest,DatabaseLead` 会被**放行**——DatabaseLead 单独就够格，
   直接 `return Ok()`，根本走不到 Guest 那一轮。

#### 🐛 顺带钉住一个已知实现瑕疵（本次只记录，未改）

`User,Guest` 返回 **401**，`Guest,User` 返回 **403**。同一组角色、仅顺序
不同。原因在 `cats-rbac/src/service_helpers.rs:104`：它只保留**最后一次**
迭代的错误，Guest 排在最后就留下 `Unauthenticated`。

两个后果：同一用户的状态码随 header 顺序变化；401 对一个**已登录**的调用方
是误导性的，会把人引去查认证链而不是权限矩阵。

**未修的理由**：`require_roles` 是 16 个 service 共用的 helper，改它的错误
优先级会影响全部服务，属于需要单独拍板的范围。当前以**特征化测试**的形式
记录在 `crates/audit-service/tests/rbac_audit_read.rs`，注释写明「若将来有人
修，应当连同注释一起改」，而不是让测试自动跟着变。

#### ⚠ 过程事实：这个分支从来没有自动 CI

所有 workflow 的触发条件都是：

```yaml
on:
  push:
    branches: [main, dev, 'feature/**']
  pull_request:
    branches: [main, dev]
```

本分支叫 `feat/e2e-real-pg`（是 `feat/` 不是 `feature/`），PR #22 的 base 是
`integrate/ci-revival-dev-ff` —— **两头都不匹配**。所以 push 不触发任何检查。

**更正**：本文档此前几轮记录的「4/4 绿」，实际只来自 `workflow_dispatch`
**手动触发**的 `ci-rust-test` 一个 workflow。`ci-rust-fmt` 与
`ci-rust-clippy` 从未跑过。本轮补跑后 fmt 立刻是红的（20 个文件的格式债，
其中 13 个缺文件末尾换行，分布在**此前已 push 的** common / cats-rbac /
translation-core / worker-service 里），已由 `8b7a675` 修掉。

> 教训与本文档开头那句一致：**绿色的检查也可能从来没运行过。**
> 「跑了 4 个 job 全绿」和「有 4 个检查会跑」是两件事。

**已修（`fa21e89`，随 PR #19 落地）**：`push.branches` 加 `feat/**`，
`pull_request` 去掉 base 过滤。改完之后 **PR #19 立刻从 0 个 check 变成全量自动
触发**（fmt / clippy / test / build / docker build / helm lint / deny / proto
check 全在跑）—— 这就是改动生效的端到端证据。

顺带一个更根本的事实：**`dev` 分支上根本没有 `.github/workflows/`**，workflow
文件全在 `ci/github-actions/`。而 GitHub Actions 只读 `.github/workflows/`，
放在那里的文件**完全失效**。PR #19 正是在做这件事：把 CI 放回生效位置。所以
触发条件的修改跟着 #19 走而不是另开 PR。

### §4.1c 撤回两处错误记录：lint 规则 7 自己的可达性有 bug 🆕

写这份文档的过程中，`lint-compose.py` 规则 7（死文件检测）**自己报错了**。

#### 现象

接线 `upstream_passthrough.rs` 时把 `cats-bff` 的基线减了一项，lint 立刻报：

```
FAIL  crate 'cats-bff' 的 src/upstream/auth.rs 没有被任何 mod 声明编译
FAIL  crate 'cats-bff' 的 src/upstream/projects.rs ...
FAIL  crate 'cats-bff' 的 src/upstream/tasks.rs ...
```

可是 `src/upstream/mod.rs` 第一屏就是：

```rust
pub mod auth;
pub mod projects;
pub mod tasks;
```

而且 `cats-bff` 一直能编译 —— 真没被声明的文件根本过不了 `cargo check`。

#### 根因

```python
cp = os.path.join(src_dir, *cand.split("/"))   # ← 恒定相对 src_dir
```

`mod` 声明**永远相对 crate 根目录**解析。走到 `src/upstream/mod.rs` 时，
`pub mod auth;` 被解析成 `src/auth.rs`（不存在）→ **从不进入 `src/upstream/`**。

#### 危害比「多报 3 个文件」大得多

它会**掩盖真正的孤儿**：只要一个 crate 用子目录组织模块，那个子目录的可达性
就是瞎的 —— 里面真躺着一个没被声明的文件，规则 7 也看不见。

修好后做了两种反向验证：

| 反例 | 期望 | 实测 |
|---|---|---|
| 子目录里塞真孤儿 `src/data/zz_negtest.rs` | FAIL 并点名 | ✅ EXIT=1 精确点名（**旧版本抓不到这种**）|
| 注掉 `pub mod smoke;` | 其文件变孤儿 | ✅ EXIT=1，点名 `src/smoke.rs` |

#### 由此撤回本文档的两条记录

1. **「`cats-mock` 2899 行孤儿，`lib.rs` 宣称四模块却一个都没声明」——假的。**
   `cats-mock/src/lib.rs` 一直声明着
   `pub mod data; pub mod db; pub mod http; pub mod infra; pub mod smoke;`。
   那 12 个文件（`data/*.rs` `db/*.rs` `http/*.rs` `infra/*.rs`）纯粹是这个
   bug 造出来的假账，已从 `ORPHAN_BASELINE` 整项移除。

2. **「孤儿存量 7 crate / 3963 行」→「6 crate / 558 行」。**

也就是说，前面几轮说的「接线进度还差 3963 行」**高估了约 7 倍**。真实剩下的
是 558 行，其中 303 行（`routes.rs` + `grpc_clients.rs`）属于下一节。

> 这一段的教训和 §4.1b 那句是同一句：**「检查通过了」和「检查看的是你以为的
> 那个东西」是两件事。** 规则 7 报告的每一个数字，都建立在「我的可达性算法
> 是对的」这个未经检验的前提上。

### §4.1d cats-bff 剩下的 303 行：一条被有意推迟的平行设计

`routes.rs` 与 `grpc_clients.rs` 不是「缺几个 API 就能接」的活。它们和
`upstream_passthrough.rs` 是**同一套设计**，而那套设计与正在跑的强类型设计
并存 —— 两个文件自己的头部都写明了这件事：

> 本文件属于另一条平行设计（`/api/v1/*` 前缀 + 裸透传），其依赖的
> `crate::upstream::UpstreamClient` 与 `crate::grpc_clients` 同为未接入文件，
> **接入前需先做适配改造**。

也就是说**这不是疏忽，是有意推迟**。第 1 步（`upstream_passthrough.rs`）已按
「对齐到共享 `Config` 的真实字段名」的方向接完（`5149a4a`），剩下两步的已知
卡点：

| 文件 | 卡点 |
|---|---|
| `routes.rs` | 依赖 `cats-proto`（manifest 里没有）；引用不存在的 `crate::upstream::UpstreamClient`；proto 类型名写成 `MatchTMRequest`，而 prost 规范化后是 **`MatchTmRequest`**（与 translation-core 接线时踩的是同一个坑）|
| `grpc_clients.rs` | 依赖 `tonic`（manifest 里没有）；`Config` 缺 `translation_core_grpc` 字段；`BffError` 缺 `From<tonic::transport::Error>` |

**风险提示**：这 809 行是照着**旧版 Config 形状**和 **`.proto` 里的类型名**
写的，编译器管不到的那类语义错误（字段含义变了没、RPC 语义对不对）不会在编译
期暴露。所以第 2/3 步要按 `upstream_passthrough` → `routes` → `grpc_clients`
的顺序做，每步单独验证。

### §4.1e cats-bff 全部接完，orphan 清零

**步骤顺序被调整过**：原计划 `upstream_passthrough` → `routes` →
`grpc_clients`，但 `routes.rs` 里有
`use crate::grpc_clients::{TmCommitAck, TmLookupResponse, TranslationClient}`
—— 它硬依赖后者，只能改成 `upstream_passthrough` → `grpc_clients` → `routes`。

第 2、3 步又翻出 4 个问题，**其中三个是"两条平行设计各写各的"才暴露的**：

| 问题 | 谁能发现它 |
|---|---|
| `UpstreamClient` 的 import 路径指向 `upstream/` | 编译器 |
| `proxy_status` 收 actix 的 `StatusCode`，而 `UpstreamClient` 返回 reqwest 的 | 编译器 |
| proto3 枚举字段是 `i32` 不是 `LanguageCode` | 编译器 |
| `MatchTMRequest` → prost 规范化后的 `MatchTmRequest` | 编译器 |

也就是说，**这 809 行里凡是编译器能抓的都抓到了；抓不到的（RPC 语义、
字段含义是否还对得上）仍然没有验证**，因为那需要真的跑一次
translation-core。

一个值得单独记的取舍：`From<tonic::Status>` 我**没有**图省事全部映射成
`DependencyUnavailable`（502）。那样会把上游的 `UNAUTHENTICATED` /
`PERMISSION_DENIED` / `NOT_FOUND` 一律谎报成 502 —— 调用方会以为"依赖挂了"，
而实际是"这个请求不该被允许"。现在逐 code 映射到 401/403/404/409/400/502。

**结果**：`ORPHAN_BASELINE` 里 `cats-bff` 整项移除。孤儿存量
**6 crate / 558 行 → 5 crate / 255 行**，剩下的全是各 service 孤立的
`state.rs`（`audit-service` 那种"四个文件互相引用、缺一不可"的情况已经没有了）。



### §4.2 audit consumer 仍是 no-op

`KAFKA_REST_URL` 没有对应的 REST proxy 容器，consumer 会退化成 30s 心跳
no-op。topic 建了但没人消费。已在 compose 注释里写明，不再靠
`KAFKA_BROKER` 假装接了 Kafka。

### §4.3 `/healthz` 的响应格式有 6 种，且 4 个 service 自报 `cats-common` 🆕

本轮 13 路探活的实际响应体（逐字）：

| service | 响应 |
|---|---|
| envoy | `ok`（纯文本，非 JSON） |
| auth-service | `{"service":"auth-service","status":"ok"}` |
| report-service | `{"service":"report-service","status":"ok"}` |
| user / project / file / notification / task | `{"name":"<svc>","status":"ok","version":"0.1.0"}` |
| audit / ai-gateway / worker / **translation-core** | `{"status":"ok","app":{"name":"cats-common","version":"0.1.0"}}` |
| cats-bff | `{"bind_addr":...,"service":"cats-bff","status":"ok","upstreams":{...}}` |

**6 种形状。** 任何统一监控/仪表盘都没法用一套解析器读全。

其中 audit / ai-gateway / worker / translation-core 这 4 个调的是
`cats_common::AppMeta::current()`，它返回的是 **cats-common 自己**的
`CARGO_PKG_NAME`，于是这 4 个 service 在 healthz 里都自称 `cats-common`，
版本号也是共享库的版本——认不出是哪个服务。

**更正**：本文件初稿写的是"12 个 service 的 healthz 全都自报
cats-common"，那是错的——另外 8 个自报的是自己的名字。真实情况是
**4 个自报错了 + 整体 6 种格式不统一**。

#### 已修：10 处不再自报 `cats-common`（2026-10-05，CI run `37281458555` 4/4 绿）

`AppMeta::current()` 返回的是 **cats-common 自己**的 `CARGO_PKG_NAME`。
`env!("CARGO_PKG_NAME")` 是编译期按**本 crate** 展开的，所以每个服务
改用 `env!` 后报的就是自己。跨 9 个 crate 改了 10 处调用点
（audit / translation-core / cats-ai-gateway 的 main.rs 与 api/mod.rs，
加 6 个 M0 占位 service）。`cats-common` 自己的单测保留
`AppMeta::current()` —— 它本来就该报自己。

顺带记一个坑：`cats-ai-gateway/src/api/mod.rs` 用 `serde_json::json!`，
不能在其值位置内嵌 struct 字面量再在里面写 `"key": value` ——
宏有自己的分词器，会把 `"name":` 当成 JSON 键值对解析。
必须先 `let app = AppMeta { .. };` 再放进 `json!`。

**仍未修**：6 种响应格式本身不统一。统一它要动 18 个 crate 的 handler
（"12"是当初从 §3.3「12 个 crate 共 5731 行未编译」串过来的，与本条无关；
§4.1m 已按实测补全为 18 个服务 / 4 种在用形状 / 另有一整套 `/readyz`）
并改共享结构，属于独立的 API 收敛工作，不在本次范围。



### §4.1f 剩下的 5 crate / 255 行：查清后删掉 6 个，只留 1 个 🆕

本节形成时，剩余孤儿已经不是“等着被接线的半成品”了。逐项调查后发现：
**5 个 `state.rs` 全部零引用，`storage.rs` 是被替代的旧稿**。接线不会让代码变活，
只会把“零调用方、门禁看得见”变成“零调用方、门禁看不见”。

#### 拍板来源（如实标注）

| 事项 | 503 |
|---|---|
| 方式 | `ask_user` 四项问卷，每项均标 `(推荐)` |
| 取得方式 | `responseSource: automatic_timeout` / `explicitUserConfirmation: false` |
| 结果 | 4 项**全部自动选中推荐项** |

**本次拍板未取得 Ulysses 的明确确认**，仅为超时自动选中推荐项。
下面的证据与实测结果可独立核对，但决策本身应当作待复核项。

#### 逐项调查结论

**1. `file-service/src/storage.rs`（63 行）—— 被 `handlers.rs` 内联重写取代，且语义冲突**

| | `handlers.rs`（活的） | `storage.rs`（孤儿） |
|---|---|---|
| 分区键 | `workspace_id` | `org_id` |
| 根目录 | 读 `FILE_STORAGE_ROOT`，缺省落到 `temp_dir()/file-storage` | 硬编码相对路径 `var/files/`（相对 cwd）|
| 文件名 | 纯 uuid + `.bin` | `{file_id}_{safe_name}`（拼用户传入的 name）|

权威是 **`workspace_id`**：`migrations/20260920_0001_init.sql` 建表用
`workspace_id` + `storage_path`，索引建在 `(workspace_id, sha256)` 上；
更早的两份迁移（`0001_init_file_db.sql` / `20260919_0001_init.sql`）
注释里明写 `org_id` 是“旧版约定、已被 workspace_id 取代”。

附带删掉 2 个**假测试**：

- `ensure_base_dir_succeeds` 算了路径、删了目录、设了环境变量，
  但**从未调用被测函数**——它永远不会红，与被测对象无关。
- `path_traversal_protection_strips_dots` 把过滤逻辑**抄了一遍**而不是调用 `write_file`，
  改生产代码它照样绿。

**2. `file-service` / `notification-service` / `project-service` / `report-service` 各自的 `state.rs`（17~24 行）—— 零引用**

`AppState` 只在文件内部出现，crate 内无任何其它引用。
4 个 crate 的 `main.rs` 都是**分别**注册 `web::Data::new(pool)` 和
`web::Data::new(rbac_checker)` 两个独立参数——活设计本来就不是“一个合并的 AppState”。

**3. `task-service/src/state.rs`（17 行）—— 权威已经定了**

与上面 4 个不同：`task-service` 的 `src/handlers.rs` **自己定义了**一个 `AppState`，
而它是活的——`web::Data<AppState>` 出现 **6 次**，`main.rs` 确实注册了
`web::Data::new(state)`。这个孤儿文件是给一个“已经存在且正在使用”的类型
又写了一份定义。

**4. `notification-service/src/consumer.rs`（69 行）—— 保留，不是旧稿**

它不是被取代的草稿：`process_event` 是真代码（反序列化 `KafkaEvent` →
`db::insert_from_event` 真落库），`kafka_event_round_trip` 是真测试。但 `run_consumer_loop`
依赖的 `poll_once` 恒返回 `Ok(0)`，接上去等于多 spawn 一个永远 5 秒一轮空转的任务；
`notification-service/main.rs` 目前完全没 spawn 任何 consumer。文件自己的文档注释写明
“真实 Kafka 集成留 Sprint 2”——**它自己声明是占位符**。
留在基线里正好避免“占位符被当成已完成”。

#### 【显式撤回】本文档两条技术断言——两条都是错的，都没被实测过

上面删除的论据很部分建立在我自己写过的两条断言上，而**这两条都没被实测过**。
本次补做了两个可回滚实验，结果是**两条都错**：

**错误 1（写在 §4.1 表格与本节之前的表格里）**：

> 原文：“只加 `mod` 会多出一个没人用的类型，workspace 严格 lint 下是
> `dead_code`（可能直接是错误）。不构成进展”

**实测**（从 git 恢复 `file-service/src/state.rs`、在 `lib.rs` 插入 `pub mod state;`，
然后 `cargo clippy -p file-service --all-targets -- -D warnings`）：

```
[result] exit=0
[result] dead_code 诊断条数 = 0
```

**原因**：`AppState` 是 `pub` 且从 crate root 可达，rustc 不会把它当死代码。

**这把删除的论据反过来加强了**：“加 `mod` ”不会让门禁变红，**它甚至不会发出任何警告**——
就是把“零调用方、门禁看得见”直接变成“零调用方、门禁看不见”。

**错误 2**（写在 §4.1「6 个 `state.rs` 的三种不同情况」表格里）：

> 原文：“**接上去会造出两个竞争的 `AppState`**，**编译期就冲突**”

**实测**（恢复 `task-service/src/state.rs`、加 `pub mod state;`，而 `handlers.rs` 里已有一个
正在使用的 `AppState`，然后 `cargo check -p task-service --all-targets`）：

```
[pre]  handlers.rs struct AppState x1, web::Data<AppState> x6
[result] cargo check -p task-service  exit=0
[result] error 行数 = 0
```

**原因**：两个同名类型在不同模块里属于不同路径（`state::AppState` vs `handlers::AppState`），
Rust 不会冲突。真正的问题是**概念上的重复定义**，不是编译错误。

#### 结果

- 删除 6 个文件（`file-service/storage.rs` + 5 个 `state.rs`），共 180 行
- 孤儿存量：**5 crate / 255 行 → 1 crate / 75 行**（仅 `notification-service/consumer.rs`）
- `ORPHAN_BASELINE` 同步移除这 6 项，并在脚注里写上逐项调查结论
- 本地验证：`cargo fmt --all -- --check` 0 / `cargo clippy --workspace --all-features
  --all-targets -- -D warnings` 0 / `cargo test --workspace --locked` 0 / `lint-compose.py` 0（`FAILCOUNT=0`）

### §4.1g `routes.rs` 终于有了运行时覆盖（不需要 Docker）🆕

§4.1e 记录了 cats-bff 三步接完、孤儿清零，但当时还留着一个尚未解决的问题：
**`routes.rs` 那 178 行从接进编译到此一直只做过类型检查**。

原因很简单：crate 里唯一跑 HTTP 的集成测试 `bff_smoke.rs` 打的是
`main.rs` 实际注册的那套 `handlers::*` 路由表（`/v1/*`），**打不到 `/api/v1/*`**。

#### 新增 `crates/cats-bff/tests/bff_routes_passthrough.rs`（13 个用例）

方法：起一个**真的 HTTP 上游**（`HttpServer` 绑 `127.0.0.1:0`，随机端口），
把 `UpstreamClient` 的两个基址指过去。这样新旧接口才能被真正验证：

| 断言的性质 | 靠什么证据 |
|---|---|
| URL 拼装正确 | 假上游**记下**自己收到的 method + path |
| body 原样透传 | 假上游记下收到的 JSON，逐字段比对 |
| 上游状态码原样透传 | 让上游回 401 / 503 / 201，断言 BFF **不是** 200/502 |
| `X-Cats-*` header 注入 | 假上游记下收到的 header 列表 |

与 `bff_smoke.rs` 的 "DEAD_URL + 断言走到哪一层" 策略**互补**：那边验证鉴权层，
这边验证透传层。**不需要 Docker**——不依赖本机 Docker Desktop 状态。

`translate/lookup` 的 gRPC 也不需要真服务：`source_text` 空值校验发生在调用
`TranslationClient` 之前，用 `connect_lazy()` 的 channel（不建立连接）就能满足提取器。

#### 验证测试本身有没有鉴别力（两个变异，都变红）

全绿不代表有效。对生产代码做两处故意破坏（实验自带回滚）：

| 变异 | 预期变红的用例 | 实际结果 |
|---|---|---|
| `proxy_status` 改成永远返回 200 | `upstream_401_...` / `upstream_503_...` | 变红：`rc=101`，`0 passed; 1 failed` |
| 去掉 `auth_base` 的 trailing-slash trim | `login_forwards_to_auth_service_v1_login_path` | 变红：`rc=101`，`0 passed; 1 failed` |

两处均已回滚，`git status` 回读确认生产代码完整。

#### 两个技术坑：不要用 actix 的 `test::start`

`actix_web::test::start` 需要 `macros` feature，本 crate 没开。改用 `HttpServer` 手动绑端口，
带上两个真正需要记下来的坑：

1. `Server` 没有 `addrs()` ——必须在 `.run()` **之前**从 `HttpServer` 上取地址。
2. **`HttpServer::run()` 返回的是 Future，不 spawn 它就永远不会监听端口**。
   只 bind 不动，假上游不会 accept，症状是 BFF 侧全部拿到
   `502 dependency_timeout`——与“上游没起来”完全一样，很容易误判为测试本身有问题。
   正确做法：`let h = server.handle(); rt::spawn(server);`。

#### 本地验证

```
cargo fmt --all -- --check                                     RC=0
cargo clippy --workspace --all-features --all-targets
    --locked -- -D warnings                                    RC=0
cargo test --workspace --locked                    EXIT=0  96 targets / 444 passed / 0 failed
python deploy/scripts/lint-compose.py                          RC=0
```

新套件单独跑：`cargo test -p cats-bff --test bff_routes_passthrough` → **13 passed / 0 failed**。

#### 环境事实：本机 24 核，`cargo test --workspace` 不能用默认并发数

原本判断是“其它 session 与我共享 target 目录互相抢锁”。
换了独立 `CARGO_TARGET_DIR` 之后**仍然退出码 -1**——说明那个假设不对。

真因：24 核机器上 cargo 默认 `-j 24`。大 workspace 会在**链接阶段**死掉 ——
每个 rustc 链接时占 1~2 GB 虚拟内存，而本机 commit limit 仅 58.5 GB，
已被其它 session 占掉约 36 GB。表现是**编译到链接阶段、零错误输出、
退出码 1 或 -1**，看起来像编译错误，实际是进程被系统杀了。

**修正：这类大 workspace 命令限定 `-j 4`。** 上面那个 EXIT=0 就是加上 `-j 4` 之后的结果。

### §4.1h 发现一个“能跑、但没镜像”的服务，并加门禁🆕

上两节（§4.1f / §4.1g）均在代码层。这一段是部署层：项目里
**有一个能产出二进制、也被 compose 声明为服务的 crate，却根本没有对应的镜像**。

#### 发现过程

对三份清单做差集：CI matrix 构建的 17 个镜像 vs `docker-compose-mvp.yml` 声明的
service vs `deploy/helm/` 下的 chart 目录，再对照 workspace 里谁有 `src/main.rs`（会产出可执行文件）。

```
产出二进制的 crate: 18 个
CI matrix 构建的: 17 个
缺的那一个: cats-ai-gateway
```

#### 为什么一直没人发现

因为**本地走的是另一条构建路径**，它把缺口藏了：

| | 本地 compose | CI 输出 |
|---|---|---|
| Dockerfile | `deploy/Dockerfile.runtime` | `deploy/docker/Dockerfile.rust` |
| 产出 | 单一 `cats-runtime:latest`，**含全部二进制** | 17 个 per-service 镜像推 Harbor |
| 结果 | `docker compose up` 一切都能跑，`ai-gateway` 强调 `command: ["/usr/local/bin/cats-ai-gateway"]` 也能跑 | —— **永远不会有 ai-gateway 镜像** |

`docker-compose-mvp.yml` 里那个 service 甚至留着一条评语记录了一个更早的同类事故：
它曾经没有 `command`，继承了 `Dockerfile.runtime` 的 `CMD ["/usr/local/bin/cats-bff"]`，
**叫 ai-gateway 的容器里跑的是 cats-bff**（现在已修）。

只有 CI 输出的 per-service 镜像会暴露这个缺口——而本地测一概组合的
`docker compose up` 永远是绿的，它们与 CI 输出的是两套不同的东西。

#### 修正

1. matrix 补入 `cats-ai-gateway`（已有 `src/main.rs`，compose 也在跑）。18 个镜像覆盖全部服务。
2. **加 lint 规则 8**，把这类静默漂移变成可发现：
   每个产出二进制的 crate 都必须在 `ci-docker-build` 的 matrix 里；
   反向也查（matrix 里的 crate 不再产出二进制 → FAIL）。
   以后新增任何服务 crate，未加入 matrix 就会 FAIL。
   唯一的例外 `m1-s0-smoke`（产出二进制但是烟雾工具，不是服务）在脚注里写了理由。

#### 规则 8 的鉴别力已实测（两个变异）

```
基线（不改）                    EXIT=0  全部通过
从 matrix 删掉 cats-ai-gateway          FAIL    精确点名它
往 matrix 塞入不存在的 ghost-service  FAIL  报为 stale
```

**写完第一版时这条规则是坏的**，它把全部 18 个 crate 都报成缺失，
正则测试一跑就会 EXIT=1。原因：正则写的是 `-\s+crate:`（同一行），
而 matrix 的 YAML 写法是 `- service: X` 换行后 `crate: Y`——**两者之间还乲着 `service:`**，
所以正则始终返回空集。改成按行匹配 `^\s*crate:` 后才对（3 -> 18）。

这张口的教训：**新规则必须变异验证，否则它可能是一个会把 CI 全部报红的写错的门禁**。
并且匹配要落在上游的**结构**（YAML 解析出来的值）而不是文本（这一条
本轮已写进记忆）。

### §4.1i 撤回我自己的错误论据，并换一个结构性理由真正回退 `--workspace` 🆕

本节记录一次**论据被证伪、动作被取消**的完整过程。`9f745e5` 把 Dockerfile 的编译层
从 `--bin ${CRATE_NAME}` 改成 `--workspace`。本轮我一度准备把它撤回
（脚本 `revert-dockerfile.py` 都写好了），**最后没有撤**。

#### 当时的论据，以及它为什么不成立

论据是：run `37314826034`（`14ec1b5`）跑到 36.4 min 时，18 个 job **全部**仍在
`Build and push`、0/18 完成；而"基线" run `37306407841` 有 7 个 job 成功。看起来改完更慢。

这个对照有两处硬伤：

1. **两边不可比。** `37306407841` 的 conclusion 是 **`cancelled`** —— 它是被
   `concurrency.cancel-in-progress` 杀掉的。那 7 个"成功"只是被杀之前的**中途快照**，
   不是它的最终成绩。拿一个进行中的 run 去比一个被取消 run 的中途快照，不构成对照。
2. **真正的基线不是那一个。** 查全量历史，目前**唯一一次完整成功**的 run 是
   `37298809794`（`fa21e896`，10-05 10:47:27 → 11:40:53）：17 job 全绿，
   墙钟 **53.3 min**，`Build and push` 单 job **27.4~53.0 min**，其中 7 个 ≤ 34.9 min。
   当前 run 在 36.4 min 时 0 完成，落在这个分布之内并不异常。

#### 顺带查清的一件事，比原结论有价值得多

**`ci-docker-build` 在本分支几乎从不跑完。** 全量 23 次 run 的结论分布：

```
cancelled  15
success     3      （10-03 两次、10-05 一次；最近一次即 37298809794）
failure     4      （全部在 10-02 ~ 10-03）
```

- **4 次 failure 已不是当前问题**：签名完全一致（`Login to Harbor` = `skipped`
  → `Build and push` = `failure`），且都发生在 10-02~10-03，此后两次完整 success 未复现。
- **15 次 cancelled 的机制**（这就是"几乎从不跑完"的原因）：
  - `concurrency.cancel-in-progress: true`，而单 job 需 27~53 min；
  - 本分支平均 **6~54 min** 就来一个 push，每次都把在跑的杀掉；
  - `paths:` / `paths-ignore:` **均为空** ⇒ 任何 push（哪怕只改文档）都拉起全部 18 个 docker job；
  - `max-parallel` **未设** ⇒ 18 个 job 同时启动；
  - `cache-to: type=gha,mode=max` **未指定 `scope`** ⇒ 18 个 job 争抢同一个默认 scope。
- **这解释了缓存为什么长期是冷的**：被取消的 job 走不到 `cache-to`，缓存导不出来。

所以 `9f745e5` 的 `--workspace` 改动——**方向是对的，但目前兑现不了**。
它要省时间，前提是缓存能跨 run 存活，而这个前提由 `ci-docker-build.yaml` 决定，
不由 Dockerfile 决定。这正是本轮要记的那条：**修法必须落在真正控制这件事的那一层**。

#### 最终仍然撤回了，但理由换了

先订正 `9f745e5` 留在注释里的两处不实：

| 原文 | 问题 | 已改为 |
|---|---|---|
| "17 个 job 的这一层 cache key 互不相同 → **每个都永远 miss**" | "永远"过头。`ENV CRATE_NAME=${CRATE_NAME}` 那层确实让 18 个 job **彼此**无法共享，但实测最近 7 次连续提交里有 **3 次 `crates/` + `Cargo.lock` 完全没变**（`9f745e5→14ec1b5`、`4d988f5→9f745e5`、`0606ca2b→2612e1a8`），跨 run 暖缓存在原理上可行 | 只说"18 个 job 彼此无法复用编译层"，并注明真正让缓存长期为冷的是上面的取消机制 |
| 引 run `37306407841` 的耗时当"改前基线" | 那是**被取消**的 run，不能当基线 | 改引唯一完整成功的 `37298809794`（53.3 min / 单 job 27.4~53.0 min） |

**然后这个改动仍然被撤回了 —— 但依据与最初那个无关。**

最初的理由（"改完更慢"）如上所述不成立。真正成立的理由是一条**结构性论证**，
不依赖任何一次 run 的耗时：

`--workspace` 的唯一收益，是让 18 个 job 共享同一个 cache key。
这个收益有一个前提：**缓存必须真的能跨 run 存活**。该前提不成立：

1. 被 `cancel-in-progress` 取消的 job 走不到 `cache-to`，缓存导不出来
   （实测 23 次 run 里 15 次被取消）。
2. 唯一一次完整成功的 run `37298809794` 全程 `CACHED` 计数为 **0**。
3. 最近 7 次连续提交里有 4 次改动了 `crates/` 与 `Cargo.lock`，
   即使某个 run 跑完，`COPY crates` 那层也会随之失效。
4. 即便前三条都解决了：**同一轮里 18 个 job 同时启动、共享同一个还空着的
   cache key ⇒ 全部同时 miss**。单轮内根本不存在"先编一次、其余命中"。

于是：**收益一次也没兑现，代价（冷缓存下每 job 从编 1 个二进制变成编全部 20 个）
每次都付。** 收益不兑现而代价照付的改动是纯亏损，故撤回。

这条论证的**顺序**值得记住：先证明**收益不成立**，再看代价。
顺序反了就会变成"反正更慢所以撤回"——那又是一次拿未经证实的对照
去支撑破坏性决策，正是本节开头刚撤回的那个错误。

#### 撤回是忠实的（已验证）

当前 `Dockerfile.rust` 的**指令序列**与 `9f745e5` 的父提交 `2612e1a8`
**逐行完全一致**（各 21 条指令）。该比对另做了两道反向验证：
`9f745e5` 与 `2612e1a8` 的指令确实不同（否则"相等"毫无信息量）；
现在与 `9f745e5` 确实不同（否则撤回没生效）。

上表之外的**实现要点**仍然记录在案，但要按当前 per-crate 形态读：
**若将来再次改成共享 cache key 的形态，builder 阶段就不得出现
`ARG/ENV CRATE_NAME`** —— `ENV X=${X}` 的 digest 随 ARG 取值而变，
会让 18 个 job 的父链摘要重新分裂，共享 cache key 白做。
当前是 per-crate 形态，builder 阶段本来就需要 `ARG CRATE_NAME` + `ENV CRATE_NAME`；
`ARG CRATE_NAME` 在 runtime 阶段再声明一次，供 `COPY` 与 `ENTRYPOINT` 使用。

#### 订正用到的断言做了变异验证（4/4）

```
未变异                                        PASS
把 ENV CRATE_NAME 塞回 builder 阶段            FAIL  ENV leaked into builder
把 ARG CRATE_NAME 塞回 builder 阶段            FAIL  ARG leaked into builder
退回 per-crate build（= 本会话原本打算做的撤回）  FAIL
只改注释、指令不动                            PASS  ← 证明断言没被注释文本污染
```

写这条断言时它**第一次就是错的**：直接对全文做 `ENV CRATE_NAME not in text` 子串判断，
而本文件的注释里恰好引用了旧版的 `ENV CRATE_NAME=${CRATE_NAME}`，于是匹配到了注释。
改成先滤掉注释行、只在**指令行**上断言才对（与 §4.1h 的 lint 规则 8 同一个坑）。

#### 顺带把单 job 的耗时构成测准了（同一 run 内自洽）

基线 run `37298809794` 里最快的 `ingestion-service`，两步耗时：

```
#26 [linux/amd64 builder 9/9] RUN cargo build --release --bin ingestion-service   161.3s
#32 [linux/arm64 builder 9/9] RUN cargo build --release --bin ingestion-service  1506.8s
```

- `161.3 + 1506.8 = 1668s ≈ 27.8 min`，**正好等于该 job 的 `Build and push` 总耗时**
  → 单 job 耗时几乎全部是这两步，没有别的地方可以省。
- **arm64 占约 90%**（`1506.8 / 161.3 = 9.3×`），QEMU 模拟的代价。
- 该 run 全程 `CACHED` 计数 **0** —— 没有任何一层命中缓存，与"缓存长期是冷的"一致。

这给 `--workspace` 改动一个**可检验的定量预测**（尚未证实，只是预测）：
冷缓存下 `--bin <单 crate>` 只需编 1 个二进制，`--workspace` 要编全部 20 个，
所以单 job 的编译步应当变长，arm64 尤甚；而它换来的"共享 cache key"在本仓库
兑现不了（见上）。两者相抵，净效果是变慢。**但这是预测，不是结论** ——
`37314826034` 未跑完前不下判断。

#### 撤回已被 CI 实测验证（`8fbc141` / run `37328644837`）

推送撤回后的代码，得到 `8fbc141` 上 7 个 workflow 的结果：

```
ci-docker-build   success   墙钟 69.8 min   18/18 完成
ci-rust-test      success   9.5 min
ci-rust-build     success   12.7 min
ci-rust-clippy    success   4.4 min
ci-rust-deny      success   3.4 min
ci-rust-fmt       success   0.3 min
ci-helm-lint      success   1.8 min
```

**`--workspace` 那一次（`37314826034`）始终没有跑完**——78.5 min 时 18 个 job 全部仍在
`Build and push`、0 完成，随后被下一次 push 取消；单 job 已跑到 72.0~76.8 min，
是基线最慢 job（53.0 min）的 1.36 倍。

| | 基线 `37298809794`（per-crate, 17 job） | `--workspace` `37314826034`（18 job） | **撤回后 `37328644837`（per-crate, 18 job）** |
|---|---|---|---|
| 结论 | `success` | 未完成，0/18 | **`success`，18/18** |
| 墙钟 | 53.3 min | > 78.5 min | 69.8 min |
| 单 job `Build and push` | 27.4~53.0 min | 72.0~76.8 min（未完成） | 27.8~60.9 min |

这个对照**不是严格的受控 A/B**：撤回后那次是 18 个 job（多一个 `cats-ai-gateway`）、
源码也不同、墙钟 69.8 min 也确实比基线慢。所以它证明的是
「**per-crate 形态能跑完 18/18，而 `--workspace` 形态一次都没跑完**」，
不是精确的倍数关系。结构性论证仍然是主要依据。

#### 顺带把"缓存"这件事说准（订正本文档自己的一处过头表述）

撤回后那次 run 的 `CACHED` 计数是 **10**，而基线那次是 **0**。逐层看：

```
撤回后 #17 ~ #26  CACHED   （10 层：apt-get 与元数据 COPY 层）
撤回后 #31 / #32  非命中   （[linux/amd64] / [linux/arm64] 的 RUN cargo build）
基线   全部非命中
```

所以准确的说法不是"缓存长期是冷的"，而是：

> **便宜的层会命中，昂贵的那层从不命中。** 因为 `COPY crates ./crates` 就在编译层
> 之前，而 crate 源码在绝大多数提交里都会变 ⇒ 编译层的前置摘要跟着变 ⇒ 编译层失效。
> 这也再次说明：`--workspace` 想要共享的那个编译层，**在任何提交频率下都共享不到**。


#### 撤回本次会话中一个"看起来在做、其实没在做"的检查

记录这次取证时我自己踩的坑，留作后续同类操作的门禁：

- 查 `37298809794` 的分层耗时时，我写的正则要求 `#N [平台 阶段]` 形式，
  匹配到 **0 行**。若就此收手，就会得出"日志里没有这些层"的结论。
  实际日志是 `#1 DONE 4.1s`（无平台前缀）**与** `#26 [linux/amd64 ...]` 两种格式混排。
  打印原始行才看清。**0 命中必须先证明模式对，不能默认"目标不存在"。**
- 查 job 耗时时，`completedAt` 对进行中的步骤是哨兵值 `0001-01-01T00:00:00Z`
  而非 `null`，我的真值判断放行了它，日期解析到年份 1，于是打印出一串
  `-1065446712.9 min` 的负数。**哨兵值不等于空值**，要用 `now - startedAt` 兜底。
- `gh` 的一次 404：`jobs[].databaseId` 是 **job id**，`gh run view <job-id>` 会把它当
  run id 去查 → 404。取 job 日志要用 `gh run view <run-id> --job <job-id> --log`。

#### 待排期（本 PR 不做）

`ci-docker-build.yaml` 属 PR #19 的改动范围，按排期单独处理。按预期收益排序：

1. 给 `ci-docker-build` 单独的 concurrency group 或 `cancel-in-progress: false` —— 让 27~53 min 的长 job 有机会跑完，**这是其余一切的前提**（不跑完就没有基线，缓存也导不出来）。
2. 加 `paths:` 过滤（`deploy/docker/**`、`Cargo.toml`、`Cargo.lock`、`crates/**`、`proto/**`）—— 纯文档 push 不再拉起 18 个 job。
3. 设 `max-parallel` —— 消掉冷缓存踩踏。
4. `cache-to` 加 `scope:`，或改结构为「1 个 build job 编 workspace + 18 个 job 只做装配」，让共享 cache key 真正兑现。
5. arm64 走 QEMU 慢约 10 倍（144.5s vs 1425.3s）—— 根治要原生 arm64 runner 或交叉编译。

### §4.1j 清掉 22 个零引用 `pub` 函数，顺带挖出 2 个真 bug 🆕

§4.1f~§4.1i 清理的是"文件级"孤儿（整个 `.rs` 从未被编译）。这一节是**函数级**：
文件在编译，`pub` 函数也在编译，但**全仓库零调用方**——既没有生产调用，
也没有任何测试引用（`#[cfg(test)] mod tests` 里的调用一样算引用）。

#### 扫描器本身先修过两次

前两版有**系统性缺陷**，都会把"目标不存在"误报成"函数是死的"：

1. 调用点正则要求名字前有 `.` 或 `::`，**漏掉裸调用** → `find_by_user_id` 被误报。
2. 引用统计只扫 `tests/` 目录，**漏掉内联 `#[cfg(test)] mod tests`** → `drain_events` 被误报。

修掉后收敛到 **24 个真·全仓库零引用的 `pub` 函数**；其中 `run_migrations` 与
`task-service::rbac::principal_id` 在随后两节被单独处理，剩 **22 个**。

#### 处置结果：20 删 + 2 接线，0 抑制

| 处置 | 数量 | 代表 |
|---|---|---|
| 删除 | 20 | `cats-mock` 的 `weak_passwords` / `with_password_hash` / `audit_only` / `users_only` / `translation_unit_schema` / `err_with_msg` / `auth_user_only` / `set_ex`；`m1-s0-smoke` 的 `start_mock_server`；`cats-rbac` 的 `proxy_reason` / `to_error_code`；5 份 `principal_id`；`cats-ai-gateway` 的 `with_providers` / `with_retry_policy`；`cats-bff` 的 `for_test_grpc`；`task-service` 的 `task_type_enum` |
| **接线** | 2 | `cats-ai-gateway::service_for_test` 接进 `quota_exceeded_short_circuits_before_provider_call`；`task-service::events::snapshot` 接进 `publish_stage_progress_then_subscribe_returns_event` |

**全程没有用任何 `#[allow(dead_code)]` 之类的抑制。** 本次 diff 的 74 条新增行里
经程序逐行检查，抑制注解数为 **0** —— 在这个项目里"用抑制把门禁刷绿但代码仍无人用"
算缺陷，不算修法。

那 2 个选择接线而非删除，是因为它们本来就在**等一个调用方**：
`service_for_test` 是 `*_for_test` 辅助函数，内联测试本来就该用却手搓了一遍等价代码；
`snapshot` 标着 `#[cfg(any(test, debug_assertions))]`，是刻意留的调试设施。

#### 子代理的一处更正值得留着

派工时我基于 `task-service` 的代码断言"`user_id` 恒为 `None`，所以 `principal_id()`
只可能返回常量 `"anonymous"`"。子代理查证后发现**这只对 5 份中的 1 份成立**：
`file` / `notification` / `project` / `report` 四个 crate 真的会从
`cats-role:<uuid>:<roles>` 这种可选形式里解析 `user_id`。5 份函数体确实字节相同
（SHA256 一致），但"只可能返回常量"是错的。**5 份仍然全部零引用、全部删除**，
只是每个文件的说明改成了该 crate 自己的真实理由，而不是照抄我那个不成立的断言。

#### 挖出并修掉的 2 个真 bug

**① `MockRedis` 的 TTL 完全不生效**（`crates/cats-mock/src/infra/redis.rs`）

```rust
// 修复前
fn is_expired(ttl: Duration) -> bool {
    let _now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    // 算了个 _now 又丢掉
    ttl.is_zero()
}
```

`expire(key, 60)` 存进去的是**相对时长** 60s，而判定只看 `is_zero()` ⇒
**非零 TTL 的 key 永远不会过期**。配套测试 `expire_returns_bool` 只断言 `expire()`
的返回值、压根不查 key 是否过期，所以一直是绿的。

改为在 `expire()` 时记下**绝对截止时刻**（`Instant::now() + ttl`），
`is_expired` 判 `Instant::now() >= deadline`。

**② `EventBus::snapshot()` 持锁调 `view()`，必然自死锁**（`task-service/src/events.rs`）

`snapshot()` 持有 `last_state` 锁时调用 `self.view()`，而 `view()` 会**再取一次**
`last_state`；`std::sync::Mutex` 不可重入 ⇒ 每次调用都死锁。原先因为零调用方而从未触发。

修法：锁内只收集 task id，出块释放锁后再逐个 `view()`。

复核过锁序：`publish_stage_progress`（`last_state` → `sender_for` → `senders`）与
`view`（`last_state` → `senders`）是**同一顺序**，`sender_for` 单独只锁 `senders`、
不碰 `last_state`，**没有反序**。所以这次修复是恢复既有纪律，不是引入新顺序。

#### 验证（含一次我自己踩的坑）

```
cargo fmt --all -- --check                                       exit 0
cargo clippy (10 个受影响 crate, --all-targets, -D warnings)      exit 0
cargo test  (同上 10 个 crate)                                    exit 0
    41 个 test target / 319 passed / 0 failed
零引用扫描器                                                       0
```

新增的 3 个 redis 用例做了**变异验证**。这里有两条值得记：

- **第一版测试抓不住这个 bug。** 我最初只测了 TTL=0，复核时才发现旧实现下
  `Duration::from_secs(0).is_zero()` 恰恰为 true，TTL=0 本来就会过期；
  真正的 bug 是**非零 TTL 永不过期**。判别性用例是"过去的时刻必须判为过期"
  和"TTL 走完后 key 读不到"。
- **第一次变异验证是假阳性。** 我把 `is_expired` 的形参从 `Instant` 改回 `Duration`
  来复现旧实现，结果**根本编译不过**，cargo 同样返回 101 —— 变异"被杀"了，
  但没有任何测试真正运行。**编译不过的变异证明不了测试的鉴别力。**
  改成保持类型不变、只把函数体换成 `false`（语义等价于旧 bug）后：

```
基线                                       PASS
M1  is_expired 恒 false（旧 bug 语义）        3/4 新增用例变红
M2  is_expired 恒 true（反向过度修正）         3/4 新增用例变红（含 M1 抓不到的那条）
```

两个方向都抓到，说明用例不是在检测"任何改动"。

#### 遗留（本节不处理）

- `crates/cats-mock/.aci.json:195` 仍对外宣称有 `translation_unit_schema`，
  且**没有任何测试校验 `.aci.json` 的文本与代码一致**。
- ~~`UserFactory.password_hash` 恒为 `None`，喂着一条死分支~~ —— **2026-10-06 已修**（§4.1l）。
- ~~`cats-ai-gateway` 的 `retry_policy` 没有配置入口~~ —— **仍存在，但已在代码里写明**
  （§4.1l）：字段上注明它恒为 `RetryPolicy::default()`、要可配需要补 `Config` + env +
  Helm values + compose，属新增功能而非死代码清理，故本次只记录事实、不擅自加配置面。
- 零引用扫描器本身还放在临时目录、**未入库**，因此没有成为常设门禁；
  它的 3 条自检夹具已因本节改动而过期（`run_migrations` 已删、`configure_app` 已接线、
  `snapshot` 已接线），下次复用前须先更新夹具。

### §4.1k 把"零引用 pub 函数"变成常设门禁（lint 规则 9）🆕

§4.1j 清掉的 22 个函数当时是用一个**放在临时目录的脚本**找出来的。
脚本不进仓库 ⇒ 没人会再跑它 ⇒ 死代码会长回来。本节把它变成 `ci-rust-test`
里那一步真正执行的门禁。

#### 先查清一件差点搞反的事

我最初 grep「`lint-compose.py` 有没有被 CI 引用」，返回 **0 命中**，
于是准备按"它没接进 CI"来写。**结论方向完全反了**——
`ci-rust-test.yaml:77` 有一步 `run: python deploy/scripts/lint-compose.py`。

真相是机制：`D:\CATs\.gitignore:31` 忽略了 `.worktrees/`，
而我整个会话的工作目录 `D:\CATs\.worktrees\e2e-real-pg\` **正在里面**，
ripgrep 遵守 gitignore ⇒ 整个目录被静默跳过，工具返回 0 却不提示。

> 只要工作目录在 gitignore 覆盖的路径下，workspace 根的 grep 查的就是另一棵树。
> 症状是"我确信目标存在但 0 命中"。**0 命中 + 确信目标存在 = 先怀疑工具的搜索根**，
> 用 `read` 或 python 直接读一次做交叉验证，成本远低于基于错的 0 写一篇文档。

#### 规则 9 的三个设计决定

规则 7 管"文件级"孤儿（`.rs` 从未被编译），规则 9 管**函数级**：文件在编译、
`pub` 函数也在编译，但没有任何调用方——既无生产调用，也无任何测试引用
（内联 `#[cfg(test)] mod tests` 与 `tests/` 目录同样算引用）。

**① 必须自带"扫描器有效性"断言。**
若 `FN_DEF` 正则失效导致一个 `pub` 函数都收集不到，零引用清单自然是空集，
`found - baseline` 与 `baseline - found` **两条比较都会得到空集 ⇒ 门禁恒绿**。
这与规则 8 第一版的失效方式**完全同型**（正则恒返回空集，§4.1h）。
所以规则 9 带一条硬断言：

```
规则 9 失效：只收集到 N 个 pub fn（阈值 120）——扫描器大概率没在查任何东西
```

实测：把正则改坏后收集到 **0** 个，门禁立刻以 exit 1 报出上面这句话。

**② 一次遍历收集全部标识符出现位置**，而不是每个候选名重扫一遍全仓库。
早前那版对每个 `(名字, 文件)` 都重算一次 `#[cfg(test)]` 块边界，
是 `O(defs × files × lines)`；入库后必须在 CI 的每台 runner 上跑，
所以改成"先扫一遍收 `occ[name] = [(文件下标, 行号)]`，再分类"。
当前实测：**278 个定义 / 163 个文件，耗时 0.3s**。

**③ 双向基线**，与规则 7 的 `ORPHAN_BASELINE` 同一形状：
`found - baseline` 新增即 FAIL（必须删或接线）；
`baseline - found` 也 FAIL（基线条目已不成立，提示删掉，否则基线会腐烂）。
`ORPHAN_BASELINE` 里的未编译文件**不参与**规则 9——它们由规则 7 记账，重复报是噪声
（这也是扫描文件数 163 而不是全仓 164 的原因）。

FAIL 信息里明确写了"**不要用 `#[allow(dead_code)]` 把门禁刷绿**"。

#### 变异验证 4/4

```
基线                                              PASS  (278 个定义 / 163 文件 / 0.3s)
M1 真的塞一个没人调用的 pub fn                    FAIL  精确点名 + 给出文件:行号
M2 把 FN_DEF 正则改坏（扫描器变瞎）                FAIL  "规则 9 失效：只收集到 0 个"
M3 往基线塞一条实际不存在的条目（基线腐烂）         FAIL  "请把它从基线里删掉"
M4 只改注释、指令不动                             PASS  ← 证明未被注释文本污染
```

M2 是最重要的一条：**它对应的是"门禁在自己失效时仍然显示绿灯"这一类缺陷**，
而这类缺陷不会在任何一次成功运行里暴露，只能靠故意破坏来证明不存在。

#### 顺带记一条性能经验

早期那版扫描器对每个候选函数名都要 `re.compile` 一次并全量重扫；
入库后必须让 CI 端到端可接受。把"多次全扫"换成"一次收集 + 查表"之后，
278 个定义 / 163 个文件只要 0.3s。**把一次性排查脚本变成常设门禁时，
扫描成本必须一起重新设计**——它从"偶尔跑一次"变成"每次 push 都跑"。

### §4.1l 收掉 §4.1j 留下的三处遗留 🆕

§4.1j 的子代理在报告末尾列出了三条"发现了但超出我的文件所有权"的问题。它们当时没有
归属人，本节处理。

#### ① `UserFactory.password_hash` 的恒假分支

子代理删掉 `with_password_hash()`（全仓零调用）之后，工厂侧的
`password_hash: Option<String>` 只剩 `new()` 里写死的 `None` 一个写入口，于是
`build_one` 的 `self.password_hash.clone().unwrap_or_else(|| …)` 变成
**只有 else 分支可达**。

把字段与那条分支一起删掉，并把原来那个 `format!` 提为常量——它的两个实参都是字面量，
结果是一个恒定字符串，却每次 `build_one` 都要重新拼一遍：

```rust
const FAKE_PASSWORD_HASH: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$fakesalt0123456789$fakehash0123456789";
```

**一处误读留个记录**：我一度以为 `as_db_row` 里那行
`password_hash: self.password_hash.clone()` 也是死的。核实后不是——那是 `User`
自己的 `String` 字段，与工厂那个 `Option` 是两个东西。

#### ② `.aci.json` 声称存在一个已删函数

`crates/cats-mock/.aci.json` 里 `schema` 模块的 description 写着
"SchemaSet + 5 schema fns: … / translation_unit_schema"，而
`translation_unit_schema` 在 §4.1j 已被删除，源码里现在只有 4 个 schema fn。
订正为 4 个，并断言"描述里声称的 schema fn 集合 == 源码里实际存在的集合"。

**相邻的一份文件刻意没动**：`crates/cats-mock/docs/regression-report-stage1-module-switch-2026-09-25.md`
第 32 行同样写着 5 个，但那是**带日期的历史记录**（`日期: 2026-09-26 JST`），
它对当时的描述是准确的。按"禁回溯叙事"的规矩，不把历史记录改成今天的样子。

#### ③ `retry_policy`：写明事实，但不擅自加配置面

`AiGatewayService.retry_policy` 恒为 `RetryPolicy::default()`
（`max_attempts_per_provider=3` / `base_backoff_ms=100`），因为 `with_retry_policy`
已被删除且没有任何 env / `Config` 入口。

判断：**这是能力缺口，不是缺陷**。重试本身工作正常，删掉可调性比重试更糟；
而"把参数做成可配"要补 `Config` 字段 + env + Helm values + compose，
是**新增功能**，不在死代码清理的范围内。所以在字段上写明事实，
免得下一个读代码的人误以为它已经可配：

> `retry_policy` 当前恒为 `RetryPolicy::default()`……要做成可配需要补
> `Config` + env + Helm values + compose，属新增功能，不在死代码清理范围内。

#### 验证

```
cargo test  -p cats-mock --lib                      98 passed / 0 failed
cargo clippy -p cats-mock -p cats-ai-gateway
             --all-targets --locked --offline -D warnings     exit 0
cargo fmt --all -- --check                                    exit 0
lint-compose.py（9 条规则）                                    exit 0
```

#### 顺带记一条工具链故障与它的双重伪装

本机 `E:\DevCache\cargo\` 整棵树（含 `bin/` 与 `target/`）突然变成**执行被拒**，
但目录可列、文件可读、`.rustup` 下同一份工具链正常 ⇒ 按路径判定的执行授权问题。

它伪装了**两次**，两次都差点让我得出错误结论：

1. `cargo test … | Select-Object …` 报 `ResourceUnavailable ... 拒绝访问`，
   而 `$LASTEXITCODE` 给出了 **0**——看起来"测试通过"，实际是
   **cargo 一次都没被启动**（PowerShell 在启动原生进程之前就失败了）。
2. 换 `.rustup` 的 cargo 后又炸在 `could not execute process … build-script-build
   (never executed)`，这个文本**长得像 build script 写错了**，
   实际是 target 目录执行被拒。
   且 `cargo-clippy` / `cargo-fmt` 子命令是从 `CARGO_HOME`（也指向被拒的路径）
   解析的，换 cargo 二进制**并不够**。

可用组合（`--offline` 需要 registry，所以不能改 `CARGO_HOME`）：

```powershell
$TC = "$env:USERPROFILE\.rustup\toolchains\1.98-x86_64-pc-windows-msvc\bin"
$env:CARGO_TARGET_DIR = 'D:\…\.target-verify'      # 必须一起覆盖
cmd /c "`"$TC\cargo-clippy.exe`" clippy … --locked --offline -j 6 -- -D warnings"
cmd /c "`"$TC\cargo-fmt.exe`" fmt --all -- --check"
```

**判据：`$LASTEXITCODE` 为 0 不等于命令跑过了。** 任何经管道的 native 命令都不能用
它判成功——要么不接管输出，要么把结果落文件再单独读。
### §4.1m 把 §4.3 的"6 种响应格式"补全 —— 实测 18 个服务、4 种在用形状，外加一整套 `/readyz` 🆕

§4.3 的结论来自 13 路运行时探活，那个测量本身没问题。**有问题的是它隐含的
"已经看全了"** —— 本节做静态逐 crate 取证，补上三件 §4.3 没覆盖的事。

#### §4.3 漏掉的三件事

**1. 它只探到 12 个服务，实际有 18 个注册了 `/healthz`。**

没被探到的 6 个是 asr / ocr / ingestion / subtitle / office-converter /
render-writer —— 它们的 healthz 全都是 §4.3 表格第 4 行那种形状。

**2. "6 种形状"里有一种不是服务。** envoy 的 `/healthz` 是纯文本 `ok`
（`deploy/envoy-mvp.yaml:34-35` 的 `direct_response`，根本不代理），
把它和 JSON 形状并列计数，会让"统一 healthz 格式"这个目标本身变得不清楚：
envoy 那一行永远不需要改。所以**服务形状是 4 种在用 + 1 种被遮蔽**（见 §4.1n）。

**3. `/readyz` 完全没被覆盖。** audit-service 与 worker-service 各开了一个，
形状是 `{status: ready|not_ready, service, db: ok|fail}`，两者一致，且都真连库
（`sqlx` 查 `SELECT 1`）。§4.3 的 13 路探活没打这两个端点。

#### 4 种在用形状（按 handler 函数体逐个读出来的，不是猜的）

| # | 形状 | 数量 | 服务 |
|---|---|---|---|
| A | `{status, app:{name, version}}` | 8 | asr / ingestion / ocr / office-converter / render-writer / subtitle / translation-core / audit |
| B | `{status, name, version}` —— 扁平，键叫 `name` 不叫 `service` | 5 | file / notification / project / task / user |
| C | `{status, service}` —— **没有 version** | 3 | auth（内联 `json!`）/ report（自带 struct）/ worker（自带 struct） |
| D | `{status, service, version, bind_addr, upstreams:{5 个上游 URL}}` | 1 | cats-bff（`handlers.rs:140`，即生产在用的那个） |

B 这一类是最容易被漏掉的一种：键名是 `name` 而不是 `service`，所以任何按
`.service` 取值的监控脚本会在**这 5 个服务上静默拿到 null**，不报错。

全部 18 个服务的 `/healthz` 都是**无认证**的（注册在所有 `web::scope("/v1/…")`
之外，符合 K8s 探针惯例），这一点是一致的。

#### 取证方法：以及我自己栽的四次

解析 `/healthz` 路由 → 定位 handler → 读出实际返回的键。四次失败同一个形状：
**看到 0 或整齐划一的结果，先怀疑解析器。**

1. 正则 `"/healthz"[^)]*?\.to\(([^)]*)\)` —— `[^)]*?` 跨不过 `web::get()` 里的
   右括号，一个路由都没匹配上。差点据此写"没有任何 handler 定义"。
2. handler 定义在全仓找、命中即 `break`，于是 **16 个服务全被解析成
   `asr-service/src/main.rs:34` 的同一个定义**。改成限定同 crate 才对。
3. 提取键时读了函数起点往后 1400 字符，于是**每个 handler 的键集合后面都拖着
   隔壁函数的键**（`error`/`message`/`detail`/`db`/`rbac` 全是串过来的）。
   形状分类器据此判出"shape A 只有 1 个"这种明显荒唐的结果 —— 荒唐值本身
   就是信号。
4. 用 PowerShell 重定向导出时中文全成 `���`，差点判定"源文件编码坏了"。
   那是 stdout 编码问题，不是文件问题：改用 `PYTHONIOENCODING=utf-8` 后原样输出。
   **在断定文件有问题之前，先确认自己这一侧的读取方式。**

#### 探针配置的真实数量：28 份，不是 18

`deploy/` 下引用 healthz 的配置实测：

```
helm values.yaml          18 份（每个服务一份，外加 cats-common）
k3s 服务清单              10 份（deploy/k3s/cats-core/）
k3s envoy 自身             1 份（deploy/k3s/cats-edge/）—— 不是后端探针
docker-compose-mvp.yml    11 处 healthcheck / depends_on
envoy-mvp.yaml             1 处（direct_response）
```

**服务探针配置 = 18 + 10 = 28 份。** 此前记的"18 份"是 helm 单独的数，漏了 k3s。

#### cats-bff 那两个多余字段：结论仍是"低"，但多了一条依据

`cats-bff` 的 healthz 比别人多吐 `bind_addr` 和 5 个上游 URL。逐条查了可达性：

```
全仓有没有东西读 upstreams / bind_addr
  -> 只有 BACKEND_STATUS_v0.4.md:431 那行文档自己，没有任何代码或脚本读

envoy 的 /healthz 是什么
  -> deploy/envoy-mvp.yaml:34-35 是 direct_response，envoy 自己应答，不代理
  -> 且路由表里**没有 cats_bff 这个 cluster**（8 条 route 全部指向后端服务）

cats-bff 的端口映射
  -> deploy/docker-compose-mvp.yml 是 "127.0.0.1:8091:8080"，只绑回环
```

**所以它今天不是对外泄露。** 我在查到 envoy 配置之前一度判断成
"公开入口可读到内部拓扑"，那是错的。残留风险只有一条且是假设性的：有人把
`"127.0.0.1:8091:8080"` 改成 `"8091:8080"`（"让它能访问"是最常见的改法），
这些字段就变成宿主网络可达的内部拓扑披露 —— 而没有任何东西读它们。

**处置：不单独改。** 统一到形状 A 之后它们自然消失；单独削掉虽是一行改动，
但会让"统一"这件事做两遍。

#### 为什么仍然不做

统一 healthz 要动 18 个 crate 的 handler + 28 份探针配置。探针只判 200 不看
body，所以改 body **不影响探针**；但它是**对外 API 形状变更**（任何读
`.status` / `.service` / `.name` / `.app.name` 的外部监控都要跟着改），属产品
决定，不该由我自行拍板。故本节只交付取证与建议。

建议的目标形状（若决定做）：

```json
{ "status": "ok", "app": { "name": "<CARGO_PKG_NAME>", "version": "<CARGO_PKG_VERSION>" } }
```

取形状 A 的理由：它已经是 8/18 的既成事实，且它是**唯一一种能同时表达
"哪个服务 + 哪个版本"** 的形状 —— B 和 C 都缺 version（C 连 name/service 都有
但没有版本），所以外部监控没法用它判断"跑的是哪次构建"。

**不要再踩回去的坑**：`asr-service` / `ingestion` / `ocr` 等 7 个文件的注释
记录了为什么用 `env!` 而不是 `AppMeta::current()` —— 后者返回的是
**cats-common 自己**的包名，会让所有服务自报 `"cats-common"`（§4.3 修掉的那个坑）。

---

### §4.1n 两条假绿：`/healthz` 被注册了两次，而 13 个用例测的是服务器不走的分支 🆕

这一节的两条都不是"发现了一处不一致"，而是**发现了一处看起来全绿、实际什么都没验的地方**。

#### 一、cats-ai-gateway 把 `/healthz` 注册了两次

```
crates/cats-ai-gateway/src/main.rs:178   .route("/healthz", web::get().to(healthz))
crates/cats-ai-gateway/src/api/mod.rs:168 .route("/healthz", web::get().to(healthz_handler))
                                        ↑ 经 main.rs:179 的 configure_routes() 注册
```

两个 handler 都在编译、body 都对，但**actix-web 对同一路径注册两次时是先注册的赢**。
这条不是查文档得出的，是实测的（`crates/cats-ai-gateway/tests/healthz_single_registration.rs`
的 `first_registration_wins`：注册两个 body 不同的 handler，返回的是先注册那个）。

于是实际后果是：

- `main.rs` 的 `healthz()` 应答，形状 A（`{status, app{name,version}}`）；
- `api::healthz_handler`（形状 `{status, app, service, version}`）**运行时永远收不到请求**；
- 而 `api/mod.rs:177` 的 `healthz_returns_ok` 单独 mount `configure_routes`，
  看不到 `main.rs` 那次注册 —— **它是绿的**。

一条永远不会被调用的 handler，配一个只测它自己的测试。这就是"实现是权威、
测试是过时的"最标准的形态：测试本身没有写错，它只是测错了对象。

**已修**：删掉 `main.rs` 的 `healthz()` + `struct HealthResponse` + 那次注册，
`/healthz` 的唯一归属变成 `configure_routes()`。同时把上面那段实测语义写成
注释留在 `main.rs` 里，说明为什么不能再加一次。

**常设门禁**（`tests/healthz_single_registration.rs`，3 个用例）：

| 用例 | 钉住什么 |
|---|---|
| `first_registration_wins` | actix 的重复路径语义。actix 若改语义，这个用例会失败，届时 `main.rs` 的注释要一起改 |
| `main_rs_registers_healthz_nowhere` | `main.rs` 里不再出现 `/healthz` 注册（只看代码，跳过注释）|
| `configure_routes_serves_healthz` | 反向防"修过头"—— `configure_routes()` 必须仍然服务 `/healthz`，且 `app.name` 必须是本 crate 包名 |

变异验证 **3/3**（`D:\Temp\mutate-healthz-gate.py`）：把 `/healthz` 注册原样放回
`main.rs` → 用例失败且**指名 `src/main.rs:<行号>`**；还原 → 恢复全绿；
还原后 `main.rs` 与变异前**逐字节相同**。

写这个门禁时它先把自己判失败了：因为 `main.rs` 里我写的注释也含 `/healthz`
字面量，整行扫描把注释当成了注册。已改成只扫 `//` 之前的代码部分。

#### 二、cats-bff 的 13 个路由用例，测的是服务器从不执行的分支

`crates/cats-bff/tests/bff_routes_passthrough.rs`（`4d988f5` 加的，13 个用例 + 2 个变异验证）
用 `routes::configure` 构造被测 App。而：

```
生产注册在哪        crates/cats-bff/src/main.rs:63-76
                   handlers::login / refresh / logout / me
                   handlers::list_projects / create_project / dispatch_task
                   handlers::healthz_handler
                   路径前缀 /v1/...（无 /api/v1）

测试注册在哪        tests/bff_routes_passthrough.rs:164
                   .configure(routes::configure)
                   路径前缀 /api/v1/...

routes::configure 的调用者，全仓只有那一个测试文件
```

`routes.rs` 的文件头（20-21 行）其实已经写明"当前生效的路由注册在 `main.rs`"，
但 `configure` 自己的文档注释写的是"注册全部路由（lib.rs 调用 + 测试 bind 用）"——
**`lib.rs` 从没调用过它**。两处说法打架，我按实测把 `configure` 那句改成了事实。

判定 `routes.rs` 是被取代的草稿，依据是产品规格而不是我的口味：

```
api/openapi/cats-openapi-v1.0.1.yaml 的 paths 段共 7 条
  /auth/login  /auth/refresh  /auth/logout  /auth/me
  /healthz  /projects  /tasks

- 没有 /api/v1/ 前缀（envoy 与 openapi 用的都是 /v1/...）
- 没有任何 translate 端点
- translate_commit 本身就是 501 stub（"proto UpdateTMMatch RPC pending v1.1"）
- envoy 路由表 8 条 route 里没有 cats_bff —— 这个服务在当前配置下
  只有 127.0.0.1:8091 回环可达
```

**已修（不预设处置方式的部分）**：三处不实注释改成事实 ——
`routes.rs` 的 `configure` 文档、`lib.rs:30` 的"接完本 crate 孤儿清零"
（"接线"在这里只指参与编译，不是挂上服务）、测试里 `bff_app` 上"`routes::configure`
这张**唯一的**路由表"。另外给 `healthz_is_local_and_200_without_any_upstream`
加注记：它断言的"不碰任何上游恒 200"只对 `routes::healthz` 成立，生产应答的
`handlers::healthz_handler` 会读 `web::Data<Config>` 并吐出 `upstreams`。

**未做（需要拍板）**：这 13 个用例里有 8 个断言的是透传行为（401/503/201 原样透传、
502 on unreachable、`cats-*` 头注入），这些行为**生产那套 `handlers::*` 是否一样，
目前没有集成测试覆盖**。要让它们真正有约束力，得把 `main.rs` 的 App 装配抽成
可测函数并让测试改用它 —— 这是对生产代码的重构，且牵动"translate 那套未上线的
能力是接线还是删掉"。两者都属结构性决定，不自行拍板。

**这一条同时说明规则 9 抓不到它**：规则 9 查的是"零引用的 `pub` 函数"，
而 `routes::healthz` 被测试引用着，所以它合规。要抓这类"引用只在测试里"的
情况，需要另一条规则（**生产代码的可达性**，即从 `main` 出发的调用图），
那是独立的一条门禁，不在本轮范围。


---

### §4.1o 把 `/healthz` 统一到唯一形状，并把它变成常设门禁（lint 规则 10）🆕

§4.1m 只交付了取证与建议。**本节是执行**：按拍板统一到形状 A，并加规则 10 防止漂移。

```json
{ "status": "ok", "app": { "name": "<CARGO_PKG_NAME>", "version": "<CARGO_PKG_VERSION>" } }
```

#### 改了哪 9 个 crate

8 个本来就已经是这个形状，一行没动（asr / audit / ingestion / ocr /
office-converter / render-writer / subtitle / translation-core）。改的 9 个：

| crate | 原形状 | 备注 |
|---|---|---|
| file / notification / project / user | `{status, name, version}` | 键叫 `name` 不叫 `service` —— 读 `.service` 的监控静默拿 null |
| task | 内联 `json!`，同上形状 | 必须先 `let app = AppMeta{..}` 再放进 `json!`（宏分词器会把内嵌字面量里的 `"key":` 当 JSON 键值对）|
| report / worker | `{status, service}` | **完全没有 version** ⇒ 外部监控无法判断跑的是哪次构建 |
| auth | 一行 `json!`，`{status, service}` | 同上 |
| cats-bff | `{status, service, version, bind_addr, upstreams{5}}` | 削掉 `bind_addr` 与 5 个上游 URL（§4.1m 已论证不是当前泄露，但没有任何消费者）|

`cats-bff` 顺带修了一处连带问题：`healthz_handler` 不再需要 `web::Data<Config>`，
参数去掉后 `use crate::config::Config;` 也就没有别的用处了，一并删除
（否则 clippy `-D warnings` 报 unused）。

`report-service` 另有一个 `healthz_response()` 工厂按旧字段构造结构体，
改成同一形状，并让 `healthz()` 委托给它，避免两份定义。

`cats-ai-gateway` 的 `api::healthz_handler` 原本多吐顶层 `service` 与 `version`
（§4.1n 说过它此前被重复注册遮蔽；遮蔽去掉后它就是真正应答的那个），也削齐。

#### 顺带订正一处已被证伪的注释

`cats-bff/src/principal.rs` 的模块文档原写："`auth-service` 签发的真实 JWT
根本不包含 `roles`，于是 `Principal.roles` 在生产恒为空，所有走 `RbacChecker`
的鉴权检查都会被拒" —— 即"生产里所有 RBAC 端点恒 403"。**这条不成立**：

- `auth-service/src/models.rs:66-67` 的 `Claims` **有** `roles: Vec<String>`；
- `auth-service/src/auth.rs:84` 的 `issue_jwt` 把它写进 payload；
- `auth-service/src/handlers.rs:201` 的 login 传 `default_roles_for(&user)`，
  值是 `["User"]`（username == "admin" 时为 `["User","Sponsor"]`），不是空 vec；
- 端到端验证：新的 `bff_upstream_passthrough.rs` 用 `roles=["User"]` 的签名
  token 打 `GET /v1/projects`，穿过验签与 RBAC 并拿到上游 200
  （`list_projects_forwards_bearer_token_to_upstream`）。

仍然成立的限制是另一件事，已在新注释里写明：角色来源是 M1 阶段的硬编码简化
逻辑，不来自 user-service / workspace-membership。

#### lint 规则 10：形状漂移从此会红

`deploy/scripts/lint-compose.py` 新增规则 10，检查 `crates/*/src/` 下每一处
`/healthz` 注册所指向的 handler 函数体：必须含 `env!("CARGO_PKG_NAME")`、必须
有 `app` 键、不得含 `upstreams` / `bind_addr`。

```
OK  规则 10  18 个 crate 的 18 处 /healthz 全部返回统一形状
             （另跳过 2 个非服务 crate：cats-mock, m1-s0-smoke）
```

四个设计决定：

1. **自带扫描器有效性断言**（`_HEALTHZ_MIN_SITES = 16`，实测 18）。
   正则一旦写坏，收集数归零，本条会 FAIL 而不是恒绿 —— 与规则 8/9 同源的失效方式。
2. **双向例外表**：`HEALTHZ_SKIP` 里若列了某个 crate、而它 `src/` 下已找不到
   `/healthz` 注册，同样 FAIL。防止例外表腐烂后静默放过下一个不合规的实现。
3. **两个显式跳过项**并各写理由：`cats-mock` 是集成测试的 mock server
   （不是部署单元，healthz 故意只有 `{"status":"ok"}`），`m1-s0-smoke` 是烟雾
   测试二进制（`#[get("/healthz")]` 只用来证明 actix 起得来）。
4. **剥注释要认字符串字面量**：第一版按 `//` 截断，结果 `upstreams` 里那些
   `http://...` URL 会把后续代码整段吃掉，让规则**看不见**真正的泄漏。
   这是变异验证抓出来的（见下）。

**变异验证 12/12**（`D:\Temp\mutate-rule10.py`）：

| 场景 | 结果 |
|---|---|
| 未改动的树 exit 0 且报出 18 | PASS |
| 把 worker-service 退回扁平 `service:` 形状 | FAIL，且点名 `crates/worker-service/src/handlers.rs:17` |
| 把 `upstreams` / `bind_addr` 加回去 | FAIL，报"泄漏内部拓扑字段" |
| 把 route 正则改坏（恒返回空集） | FAIL，报"只收集到 0 处……空结果不等于没有问题" |
| 还原后与变异前**逐字节一致** | PASS（sha256 相同）|
| 附加：注释里写 `/healthz` 不算注册点 | PASS（计数仍是 18/18）|
| 附加：`HEALTHZ_SKIP` 里塞过期例外 | FAIL |

#### 本地验证

```
cargo fmt --all -- --check                                       exit 0
cargo test（10 个受影响 crate, -j 2）                             exit 0
cargo clippy --all-targets -D warnings（分两批，-j 2）             exit 0 / exit 0
lint-compose.py（10 条规则）                                       exit 0
```

`cargo test` 必须带 `-j 2`：10 个 crate 并行编译会撞 `os error 1455`
（提交限制耗尽）→ `rustc-LLVM ERROR: out of memory`，症状看起来像代码问题，
实际是资源问题。CI 上是分 job 跑的，不受此影响。

---

### §4.1p cats-bff：删掉 447 行"接了 3 步但没挂上服务"的链，并让测试挂上真跑的那张表 🆕

§4.1n 记录了"13 个用例测的是服务器不走的分支"，但没有动结构。本节按拍板处置。

#### 删掉三个模块（447 行）

```
crates/cats-bff/src/routes.rs               178 行
crates/cats-bff/src/grpc_clients.rs         125 行
crates/cats-bff/src/upstream_passthrough.rs 144 行
```

判定依据是**规格**而不是口味：

```
api/openapi/cats-openapi-v1.0.1.yaml 的 paths 共 7 条
  /auth/login /auth/refresh /auth/logout /auth/me /healthz /projects /tasks
  - 没有 /api/v1 前缀（envoy 与 openapi 用的都是 /v1/...）
  - 没有任何 translate 端点
  - routes::translate_commit 本身就是 501 stub（"proto UpdateTMMatch RPC pending v1.1"）
deploy/envoy-mvp.yaml 的 8 条 route 里没有 cats_bff（只有 127.0.0.1:8091 回环可达）
```

三者是一条链：`routes::configure` 是链顶，`grpc_clients` 被它依赖，
`upstream_passthrough` 被它依赖；而链顶唯一的调用者是那个测试文件。
**编译通过、类型检查通过、运行时不可达。**

#### 生产装配抽成可测函数，三份副本并成一份

改之前 `main.rs`、`routes.rs`、`tests/bff_smoke.rs` 各有一份路由表，三份各自演化
—— 这正是"测试挂的表和服务器跑的表不是同一张"的来源。现在：

```
handlers::configure_routes(cfg: &mut ServiceConfig)   <- 唯一一张
handlers::json_config() -> JsonConfig                  <- 400 错误信封（app_data 只能挂 App）
  ^ 被 main.rs、tests/bff_smoke.rs、tests/bff_upstream_passthrough.rs 三处共用
```

形状与 `audit-service` 里既有的同名 `configure` 一致，沿用仓库约定。

#### 13 个用例改挂生产，并逐条核对旧断言

旧的 13 个用例打的是 `routes::configure`。逐条核对后发现 **8 条透传断言没有一条
对生产成立**：

| 旧断言 | 生产实际 |
|---|---|
| 请求打 `/api/v1/auth/login` | 生产是 `/v1/auth/login`（无 `api` 段）|
| 上游 401 **原样透传** | `BffError::UpstreamError` → **502 `dependency_unavailable`**（`error.rs:188`）|
| 上游 201 **原样透传** | 201 走 `status.is_success()` 分支，根本不会变成错误 |
| 注入 `X-Cats-User-Id` / `X-Cats-Org-Id` | 生产只发 `Authorization: Bearer`（`upstream/projects.rs:87`）|
| `/api/v1/translate/*` 两端点 | 端点不存在，且从未进过 openapi |
| body **原样透传** | 走强类型 DTO 往返：`tenant_id` 被补成 `null`，projects 的三个可选字段被物化成 `null` |
| `detail` 是嵌套对象 | `ErrorBody.detail` 是 `Option<String>`，装的是上游**原文**不解析 |

所以这不是"把路径改一下"，是**断言对象整个换掉了**。新文件
`tests/bff_upstream_passthrough.rs`（13 个用例）断言的是真跑的那张表能证明的东西：

| 契约 | 靠什么证据 |
|---|---|
| URL 拼装（含 trailing-slash trim） | 假上游记下 method + path |
| 上游非 2xx 的处理 | 502 `dependency_unavailable` + `message` 含上游状态码 + `detail` 保留原文 |
| **本地校验早于网络** | 空 username → 400，**且断言假上游一次都没被调用** |
| **鉴权早于网络** | 无 token → 401，断言上游一次都没被调用 |
| RBAC 真的生效 | `roles=["Guest"]` → 403，且上游一次都没被调用；`roles=["User"]` → 200 |
| access token 转发 | 假上游记下 `Authorization`，并**钉住"不发 `X-Cats-*`"** |
| healthz 形状 | `app.name` == 本 crate 包名；无 `service`/`name`/`upstreams`/`bind_addr`；纯本地不碰上游 |

首轮跑出 3 条红，逐条查证后确认是**我写错了断言、不是生产有问题**：
`detail` 是字符串不是对象、请求体会被 DTO 补 `tenant_id: null`、响应里可选字段
被物化成 `null`。三处都改成对真实契约的断言，并把两个用例名里的 "verbatim"
去掉（它们本来就不逐字）。

#### 规则 9 为什么抓不到这一整类

规则 9 查的是"零引用的 `pub` 函数"，而 `routes::healthz` 被测试引用着，所以合规。
要抓"引用只在测试里、生产不可达"这类，需要另一条门禁：**从 `main` 出发的
生产可达性**（生产入口到该函数的调用图）。那是独立的一条，不在本轮范围。


---

### §4.1q 一次"本地全绿、只有 CI 红"的回归：e2e 断言是本地盲区 🆕

§4.1o 统一了 18 个服务的 `/healthz` 形状。本地验证是绿的：

```
cargo fmt --all -- --check                                    exit 0
cargo test（10 个受影响 crate, -j 2）                          exit 0
cargo clippy --all-targets -D warnings（分两批）               exit 0 / exit 0
lint-compose.py（10 条规则）                                    exit 0
```

**但 CI 的 `e2e (real PostgreSQL)` 红了**：

```
thread 'e2e_healthz_returns_200' panicked at
  crates/auth-service/tests/e2e_auth.rs:261:5:
assertion `left == right` failed
  left: Null
 right: String("auth-service")

test result: FAILED. 7 passed; 1 failed
```

#### 两个洞叠在一起

**洞 1：断言旧形状的用例全部是 `#[ignore]`，本地一条都不跑。**

`cargo test -p auth-service` 的本地输出是 `2 passed; 9 ignored` —— 那 9 条
`#[ignore = "e2e-needs-real-pg"]` 的用例被直接跳过。全仓扫下来，断言
`/healthz` 响应体的用例共 **9 个，其中 5 个是 `#[ignore]`**：

| 文件 | 断言的键（统一前） |
|---|---|
| `auth-service/tests/e2e_auth.rs:251` | `["service"]` |
| `file-service/tests/integration.rs:108` | `["name"]` |
| `notification-service/tests/integration.rs:108` | `["name"]` |
| `project-service/tests/integration.rs:109` | `["name"]` `["version"]` |
| `user-service/tests/e2e_t02.rs:85` | `["name"]` `["version"]` |

也就是说"本地全绿"对这 5 个断言**完全没有覆盖**，而它当时被当成了证据。
这不是粗心，是验证手段本身有盲区 —— 本地没有真 PostgreSQL，那 5 条本来
就不可能跑。

**洞 2：CI 的 e2e 步骤撞到第一个失败就中止，5 个坏 suite 只报得出 1 个。**

原步骤是 `set -euo pipefail` + 顺序调用，于是 auth-service 一红，后面的
user / project / file / notification **四个 suite 根本没执行**。
一次 shape 变更打中 5 个 suite，CI 只报了 1 个，其余 4 个每个都要再等一个
CI 周期才暴露。

用 stub `cargo` 对照验证（`D:\Temp\prove-e2e-old-aborts.py` vs
`prove-e2e-accumulate.py`）：

```
旧版（set -e，无累积）：实际执行 1 / 6 个 suite，退出码 101
新版（失败累积）      ：实际执行 6 / 6 个 suite，退出码 1，且点名全部失败 suite
```

#### 处置

**改 e2e 步骤为失败累积**（保留串行 —— 这些测试共用固定表名，并发会互相污染
种子数据；只是不再早停）。行为已用 stub `cargo` 实证，不是靠读代码。

**改那 5 个断言**，并顺手把硬编码的 crate 名换成 `env!("CARGO_PKG_NAME")`：
测试就在该 crate 的 `tests/` 下，这个宏拿到的正是它自己的包名，以后再改名
不会漏改断言。

**新增 lint 规则 11**：无论用例是不是 `#[ignore]`，只要它对 `/healthz` 响应体
做断言，就不许用统一前的键。带收集数下限自检（阈值 5，实测 9），并在每次运行
时打印"其中 N 个是 `#[ignore]`"—— 把这个盲区的大小**显式摆在输出里**。

#### 规则 11 的两个设计坑

1. **只能判顶层访问。** `body["name"]` 是旧扁平形状，`body["app"]["name"]` 是
   统一后的形状；朴素子串搜索分不开这两者 —— 第一版因此在**已经改对的 5 个
   用例**上报了 5 个假阳性。正则是 `(?<!\])\["key"\]`。
   *门禁在正确代码上失败比门禁不响更糟*，它会在下一次争论里被删掉。
2. **负向断言要豁免。** `body.get("service").is_none()` 是"这个键必须不出现"，
   那是规则 10 在服务侧钉的事，在测试侧钉住是合理的，不能报。

变异验证 5/5（`D:\Temp\mutate-rule11.py`）：

| 场景 | 结果 |
|---|---|
| 未改动树 exit 0 且报出 9 个 / 5 个 `#[ignore]` | PASS |
| 把旧形状放回一个 `#[ignore]` 用例 | FAIL，指名 `file:line` 并注明该用例 `#[ignore]` |
| 改坏扫描器正则 | FAIL，报"规则 11 失效……空结果不等于没有问题" |
| **反向**：嵌套 `body["app"]["name"]` 与 `.is_none()` 负向断言不得误报 | PASS |
| 还原后与变异前逐字节一致 | PASS |

#### 扫描器自身的两个 bug（本轮第三次栽在同一类地方）

写扫描器时又出现"0 结果当结论"，第三次：

1. 第一版要求 `#[test]` 与 `fn` 同行，而 actix 写在**上一行** ⇒ 扫出 0 个，
   差一步就写成"没有这样的测试"。加了"扫到的测试函数总数 < 200 即判定扫描器
   可疑"的自检（实测 455）才暴露。
2. 规则 11 的初版把"提到 healthz 的用例体"（26）当成被检查集合，与独立实现
   算出的"真的断言了键的用例"（9）不一致。阈值会虚高、误报面会放大，收紧成
   后者，并**用两个独立实现对拍**（9 / 5，两边完全一致）才收工。
---

### §4.1r 门禁自己也会假绿：规则 10 检查了 17/18 个**别人的**代码 🆕

§4.1o 加了 lint 规则 10（18 个服务的 `/healthz` 必须统一形状），并在本地跑通。
**它当时是绿的，而且绿的没有道理。**

#### 怎么发现的

本来是要做规则 12（openapi 的 `HealthResponse` 必须与 Rust 实测形状对拍），
动手前先要证明"Rust 侧确实是这个形状"。当时手上只有
`D:\Temp\check-openapi.py`，它打印了一行：

```
  matches the canonical handler shape: True
```

看着像证据。**它不是** —— 那行断言的第 40~43 行是：

```python
# The shape the 18 handlers actually emit:
expected_top = ['status', 'app']        # 硬编码常量
expected_app = ['name', 'version']
ok = list(hs['properties'].keys()) == expected_top and ...
```

整个文件里**没有任何扫描 Rust 的代码**。它证明的只是"spec 自己和自己一致"。
拿它当"Rust 与 spec 对得上"的证据，等于用同一个数去比它自己。

于是写了一个真正去源码里抽形状的**独立实现**（`dump-healthz-shape.py`）。
那个独立实现自己也错了三次，但正是这三次把规则 10 的问题顶了出来。

#### 规则 10 的真 bug：检查循环读的是收集循环的残留变量

```python
for _crate in sorted(os.listdir(crates_root)):     # 收集循环
    ...
    _drel = _body = _bline = None
    if _name:
        ...
        if _cands:
            _drel, _body, _bline = _cands[0]      # _body 在这里被赋值

for _site in _healthz_sites:                       # 检查循环（另一个循环）
    _crate, _reg_rel, _reg_line, _drel, _bline, _name = _site
    #                                          ↑ 元组里没有 _body
    if not _HEALTHZ_PKG_RE.search(_body):         # 读的是上面残留的值
```

收集循环按字母序走完 18 个 crate，循环结束后 `_body` 里剩下的是**最后一个
crate（worker-service）的 handler 函数体**。于是检查循环拿它去检查全部 18 个
site：

- **17 个 site 检查的是 worker-service 的代码**，不是它们自己的；
- **report-service 从来没被检查过** —— 它的 handler 是纯委托
  （`healthz()` 的整个函数体只有一句 `healthz_response()`，自身不含任何响应
  字段）。它本该第一个报红，实际却因为"借用了 worker-service 的 body"而通过。

也就是说这条规则的真实覆盖是 **1/18**，另外 17 个是恒真的。

#### 变异证明（`D:\Temp\mutate-rule10-ownbody.py`，4/4）

修复前后的差别用一个判据就能分开：**改坏一个 crate，门禁应当只点名那一个。**

| 变异 | 修复前 | 修复后 | 判定 |
|---|---|---|---|
| 改坏 asr-service（**第一个** crate）丢 `app` 键 | exit 0（**假绿**） | exit 1，只点名 `asr-service` | PASS |
| 改坏 worker-service（**最后一个**）丢 `app` 键 | exit 1，**18 个 site 全红** | exit 1，只点名 `worker-service` | PASS |
| 改坏 report-service **被委托的工厂** `healthz_response()` | — | exit 1，点名 `report-service` | PASS |
| cats-bff 重新泄漏 `bind_addr` | — | exit 1，点名 `cats-bff` | PASS |

第一行说明旧门禁对非末尾 crate 完全失灵；第二行说明旧门禁的"红"也是假的
（一个 crate 的问题被复制成 18 条噪音，掩盖真正的问题）。

#### 修法

1. `_healthz_sites` 的元组里带上 `_body` —— 检查循环不再读残留变量；
2. 新增 `_healthz_defs[crate]`，**逐 crate 留存**函数索引（不逐 crate 留存就只有
   最后一个 crate 的）；
3. 新增 `_healthz_payload()`：跟随纯委托（最多 3 跳，带环检测），因为
   report-service 的响应体在被委托的工厂里；
4. **抽不出响应体一律 FAIL** —— 把"解析失败"当成"形状正确"正是这个 bug 的
   表现形式。

#### 规则 12：两侧各自自洽 ≠ 两侧一致

规则 10 只看 Rust 侧。规格与实现之间**没有任何东西**在互相校验：实现统一到
`{"status","app"}` 之后，openapi 里仍然写着统一前的 `service: string`；反过来
把 spec 改坏也没有任何检查会响。规格是给代码生成器和外部消费者用的。

规则 12 从 Rust 源码**抽**出每个 handler 的顶层键集合，再与
`components.schemas.HealthResponse` 的 `properties` / `required` 逐项比。
两个设计要点写进了代码注释：

1. **Rust 侧的形状不能写成常量** —— 写死 `["status","app"]` 的检查永远绿，
   改 Rust 也不响，等于什么都没查；
2. 抽不出形状必须报错，不能当成"一致"（独立的 `_HEALTHZ_SHAPE_MIN` 阈值）。

变异验证 5/5（`D:\Temp\mutate-rule12.py`，双向）：

| 场景 | 判定 |
|---|---|
| 未改动树 exit 0，报"18 处实测形状与 openapi 逐项一致（顶层键 `['app','status']`）" | PASS |
| **Rust 加顶层键** `commit_sha`，spec 不动 → 门禁红 | PASS |
| **spec 加顶层键** `commit_sha`，Rust 不动 → 门禁红 | PASS |
| spec 的 `required` 少列 `app` → 门禁红 | PASS |
| spec 的 `required` 清空 → 门禁红 | PASS |
| 删掉整个 `HealthResponse` schema → 门禁红 | PASS |
| 还原后逐字节一致 | PASS |

> 第一个变异走的是"形状不唯一"分支而非"与 spec 不符"分支：全仓形状一旦分叉，
> 就没有"单一形状"可与 spec 比了。两个分支都说明门禁响了，要证明的是**它听见
> 了这个新键**。

#### 独立实现对拍的价值，以及它的代价

那个一次性扫描器自己也栽了三次，全部是同一类错误：

| # | 错误 | 症状 | 为什么没被当成"形状不统一" |
|---|---|---|---|
| 1 | `_HEALTHZ_FN_DEF_RE` 漏了 `re.M` | `^` 只匹配偏移 0 ⇒ 解析到 16/18 | 剩下的两个**恰好形状相同** |
| 2 | raw string 的收尾符算成 `""`，找不到就吞到文件尾 | 后续所有 `}` 消失 ⇒ 同样解析不到 | 同上 |
| 3 | `defs` 定义在 crate 循环内却在循环外读 | **全部 18 个 site 都解析到 worker-service 的函数体** | 全部形状相同 ⇒ 报出"1 种形状 / 18 处" |

第 3 条与规则 10 的 bug **同型**。当时若只有这一个实现，它会安静地报出
"全仓形状一致"；正因为规则 10 是另一个独立实现，两边不一致才暴露出来。

代价是它一度给出**比规则 10 更不可信的结论**（16/18 "成功"其实全是错的）。
所以每次打印形状都同时打印 `body` 的来源文件，并且断言来源必须在同一个 crate
内（`CROSS-CRATE LEAK` 自检）—— 让"解析成功"这件事本身可被检查。

#### 落到纪律上的三条

1. **门禁报绿时，先证明它在看你要它看的东西**。把元组少写一个字段这种错误，
   症状是 exit 0。
2. **"抽不出"必须 FAIL**。任何"解析失败就跳过"的分支都会变成静默盲区，
   而盲区的形状和"没有问题"一模一样。
3. **判定必须能被反向证伪**。本节所有结论都配了"改坏哪一个、应当只点名哪一个"
   的判据，而不是靠读代码确认。
---

### §4.1s k3s 上 8 个服务的 Pod 会永远 NotReady —— 而 CI 从没看过 deploy/k3s 🆕

§4.1o~§4.1q 做的都是「代码对不对」。这一节是**部署清单对不对**，问题更重：
它不是一个会报错的配置，而是一个**看起来部署成功了**的配置。

#### 症状

`deploy/k3s/cats-core/` 下 10 个 Deployment 有 **8 个** 的 `readinessProbe`
打的是 `/readyz`：

| Deployment | liveness | readiness（本轮之前） | 源码里真的注册了 /readyz 吗 |
|---|---|---|---|
| audit-service | `/healthz` | `/readyz` | ✅ `src/handlers.rs:36` |
| worker-service | `/healthz` | `/readyz` | ✅ `src/main.rs:64` |
| auth-service | `/healthz` | **`/readyz`** | ❌ |
| user-service | `/healthz` | **`/readyz`** | ❌ |
| project-service | `/healthz` | **`/readyz`** | ❌ |
| file-service | `/healthz` | **`/readyz`** | ❌ |
| notification-service | `/healthz` | **`/readyz`** | ❌ |
| report-service | `/healthz` | **`/readyz`** | ❌ |
| task-service | `/healthz` | **`/readyz`** | ❌ |
| cats-ai-gateway | `/healthz` | **`/readyz`** | ❌ |

全仓搜 `"/readyz"` 只有 3 处命中（`worker-service/src/main.rs:64`、
`audit-service/src/handlers.rs:36`、以及 `audit-service/src/main.rs:13` 的注释）。

那 8 个服务的 Pod 会**永远 NotReady**，Service 拿不到 endpoint —— 在 k3s 上
等于完全不可访问。失败模式最恶劣的地方在于：Pod 起来了、Deployment 有了、
rollout 也在跑，只是**永远不 Ready**，而这恰恰是"部署成功"的长相。

#### 为什么一直没被发现

`deploy/k3s/` **完全不在 CI 里**。`ci-helm-lint.yaml` 只 lint `deploy/helm/`，
`deploy/k3s/` 的每一个字节都没被任何检查看过。对照：`deploy/helm/` 的
19 个 chart × 2 = 36 条探针**全部**是 `/healthz`，全部正确。

同一个概念，两套部署清单，一套被 CI 守着、一套完全没人看 —— 于是坏的那套
慢慢腐烂。这也是本轮唯一一处**不是**由"代码里有个变量读错了"造成的缺陷：
它纯粹是**覆盖缺口**。

#### 处置（2026-10-07 拍板）

把这 8 个 Deployment 的 `readinessProbe` 改回 `/healthz`；**audit-service 与
worker-service 保留 `/readyz`** —— 它们确实实现了，降级一个本来正确的探针与
制造这个缺陷是同一类错误。给这 8 个服务补真正的 `/readyz`（含依赖探活）
作为独立工单，不在本轮。

顺带记录本轮核对出的另外两处部署清单不一致，**均未改动**：

- ~~`translation-core` 与 `envoy` 在 `deploy/k3s/` 下**没有任何探针**，而
  `deploy/helm/translation-core/values.yaml` 有 liveness+readiness；~~
  > **更正（2026-10-07，订正上一条的第一句）**：**这句是错的，而且是我自己的
  > 检查器给的。** 当时的脚本读的是 `probe.httpGet.path`，两者都返回 `None`，
  > 我读成了"没有探针"。实际重新解析（打印探针的**类型**而不是 path）后：
  > `translation-core` 的 liveness/readiness 都是 **`tcpSocket: 50051`**（gRPC 端口，
  > 与它唯一声明的 containerPort 一致），`envoy` 是 **`tcpSocket: 8080`**。两者都
  > 有探针，而且都是**正确**的写法。
  > 教训写进了规则 13 的代码注释：`get("a", {}).get("b")` 返回 `None` 同时意味着
  > 三件事（`a` 不存在 / `a` 里没有 `b` / `b` 真的是 None），而后两者后果完全不同。
  > 下面第二条仍然成立。
- `deploy/docker-compose-mvp.yml` 里**12 个应用服务一个 `healthcheck` 都没有**，
  只有 postgres / kafka 有。因此 openapi `HealthResponse` 的 description 里那句
  「compose `healthcheck` 也只判状态码不看 body」**并不成立** —— 应用服务
  根本没有 healthcheck（该 description 写在 `d695578`，本节是其订正）。

#### 规则 13：探针路径必须在那个服务的源码里真的注册过

堵的是整类问题：部署清单与源码之间没有任何交叉校验。规则 13 收集
`deploy/helm/*/values.yaml` 与 `deploy/k3s/**.yaml` 里的每一条
liveness/readiness/startup 探针路径，回源码查该服务注册过的路径集合（含
`web::scope` 前缀拼接），对不上就 FAIL，并打印该服务**实际注册了什么**。

范围只限自有服务，判据是 `crates/<name>/src/main.rs` 存在 —— 这一条自动排除了
`cats-common`（库，没有 bin 目标）、`cats-mock` / `m1-s0-smoke`（测试用），
也不必为 postgres / kafka 这类外部镜像维护名单。envoy 的 `/healthz` 是
`direct_response` 的纯文本、根本不代理，同样不在范围内。

**最有说服力的一次验证不是造变异，而是把修复退回去**：用
`git stash push -- deploy/k3s` 还原到修复前，规则 13 精确报出那 8 条，
每条附上该服务实际注册的路径，且**没有**误报 audit/worker 的合法 `/readyz`。

变异 + 反向场景 6/6（`D:\Temp\mutate-rule13.py`）：

| 场景 | 结果 |
|---|---|
| k3s 探针改成未注册的 `/healthcheck` | FAIL，指名 project-service |
| helm 探针改成未注册的 `/alive` | FAIL，指名 user-service |
| task-service 不再注册 `/healthz` | FAIL（连带 4 条，因为它 4 处清单都指这里） |
| **反向**：audit / worker 的 `/readyz` 不得误报 | PASS |
| **反向**：探针指向 `web::scope` 内的 `/v1/auth/login` 须认得已注册 | PASS |
| 扫描器被致盲（`route` 正则改不可能匹配） | FAIL 54 条，不是静默放行 |

倒数第三行是这条规则能不能被长期使用的关键：scope 前缀不拼的话，规则会把
所有 scope 内的路径误报，而**在正确配置上失败的规则会被直接删掉**。

---

### §4.1t 并行审计查出的"潜伏假绿"清单（本轮记录，未修）

派了三个只读子代理横向排查 §4.1n~§4.1r 这一类缺陷。结果如下 ——
**没有任何一条是"重复注册路由"**（cats-ai-gateway 是全仓唯一一例，已修），
但查出三类值得单独排期的债。

#### 1. 死用例：m1-s0-smoke 的 `config()`

`crates/m1-s0-smoke/src/actix_smoke.rs:27` 的 `pub fn config` 是 `#[get("/healthz")]`
的唯一装配点，而调用者只有同文件 `#[cfg(test)] mod tests`（`:38`）。该 crate
**没有 `main.rs`**，`Cargo.toml` 里也没有任何 crate 依赖它 —— 全仓零非测试调用者。
这是本仓最纯粹的一例：被断言的 `/healthz` 端点只存在于测试里。
（该 crate 已在 `HEALTHZ_SKIP` 里，不计入规则 10 的 18 个。）

#### 2. 潜伏假绿：6 个 crate 的测试各自抄了一份路由表

`auth-service` / `user-service` / `project-service` / `file-service` /
`notification-service` / `task-service` 的**生产**在 `main.rs` 注册路由，
**测试**在测试文件里内联重抄一份。逐条核对后**当前全部一致**，所以现在不是假绿。

但形态与已修的 cats-bff 同源，且已经咬过一次：task-service 的
`handlers.rs:591` 注释写着「**唯一生成表** —— `main.rs` 与集成测试都从这里接进来」，
而后半句**不成立** —— `tests/integration.rs` 一次也没调 `configure_app`，它的
`make_app_no_db`（`:378`）在 `:391-399` 自己 `.route(...)` 抄了 3 条。
更讽刺的是 `handlers.rs:594~598` 警告的正是这个陷阱
（「在一张**已经与生产漂移**的路由表上测试」），而集成测试至今还在自己那张表上。
该注释已按"带日期的历史记录不改写"加更正块（见 `handlers.rs:602` 之后）。

排期建议：照 `audit-service`（生产 `main.rs:87` 与 15 处测试**共用同一个**
`handlers::configure`，本仓唯一做到这点的 crate）与 `cats-ai-gateway` 的样子收敛。

> **进展（2026-10-07，本节后半段全部完成）**：6 个 crate 已全部收敛。
>
> | crate | 路由（生产 = 测试） | app_data | 受影响用例 | 收进 |
> |---|---|---|---|---|
> | task-service | 7（测试原本只抄 3 条） | 1 | 3 | `handlers::configure_app`（已有，接入） |
> | auth-service | 5 = 5（`e2e_auth` 漏 `logout`，`e2e_t01` 两处漏 `healthz`） | 1 | 17 | `handlers::configure_routes`（新建） |
> | user-service | 4 = 4 | 1 | 5 | `handlers::configure_routes`（新建） |
> | project-service | 6 = 6 | 2 | 7 | `handlers::configure_routes`（新建） |
> | file-service | 6 = 6 | 2 | 8 | `handlers::configure_routes`（新建） |
> | notification-service | 5 = 5 | **3** | 6 | `handlers::configure_routes`（新建） |
>
> **本仓 7 个有集成测试的服务现已全部单一事实来源**；只剩 `cats-mock` 仍按
> flag 条件装配（它的"生产"是 mock server，语义不同，不在本条范围）。
>
> 顺带消除了一类隐患：这几个 crate 的 app_data 都不止一个。测试漏注册 RBAC /
> 事件总线那个时，actix extractor 取不到 → 500 `Requested application data is
> not configured correctly`；而 `/healthz` **不吃**这些 extractor —— 症状是
> **"healthz 过、其余全挂"**，极易被误判成 RBAC 逻辑或事件总线坏了。把 app_data
> 与路由一起收进 `configure_routes` 后，这类"忘了注册"只剩一种写法。
>
> 三个坑记在各 commit message 里：`HttpServer::new` 的闭包是 `Fn` 不是
> `FnOnce`（每个 worker 线程各调一次，**必须先 `clone()` 再 move**），
> 四个 crate 里有三个在这里撞过；测试文件对 handler 的引用风格不统一
> （有的 `use handlers`，有的全限定），按错风格改会编译不过；以及
> `e2e_t01.rs` 的 import 收窄（原来 import 的四个 handler 名在文件里只出现在
> 字符串与注释中，零函数引用）。
>
> **验证状态**：43 个受影响的 e2e 用例全部带 `#[ignore = "e2e-needs-real-pg"]`，
> **本地一个都跑不到**（无真 PostgreSQL）。它们的唯一执行点是 CI 的
> `e2e (real PostgreSQL)` job —— auth 那一轮与这四个 crate 那一轮均为
> **success**。这是本轮第四次撞上"本地全绿必须连 ignored 计数一起读"。

#### 3. 反向缺口：生产注册了但零测试覆盖

`worker-service`（3 条）、`report-service`（4 条）、`asr` / `ocr` / `ingestion` /
`subtitle` / `office-converter` / `render-writer`（各 1 条 `/healthz`）——
这些路由只有 `tests/smoke.rs` 里一行 `name_matches_crate!` 宏，没有任何请求级用例。
`task-service` 7 条里有 4 条无覆盖。`audit-service` 的 `/readyz` 也无覆盖。

子代理另外提出一条**本轮不采信**的怀疑：它说各 e2e 用例大量带
`#[ignore = "e2e-needs-real-pg"]`，因此"绿"可能意味着没跑。该条与 §4.1q
的结论重复，且当时未核对 CI 配置；§4.1q 已用规则 11 把 healthz 那一类变成
静态检查，e2e 步骤也已改为失败累积（不再早停），全量 `#[ignore]` 清单待单独一轮。
---

### §4.1u 规格 ↔ 实现漂移清单（本轮核实，未修）

并行审计的第三份结果。**这一节与前面几节不同：前面是"门禁在骗自己"，
这里是"规格和实现本来就在各说各话，而没有任何检查会响"。**

下面每一条都标注了核实状态。子代理的结论是**证据**，不是定论 —— 凡是写
"已核实"的都是本轮用独立脚本从源文件重新推出来的。

#### 已独立核实（可作为工单依据）

**1. `POST /tasks` 的成功状态码，BFF 与后端服务彼此分歧，而 spec 只对上 BFF。**

```
openapi /tasks 声明          202
crates/task-service          HttpResponse::Created()   -> 201
crates/cats-bff             HttpResponse::Accepted()  -> 202
```

同一条路径，穿过和不穿过 BFF 返回不同状态码。spec 恰好与 BFF 一致，
于是"直接调后端"的调用方拿到 201 会与 spec 冲突。这条不是任何一方的"笔误"
能解释的 —— 它反映 BFF 与后端对"已受理"和"已创建"的语义分歧从未被对齐过。

**2. 服务实际发出、但 openapi `ErrorCode` 枚举里没有的错误码：10 个。**

openapi 枚举实测 **27** 个值（文档多处写"28 条"，见下）。用字符串字面量在
各服务 `src/` 下（排除 `tests/`）重新扫了一遍真实发出的错误码，10 个在枚举外：

| 错误码 | 出现在 | 错误码表提到过吗 |
|---|---|---|
| `project_not_found` | `project-service/src/handlers.rs` | ❌ |
| `file_not_found` | `file-service/src/handlers.rs` | ❌ |
| `file_too_large` | `file-service/src/handlers.rs` | ❌ |
| `notification_not_found` | `notification-service/src/handlers.rs` | ❌ |
| `task_not_found` | `task-service/src/handlers.rs` | ❌ |
| `qa_blocked` | `common/src/error.rs` | ❌ |
| `compliance_blocked` | `common/src/error.rs` | ❌ |
| `jti_revoked` | `auth-service/src/handlers.rs` | ✅ |
| `refresh_revoked` | `auth-service/src/handlers.rs` | ✅ |
| `service_unavailable` | `cats-mock/src/http/response.rs` | ❌ |

`CATs_错误码表` 明文写着「**不允许新增未在本表定义的错误枚举**」。上表 8 个
连文档都没进。前 5 个是明确对外的 404/413；`qa_blocked` / `compliance_blocked`
在 `cats-common` 里，是否真的对外**未判定**；`service_unavailable` 来自
`cats-mock`（mock server，不算生产错误码）。

> 第一遍扫描只认 `CatsError::X` / `ErrorCode::x` 两种构造，扫出 **0** 个，
> 看起来像是子代理报错。换成字符串字面量（`ErrorBody { error: "..." }` 这种
> 同样是真的在发码）才扫出 10 个。**"扫出 0"在这里不是"不存在"，是扫描器
> 太窄** —— 本轮第三次栽在同一个地方。

**3. openapi 枚举实测 27 条，而文档写的是 28 条**（openapi `:286`、
错误码表 `:179/:189`、接口设计书 `:542`）。反向核对：27 个枚举值在两份错误码表里
都有出现，所以不是"漏登记"，是**计数写错**。

**4. auth 三个端点的 schema 与实现对不上 —— 已修**（用内联 schema 重读后确认；
子代理第一次读的是 `$ref`，而这些 schema 是内联的，所以它给的字段数要重取）。

逐字段对拍（spec ↔ `crates/auth-service/src/models.rs`）：

| 端点 | 原文 | 实现 | 处置 |
|---|---|---|---|
| `POST /auth/login` 请求 | `required: [username, password, **tenant_id**]` | 只有 `username, password` | `tenant_id` 降为可选 |
| `POST /auth/login` 200 | 3 字段 | **6** 字段（多 `token_type`/`user_id`/`username`） | 补齐 3 个 |
| `POST /auth/refresh` 200 | 3 字段 | **4** 字段（多 `token_type`） | 补齐 1 个 |
| `POST /auth/logout` 200 | **完全没有 body schema** | `{revoked, revoked_at}` | 补上 |
| `GET /auth/me` 200 | 3 字段 | 3 字段 | ✅ 本来就一致 |

`tenant_id` 保留为可选属性而不是删掉：`cats-bff` 侧是
`Option<String>` + `#[serde(default)]`（`upstream/auth.rs:22-27`），**线上格式
允许它存在**，错的只是把它标成必填。

`POST /auth/logout` 那条最实际：客户端根本无从判断登出有没有生效。

#### 子代理单方证据（双侧 file:line 齐备，但本轮未逐条复核）

- `common/src/error.rs` 的 30 个 `CatsError` 变体与 openapi 的 27 个只是部分重叠；
  `cats-bff/src/error.rs` 是唯一与 openapi 完全一致的一套。该文件 `:35` 的注释
  声称"与 openapi `components.schemas.ErrorCode` 一致（per §8.5 已落地）"——
  按上表这个声称**不成立**。
- `ErrorBody.detail`：openapi 声明 `string`，`common/src/error.rs` 是
  `Option<serde_json::Value>`（audit-service 走它，能序列化出对象/数组型 detail）。
- `GET /auth/me` 的 `user_not_found`：错误码表 `:289` 写 401、`:218` 写 404
  （同一份表里自相矛盾），实现实际返回的是 `error: "invalid_token"` + 401。
- `X-Cats-Error-Code` / `X-Cats-Request-Id` / `WWW-Authenticate` 三个错误响应头
  错误码表有声明，实现侧只搜了字面量没找到，可能由中间件注入 —— 未穷尽。

#### 为什么本轮不就此加门禁

本可以照规则 12 的样子加一条"服务发出的错误码必须在 openapi 枚举内"，但
**"这个码到底对外返回没有"无法静态判定**：`qa_blocked` 定义在 `cats-common`
里，是否被任何 handler 用到、`service_unavailable` 来自 mock server、
`jti_revoked` 又在错误码表里有记录。一条误报率高的门禁会被直接删掉，而被删掉的
门禁比没有门禁更糟（它会让人以为这里被看着）。

所以本轮只记录。**下一步的做法**已经想清楚：照 `HEALTHZ_SKIP` 的模式加一张
**双向例外表** —— 已知的 10 个逐条登记并写明理由（对外 / 未对外 / mock），
此后任何**不在表里的**新错误码一律 FAIL。例外表因此会自己腐烂（用过的可以删、
过期的必须删），而这正是 `HEALTHZ_SKIP` 那条"例外不能腐烂"检查的用意。

#### 同类：openapi 的 `/healthz` 前缀问题（未判定）

openapi 的 `servers[0].url` 是 `https://api.cats.internal/v1`，而 `/healthz`
与 `/auth/login` 同处这个 server 下 —— 拼出来是 `/v1/healthz`，但 18 个服务
全部注册在根 `/healthz`，envoy 更是对 `/healthz` 直接 `direct_response`、
根本不代理。`/auth/login` 因为代码里确实是 `/v1/auth/login` 所以对得上，
`/healthz` 对不上。缺一份"envoy 是否重写前缀"的权威说明，故只记录不判定。

### §4.1v RBAC 的「第二参数」语义被误读，导致 `worker/tick` 的鉴权从未生效（2026-10-07）

#### 缺陷：`require_roles` 的第二个参数是**调用者角色**，不是「允许的角色」

`cats_rbac::service_helpers::require_roles(checker, roles, resource, action)` 内部是

```rust
for role in allowed_roles {                 // 循环的是**传进来的那个列表**
    match checker.check_roles(&[*role], resource, action).await { Ok(()) => return Ok(()), ... }
}
```

而 `check_roles(user_roles, ...)` 问的是「**给定这些角色**，有没有 `resource:action`
权限」。于是这段循环在问「**这个角色**有没有权限」，而不是「调用者是不是这个角色」。

worker-service 的 `POST /v1/worker/tick` 把

```rust
&[Role::Sponsor, Role::RustLead, Role::SRELead]   // 作者想表达"允许的角色"
```

传了进去，同时把从 header 解出来的角色丢进了 `let (_user_id, _roles)`。
第一个迭代就查「Sponsor 有没有 Task:Update」——有（Sponsor 全权 `resource_all ×
action_all`）——直接 `Ok`。**调用者是谁完全不影响结果。**

#### 实测（修复前，`X-Cats-User-Id` 合法，只改 `X-Cats-Roles`）

| `X-Cats-Roles` | 应有结果 | 实得 |
|---|---|---|
| 不带头 | 401 `missing_authorization` | 401 ✅ |
| 只有 uid | 401 `unauthorized` | 401 ✅ |
| uid 非法 | 401 `invalid_token` | 401 ✅ |
| `NoSuchRole` | 401 `unauthorized` | 401 ✅ |
| `Sponsor` | 500（放行，库死） | 500 ✅ |
| **`User`** | **403** | **500 ← 进了 `tick()`** |
| **`Guest`** | **401/403** | **500 ← 进了 `tick()`** |

#### 波及范围：全仓只有这一处

其余 6 个服务（file / project / report / task / notification / cats-bff）都是
`check_roles(&auth.roles, ...)`，audit-service 是
`require_roles(&state.checker, &roles, ...)` —— 都传**请求头解出来的调用者角色**，
用法正确。

#### 可达性：潜伏，未兑现

该端点**未被 envoy 路由**；compose 里 worker 只绑 `127.0.0.1:8089`；k3s 是
ClusterIP 且无网关路由。所以这不是「正在被利用」，而是「一旦有人加了网关路由或把
绑定改成 `0.0.0.0` 就直接可用」。

#### 修法：显式白名单，且**不用**权限矩阵

改为比对从 header 解出来的角色是否持有 `TICK_ROLES` 之一。

**为什么不用矩阵判定**：`default_permissions()` 里 `Task:Update` 目前**只有
Sponsor 持有**（RustLead 只对 `Service` 域全权，SRELead 无 Update）。若改成
`require_roles(&checker, &caller_roles, Task, Update)`，RustLead 与 SRELead 会被
矩阵挡掉，与本 handler「这三个角色可 tick」的原意相反。白名单才是这里真正想
表达的东西 —— 测试 `tick_as_rustlead_passes_allowlist_even_without_task_update_perm`
专门把这两条判定路径区分开。

#### 同批修掉：`/readyz` 在 DB 挂时仍返 200

worker-service 与 audit-service 的 `readyz` 是逐字相同的两份实现，DB 连不上时
**仍返回 200**，只在 body 写 `db:"fail"`。k8s 的 readinessProbe **只看状态码**，
而 `deploy/k3s/cats-core/worker-service.yaml:33` 正是拿 `/readyz` 当 readinessProbe
—— 库挂掉的 Pod 会被判 Ready、继续接流量。改为 DB 失败时返回 **503**（两者同步），
响应体不变。

#### 门禁：lint 规则 15

扫 `crates/*/src`，禁止把角色**字面量数组**传给 `require_roles`。排除
`cats-rbac` 自身 —— 它自己的单元测试用字面量模拟调用者角色是合法用法。
自失效保护：扫到的 `.rs` < 10 或全仓 `require_roles` < 5 处时报错
（「空结果不等于没有问题」）。当前实测 127 个 `.rs` / 18 处。

变异验证 14/14，含双向：

| 变异 | 期望 | 实得 |
|---|---|---|
| 退回绕过形态 | User / Guest 用例 FAILED，Sponsor 用例仍 ok | PASS |
| 改成纯矩阵判定 | RustLead 用例 FAILED，lint 仍 exit 0 | PASS |
| 注入字面量数组调用 | lint exit 1 且点名 worker-service | PASS |
| 传变量（合法写法） | lint 仍 exit 0（不误报） | PASS |

#### 本轮我自己踩的两个坑（都是「验证器骗人」）

1. **变异脚本用 `shutil.copy2` 还原会连 mtime 一起复制**。还原后源文件时间戳比
   上次编译还早，cargo 的 mtime 指纹判定「没变过」→ **跳过重编** → 跑出来的是
   **变异体编译出的旧二进制**。当时 sha256 对得上、git status 干净、clippy exit 0，
   只有测试仍在报变异态的失败。**还原必须刷新 mtime。**
2. 用 `io.open(p,'w').write(io.open(p).read())` 刷新时间戳时，**`open(p,'w')` 先求值
   并把文件截断**，随后才读 —— 两个 crate 下 16 个 `.rs` 全被清成 0 字节。刷新
   时间戳要用 `os.utime`，或先读进变量再写。