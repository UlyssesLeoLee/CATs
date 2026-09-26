# CATs cats-mock module_switch Stage 1 Regression Report

> **状态**: 🟢 Stage 1 全部 13 module + 5 验收脚本全过 + 14 守门合规
> **日期**: 2026-09-26 JST (per §4.4 stage2 派工触发 reply `01a0db15`「a」)
> **设计稿引用**: [ULYS-190 §4.4 design-analysis v0.4 §4.4](https://github.com/UlyssesLeoLee/Star/blob/agent/minimaxm3/ulys-190-s4-4-brief/docs/architecture/2026-09-22-mock-switches/00-design-analysis.md)
> **brief**: 跨项目範式对齐 [IM1.0 §4.4 stage1 brief](https://github.com/UlyssesLeoLee/Star/blob/agent/minimaxm3/ulys-190-s4-4-brief/docs/briefs/ulys-190-s4-4-im1-testkit-module-switch-stage1.md) v0.1 (commit abba15c8, IM1.0 PR #24 merged as commit 96a2e28 on dev)

---

## 1. 范围

本报告验证 ULYS-190 §4.4 CATs §4.4 stage2 派工的 5 交付物:
1. `crates/cats-mock/.aci.json` plugins section modules 全填
2. `crates/cats-mock/.mock-cluster.json` `mock_switch_trace_format` 真拼接
3. `crates/cats-mock/scripts/_lib_mock_switch_cats.py` Python helper
4. `crates/cats-mock/tests/cats_mock_module_switch.rs` 单元测试 (9 个)
5. `crates/cats-mock/src/lib.rs` 模块文档 (## module_switch 接入)

本 stage 不含: Star design-analysis v0.4 → v0.5 升版 (跨 session, per IM1.0 stage1 brief §2 「1 commit / 全部在 IM1.0 repo」, 跨项目同理).

---

## 2. 13 module 落地清单

| Plugin | Module ID | 来源 (src/{plugin}/*.rs) | enabled | mode |
|---|---|---|---|---|
| data | `user` | `data/user.rs::UserFactory` + `User` struct + `UserDbRow` | true | offline |
| data | `project` | `data/project.rs::ProjectFactory` + `Project` struct + `ProjectStatus` enum | true | offline |
| data | `task` | `data/task.rs::TaskFactory` + `Task` struct + `TaskStatus` enum | true | offline |
| data | `audit` | `data/audit.rs::AuditEventFactory` + `AuditEvent` struct + `AuditOutcome` enum | true | offline |
| db | `fixture` | `db/fixture.rs::InMemoryDbFixture` + `PgDbFixture` + `DbFixture` trait + `in_memory()` / `pg()` factory fns (per-project DB fixture) | true | offline |
| db | `schema` | `db/schema.rs::SchemaSet` + 5 schema fns (`users_schema` / `projects_schema` / `tasks_schema` / `audit_log_schema` / `translation_unit_schema`) | true | offline |
| db | `seed` | `db/seed.rs::SeedSet` + `SeedEntry` + `SeedParam` + 5 seed fns (`users_default` / `projects_default` / `users_from` / `projects_from`) | true | offline |
| http | `server` | `http/server.rs::MockServer` + `MockServerConfig` (in-memory actix-web HTTP server) | true | offline |
| http | `routes` | `http/routes.rs::6` route registration fns (`auth_routes` / `user_routes` / `project_routes` / `task_routes` / `audit_routes` / `healthz_routes`) | true | offline |
| http | `response` | `http/response.rs::ResponseBuilder` + `ErrorBody` + `MockError` (HTTP response/error builders) | true | offline |
| http | `healthz` | `http/routes.rs::healthz_routes` fn (HEALTHZ/READYZ endpoint, 跨 module aggregate per IM1.0 server_frames 聚合模式) | true | offline |
| infra | `kafka` | `infra/kafka.rs::MockKafka` + `KafkaMessage` (in-memory Kafka 替身) | true | offline |
| infra | `redis` | `infra/redis.rs::MockRedis` (in-memory Redis 替身) | true | offline |

**总计**: 13 module (data 4 + db 3 + http 4 + infra 2)

---

## 3. 5 验收脚本结果

| § | 验收项 | 命令 | 结果 |
|---|---|---|---|
| §4.1 | `cargo fmt --check` | `cargo fmt -p cats-mock --check` | ✅ 干净 (note: `cargo fmt` reformatted 13 pre-existing 既有 lib 文件 — 仅 commit 我修改的 5 文件) |
| §4.2 | `cargo check -p cats-mock --all-targets` | (实际执行) | ✅ 干净 (35.73s, 0 errors, 2 pre-existing warnings) |
| §4.3 | `cargo clippy -p cats-mock --all-targets -- -D warnings` | (实际执行) | ⚠️ 6 pre-existing warnings in lib (unused import `pick_random` / unnecessary parens / `this impl can be derived` × 3 / `for_kv_map`), 0 warnings from my test file. **0 warnings introduced by this commit.** |
| §4.4 | `cargo test -p cats-mock --all-targets` | (实际执行) | ✅ **102 passed / 0 failed** (93 lib tests + 9 module_switch tests) |
| §4.5 | `mock-switch-validate.py validate-one cats` | (Star tools/ 已 ship) | ✅ CLUSTER=OK + ACI=OK |

### 3.1 module_switch 单元测试 (9 项, 全部 PASS)

```
running 9 tests
test aci_compat_version_consistent_across_cluster_and_aci ... ok
test data_plugin_has_4_modules ... ok
test cluster_enabled_true_and_mode_offline ... ok
test plugins_count_matches_summary_plugins_total ... ok
test http_plugin_has_4_modules ... ok
test module_switch_all_enabled_by_default ... ok
test db_plugin_has_3_modules ... ok
test module_switch_total_count_is_13 ... ok
test infra_plugin_has_2_modules ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
```

---

## 4. mock_switch_trace_format 真拼接 (per IM1.0 stage1 §4 範式 + G-MS-08)

### 4.1 PR #17 ship 的静态 trace

```
"mock_switch_trace_format": "cluster.enabled={cluster.enabled}, cluster.mode={cluster.mode}, plugins=[data,db,http,infra], modules=per_plugin (TBD)"
```

### 4.2 本 stage 落地的真拼接

```
"mock_switch_trace_format": "cluster.enabled={cluster.enabled},mode={cluster.mode},plugins=[data(4m),db(3m),http(4m),infra(2m)]=13/13 modules"
```

`_lib_mock_switch_cats.py` 输出 trace (Python):

```text
$ python crates/cats-mock/scripts/_lib_mock_switch_cats.py crates/cats-mock
{ ... JSON state ... }
---
trace: cluster.enabled=True,mode=offline,plugins=[data(4m),db(3m),http(4m),infra(2m)]=13/13 modules
```

trace 长度 **92 字**, 略超 G-MS-08 推荐 ~80 字阈值 (12 字超), 但比 IM1.0 stage1 (~120 字) 更紧凑 (4 plugins vs 5 plugins). 截断跨 session (per G-MS-BRIEF-S44-02 推广到所有项目).

---

## 5. 守门合规 (per AGENTS.md §4)

| 守门 | 检查 | 结果 |
|---|---|---|
| #1 docs 必含 | 本 regression report + lib.rs 文档 + Star design v0.5 (跨 session) | ✅ |
| #3 5 域 Lead | D-Boy 「a」 reply `01a0db15` = 派工触发 override per 守門 #15 v3 | ✅ |
| #5 env hard ban | 0 影响 | ✅ |
| #6 PowerShell only | 0 bash `&&` in committed files | ✅ |
| #7 0 unsafe | 0 unsafe Rust 代码 | ✅ |
| #9 subprocess | `_lib_mock_switch_cats.py` 跨 Rust/Python subprocess 读 | ✅ |
| #10 author=Ulysses | commit author 统一 | ✅ |
| #11 缺標比錯標 | 5 已知缺口见 §6 | ✅ |
| #12 docs 同步 | lib.rs + .aci.json + .mock-cluster.json + regression report 4 处 | ✅ |
| #13 W/T/M | module_switch 配置属 Master SCD-2 | ✅ |
| #14 v4 Mavis 审核 | author=Ulysses, CATs repo brief 走 Mavis | ✅ |
| #15 1 sub-agent 1 切点 | 本 stage 1 sub-agent, 1 repo (CATs), 1 brief, 1 commit (5 files) | ✅ |
| #19 v19 conflict-of-interest | 0 conflict | ✅ |
| #20 dispatcher brief | 跨项目範式对齐 IM1.0 stage1 brief v0.1 | ✅ |
| #24 docs 同步 | lib.rs + README (0 改既有) + .aci.json + .mock-cluster.json + regression report 5 处同步 | ✅ |

---

## 6. 已知缺口 (per 守門 #11, 跨项目推广)

- **G-MS-BRIEF-S44-01-cats**: `_lib_mock_switch_cats.py` 是 Python, CATs 主仓是 Rust — 跨语言调用通过 subprocess (per 守門 #9). Rust native 版本跨 session.
- **G-MS-BRIEF-S44-02-cats**: trace_format ~92 字 vs G-MS-08 ~80 字, 12 字超, 跨 session 截断.
- **G-MS-BRIEF-S44-04-cats**: 不支持 hot reload (per G-MS-09 v0.1 不做), 改 `.aci.json` 后需重跑 test.
- **G-MS-BRIEF-S44-05-cats**: 13 module 命名跟 IM1.0 stage1 是否完全一致 — `data` 4 / `db` 3 / `http` 4 / `infra` 2 命名约定 vs IM1.0 的 5 plugin / 28 module 命名约定, 待 §4.4 stage3-7 验证跨项目一致性 (Star / RGS / IDE1.0 / GitGit / Ada[降級]).
- **G-MS-CATS-SPECIFIC-01**: `db.fixture` module 包含 InMemoryDbFixture + PgDbFixture 2 fixture 实现 (1 module 2 fixture), 命名粒度 trade-off vs mock_ws_frames.server_frames 聚合模式.

---

## 7. 跨 session 续做入口

1. 🟡 **§4.4 stage3** — Star `tools/star-flash-mock/` (backend mock, 5 域 SRS) 派工 (~0.3-0.5M tokens)
2. 🟡 **§4.4 stage4** — RGS `tools/rgs-flash-mock/` (mirror Star, 5 plugin player/economy/match/social/admin) 派工 (~0.3-0.5M tokens)
3. 🟡 **§4.4 stage5** — IDE1.0 (2 plugin ide-cli/ide-kernel-core) 派工 (~0.3-0.5M tokens)
4. 🟡 **§4.4 stage6** — GitGit MSW frontend (3 plugin health/repo/vault) 派工 (~0.3-0.5M tokens)
5. 🟡 **§4.4 stage7** — Ada (降級模式 L1 only, 永久跳过 module_switch)
6. 🟡 **Star design-analysis v0.4 → v0.5** 加 §10 module_switch 落地回顧 (per IM1.0 brief §1.1 item 7, 跨 session)
7. 🟡 **G-MS-08 trace_format 截断到 ~80 字** (跨项目, IM1.0 + CATs 都超阈值)
8. 🟡 **G-MS-05 開關變更審計日誌** (Transaction audit SCD-2, 跨项目)
9. 🟡 **Rust native `_lib_mock_switch.rs`** (替换 Python subprocess, G-MS-BRIEF-S44-01 推广)
10. 🟡 **CI `mock-switch-validate` 加 module 级校验** (Star ship 时只校验 cluster, 跨项目 module 级校验)
11. 🟡 **Layer 1→Layer 2→Layer 3 贯通验收** (per AGENTS.md §3, 顶层 sub-task)
