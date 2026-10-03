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

### §4.1 孤儿代码的清债方案待拍板

| 选项 | 代价 | 收益 |
|---|---|---|
| A. 只保留闸门，先不接线 | 0 | 债务可见、不再扩大 |
| B. 接 translation-core + worker-service | 中（需定 HTTP/gRPC 传输形态、DB 接入） | 两个 service 从"只有 healthz"变成真服务 |
| C. 接全部 12 个 crate | 大 | `cats-mock` 2899 行复活，e2e 可用 mock 层 |

`cats-bff` 809 行卡在 4 个不存在的 `Config` API 上（v0.3 §6），
`common/error.rs` 550 行与 `lib.rs` inline 版本重复，**改代码的人很容易去改
那份不参与编译的**——这是当前最实际的风险。

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
**4 个自报错了 + 整体 6 种格式不统一**。以本节表格为准。

**未修**：统一它要动 12 个 crate 的 healthz handler + 改共享 API，
不属于本轮范围，记在这里避免下一个人以为 healthz 格式已经统一。


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

run **`37157912198`**（commit `5a9a74a`）：**4/4 绿**

| job | 结果 |
|---|---|
| test (ubuntu-latest) | success（含 `Lint docker-compose + envoy invariants` 步骤） |
| test (macos-latest) | success |
| test (windows-latest) | success |
| e2e (real PostgreSQL) | success |

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

## §6 修订历史

| 版本 | 日期 | 变更 |
|---|---|---|
| v0.1 | — | 初始启动状态 |
| v0.2 | — | 45 个测试未执行的根因分析 |
| v0.3 | — | 45 个 e2e 首次跑通；`docker compose up` 从 0 可用性到跑通（12 处缺陷） |
| **v0.4** | **2026-10-04** | 撤回 v0.3 §4 的"12 路 healthz 全绿"覆盖表述；新增 lint 规则 6/7；抓出 translation-core P0 + 7 处死 env + 5731 行未编译代码 |

## 附：本轮新增/修改

- `deploy/scripts/lint-compose.py` — 新增规则 6（env 变量名交叉，含 path 依赖）、规则 7（死文件 + 基线）；失败路径也输出提醒；支持位置参数以便对历史版本做双向验证
- `deploy/scripts/mvp-backend-up.sh` — 探针清单改为从 compose 推导，闭合覆盖漏洞
- `deploy/docker-compose-mvp.yml` — 删 7 个死变量 + 1 个无人读���的卷；清掉对已删变量无意义的 `depends_on`
- `crates/translation-core/src/main.rs` — 修 `GRPC_BIND_ADDR` P0 + 3 个单测
- `deploy/COMPOSE_UP_DEFECTS_v1.0.md` — 追加缺陷 13/14/15 与 §更正
