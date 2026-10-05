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

#### 剩余（7 crate / 3963 行，2026-10-05 audit-service 接线后）

| crate | 行数 | 状态 |
|---|---|---|
| cats-mock | 2899 | `lib.rs` 文档宣称提供 http/db/infra/data 四模块，实际一个都没声明 |
| cats-bff | 809 | 依赖 4 个不存在的 `Config` API（v0.3 §6 已记），需先补 API |
| 其余 5 个 | 255 | 各自的 `state.rs` 等，**但情况各不相同，见下** |

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

**仍未修**：6 种响应格式本身不统一。统一它要动 12 个 crate 的 handler
并改共享结构，属于独立的 API 收敛工作，不在本次范围。



### §4.4 v0.3 §6 其余各项未变

Tauri 客户端、真实 AI provider、Vault secret 注入、translation-core 接真
project_db + pgvector、真 mTLS（per ADR-009）均未变。

---

## §5 检查本身的验证

### §5.1 端到端：重建镜像 + `docker compose up`

```
$ docker build -f deploy/Dockerfile.runtime -t cats-runtime:latest .   # BUILD_EXIT=0

$ docker logs cats-verify-translation-core
INFO translation_core: starting translation-core bind_addr=0.0.0.0:50051
INFO actix_web::server: starting service: "actix-web-service-0.0.0.0:50051"
```

修复前同一容器打印的是 `bind_addr=0.0.0.0:8090`。

按编排推导出的 13 个目标逐个探活，**13 个全 HTTP 200，0 失败**
（`translation-core` 经宿主 50151 → 容器 50051 应答；本机 50051 被
Windows 服务 `endpointService` 占用，故用 override 换端口，
容器内端口与正式编排一致）。

### §5.2 CI

| run | commit | 结果 |
|---|---|---|
| `37157912198` | `5a9a74a` | **4/4 绿**（含 lint 步骤） |
| `37166472300` | `481618c` | **4/4 绿**（接线 4 个 crate 后） |
| `37278878858` | `4a2b319` | 3/4 绿 —— ubuntu 红在 `Upload coverage`（codecov TLS 握手失败），**测试与覆盖率门禁本身通过** |
| `37280627724` | `3d8f96b` | **4/4 绿**（加 `continue-on-error` 后，同一位置不再拖红） |
| `37281458555` | `6eb069b` | **4/4 绿**（healthz 修复后） |

#### run `37278878858` 暴露的 CI 脆弱点（已修）

`Upload coverage` 步骤红，错误是

```
Error: write EPROTO ...: ssl3_read_bytes: ssl/tls alert handshake failure
```

同一个 job 里 `Run tests` 与 `Coverage gate (fail-under 40%)` 都通过，
macOS / Windows / e2e 也全绿 —— 整条流水线只因为「把报告传到第三方」
而变红。

**`fail_ci_if_error: false` 挡不住**：那是 codecov 解析/上传失败时的开关，
而 TLS 握手失败是 action 内部的 JS 硬错误，绕过了该开关。
真正能兜住的是 `continue-on-error: true`。

改动的安全性依据：覆盖率真正的门禁是**上一行**的
`cargo llvm-cov --fail-under-lines 40`，它在上传之前就已给出结论。
上传只是把报告送到 codecov 做可视化。让一个纯上报动作拥有否决整条
流水线的权力，等于把第三方服务的可用性混进了代码质量的判定里。


### §5.3 各闸门的反向验证

| 检查 | 反例 | 正例 |
|---|---|---|
| lint 规则 6 | 喂修复前的 compose → **恰好 7 项违规**，全是死 env 变量 | 当前 compose → **0 项** |
| lint 规则 7 | 塞一个 `probe_orphan.rs` → **FAIL**（提醒行数 5731→5734 同步变） | 移除后 → 0 项 |
| 探针清单推导 | —— | 从编排推导出 **13 个**目标，含 translation-core |
| `resolve_bind_addr` | —— | 3 个单测（优先级 / 兼容 / 默认值）全过 |
| 死文件断言 | `error.rs` 注入语法错误 → `cargo check` **仍退出 0** | —— |

> 一次翻车值得记：第一次做规则 6 的反向验证时，用 PowerShell
> `Out-File` 写出对照文件，编码被破坏，脚本对着一个坏文件报了 19 项，
> 差点当成真结论。**对照组本身要先验字节**——否则你比较的不是版本差异，
> 是编码差异。
>
> 同类还有一次：查容器清单时用 `Select-String "catsverify"` 过滤，
> 而容器实际叫 `cats-verify-*`，于是误判成"只剩 9 个容器在跑"。
> **断言"不存在"之前，先确认搜索模式能命中真实命名。**


---

## §6 拍板记录（2026-10-04）

| 事项 | 决议 | 依据与后果 |
|---|---|---|
| 5731 行孤儿代码怎么处理 | **接 translation-core + worker-service** | 已执行，见 §4.1。实测发现真正的根因是 `common/src/error.rs`（共享错误体系）从未被打开，必须先接它 |
| audit-service 取消按月分区是否符合设计意图 | **接受降级，保持现状** | schema 迁就代码，优先保证 Kafka 至少一次投递的幂等键正确。恢复分区需先拆一张非分区去重表，属独立变更。**这是显式降级，不是遗漏** |
| PR #19 审批 | Ulysses 本人执行 | `dev` 要求 1 人审批且 `enforce_admins`，admin 无法自批 |

> 拍板选项均以 `(推荐)` 标注推荐项，3 项均选中推荐项。

## §7 修订历史

| 版本 | 日期 | 变更 |
|---|---|---|
| v0.1 | — | 初始启动状态 |
| v0.2 | — | 45 个测试未执行的根因分析 |
| v0.3 | — | 45 个 e2e 首次跑通；`docker compose up` 从 0 可用性到跑通（12 处缺陷） |
| **v0.4** | **2026-10-04** | 撤回 v0.3 §4 的"12 路 healthz 全绿"覆盖表述；新增 lint 规则 6/7；抓出 translation-core P0 + 7 处死 env + 5731 行未编译代码；**接线 4 个 crate（约 1450 行），根因是共享错误体系从未打开** |

## 附：本轮新增/修改

**闸门**
- `deploy/scripts/lint-compose.py` — 新增规则 6（env 变量名交叉，含仓内 path 依赖）、规则 7（死文件 + 基线）；失败路径也输出提醒；支持位置参数以便对历史版本做双向验证
- `deploy/scripts/mvp-backend-up.sh` — 探针清单改为从 compose 推导，闭合覆盖漏洞；支持多端口 service

**编排**
- `deploy/docker-compose-mvp.yml` — 删 7 个死变量 + 1 个无人读取的卷；translation-core 补 HTTP healthz 端口；worker-service 恢复 `DATABASE_URL`（接线后它真的读了）
- `.github/workflows/ci-rust-test.yaml` — lint 步骤注释更新为 7 条规则各自的实证口径

**接线（本次新活）**
- `crates/common/src/lib.rs` — 加 `pub mod error;` + 根路径 re-export
- `crates/common/Cargo.toml` — 加 `actix-web` / `tonic` / `sqlx`
- `crates/cats-rbac/src/lib.rs` — 加 `pub mod service_helpers;`
- `crates/cats-rbac/Cargo.toml` — 加 `actix-web` / `uuid`
- `crates/translation-core/` — `lib.rs` 声明 6 个模块；`main.rs` 改为 tonic gRPC(50051) + HTTP healthz(8080) 双监听；`Cargo.toml` 加 `async-trait` / `uuid`
- `crates/translation-core/src/{service,qa,ai_gateway}.rs` — 修 prost 类型名、枚举 i32 转换、`code()` 方法名、一个 move-after-borrow
- `crates/worker-service/` — `lib.rs` 声明 3 个模块；`main.rs` 改为建池 + 启动调度器 + 注册 3 条路由；`Cargo.toml` 加 `cats-rbac` / `uuid`

**文档**
- `deploy/COMPOSE_UP_DEFECTS_v1.0.md` — 追加缺陷 13/14/15 与 §更正
- `deploy/BACKEND_STATUS_v0.4.md` — 本文件

