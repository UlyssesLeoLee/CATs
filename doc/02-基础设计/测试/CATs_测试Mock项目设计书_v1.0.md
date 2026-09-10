# CATs 测试 Mock 项目设计书 v1.0

| 项目代号 | `cats-mock` |
| --- | --- |
| 文档版本 | v1.0 |
| 状态 | 📝 草案 |
| 编写者 | 架构师 (Mavis 接手 agent per DEC-008) |
| 修订人 | Ulysses (一人公司 12 角色 per DEC-008) |
| 审批 | 架构师 (Mavis 接手 agent per DEC-008) + 自审 |
| 适用 | CATs M0 → M2 全部 16 服务测试基础设施 |
| 引用 | 微服务架构书 v1.0 §4.1 / Rust 选型书 v1.0 §4-§11 / 实施前 QA 登记册 v1.3 §3.4 |

---

## 第 1 章 背景与目标

### 1.1 背景

CATs 16 个 Rust 微服务在 `cargo test --workspace --all-features --locked` 约束下,目前 63% 的"单元测试"是版本号字符串断言(per 2026-08-31 UT 审查报告),业务真正被覆盖的代码集中在 4 个 crate。其根因:

1. **缺乏统一 mock 工厂** — 16 个 service 各自重复 `version_is_semver_like` / `name_matches_crate` 占位符,只为过 CI "至少 1 test" 硬约束
2. **缺乏 fixture 共享层** — `audit-service::DbAuditSink` 的 `ON CONFLICT DO NOTHING` 幂等性零测试,`verify_jwt` 的 `token_type` 校验缺边界用例,根本原因是每次写测试都要手搓一遍用户/项目/任务/审计对象
3. **缺乏基础设施 mock** — worker-service / audit-service / notification-service 的 Kafka / Redis 集成测试只能靠真容器,CI 重,本地启动慢
4. **缺乏测试设计书** — 没有 "Mock 项目" 这一层的规约,新增 service 不知道该建什么 mock

### 1.2 目标

| 目标编号 | 描述 | 验收判据 |
| --- | --- | --- |
| G-1 | 统一 4 类 mock 入口(HTTP / DB / 基础设施 / 业务数据) | `use cats_mock::*` 一次性拿全 |
| G-2 | 替换 16 个 service 的 `tests/smoke.rs` boilerplate | 每个 `smoke.rs` 行数 < 5,内容仅 `use cats_mock::smoke::*;` |
| G-3 | 提 16 个 service 的可测面 | `cargo llvm-cov --workspace --lcov --fail-under-lines 40` 在 CI 跑通 |
| G-4 | 业务对象 factory 默认 random、可 deterministic | 同一 `fixed_id` 两次 build 出同一 id |
| G-5 | 基础设施 mock 零外部依赖(默认 in-memory) | `cargo test` 不依赖 docker / redis / kafka 即可跑 |

### 1.3 非目标

- **不替代真 service 路由测试** — `cats-mock` 仅提供 "能跑通端到端" 的响应,具体业务逻辑仍由 service 自己的单测覆盖
- **不实现完整 rdkafka / redis 协议** — 仅覆盖测试高频子集(produce / consume / KV / SET / ZSET)
- **不在 M0 阶段启用 testcontainers** — 默认 in-memory,真 PG 仅 CI 有 docker 时启用(stub 已留接口)

---

## 第 2 章 总体架构

### 2.1 模块拓扑

```
crates/cats-mock/
├── Cargo.toml              # workspace 成员
├── README.md               # 5 分钟上手
└── src/
    ├── lib.rs              # 4 大模块 re-export + crate 元信息
    ├── data/               # 业务对象 factory
    │   ├── mod.rs          # Factory trait + 通用枚举
    │   ├── user.rs         # UserFactory
    │   ├── project.rs      # ProjectFactory
    │   ├── task.rs         # TaskFactory
    │   └── audit.rs        # AuditEventFactory
    ├── db/                 # DB fixture
    │   ├── mod.rs          # 入口
    │   ├── schema.rs       # SQL DDL 集合 (per 16 service)
    │   ├── seed.rs         # INSERT 工厂
    │   └── fixture.rs      # InMemoryDbFixture + PgDbFixture (stub)
    ├── infra/              # 基础设施 mock
    │   ├── mod.rs          # 入口
    │   ├── kafka.rs        # MockKafka
    │   └── redis.rs        # MockRedis
    └── http/               # HTTP API mock
        ├── mod.rs          # 入口
        ├── response.rs     # ResponseBuilder + MockError + ErrorBody
        ├── routes.rs       # 预制业务路由 (auth/user/project/task/audit)
        └── server.rs       # MockServer (actix-web 启动)
```

### 2.2 依赖矩阵 (workspace.dependencies 新增)

| 依赖 | 版本 | 用途 | 引用 |
| --- | --- | --- | --- |
| `chrono` | 0.4 + serde | 时间戳 | 业务对象 created_at |
| `fake` | 2 | 真实感数据生成 | user/project 描述 (注: faker 路径跨版本不稳时回退到自定义) |
| `rand` | 0.8 | 随机数 | ID / email / 边界用例 |
| `reqwest` | 0.12 (rustls-tls) | HTTP 客户端 (e2e 验证) | MockServer 单测 |
| `pgvector` | 0.4 (sqlx) | 向量类型 (翻译 unit) | translation_unit schema |
| `cats-mock` | path = "crates/cats-mock" | 自引用 (供其它 service 加 dev-dep) | 16 service 引用 |

### 2.3 与既有 crate 的关系

| 既有的 | 与 cats-mock 关系 |
| --- | --- |
| `cats-common` | 提供 tracing / 配置初始化;`cats-mock` 复用其错误/日志 trait |
| `cats-proto` | 提供 gRPC contract;`cats-mock` 不重写 proto,仅 mock 业务对象 |
| `m1-s0-smoke` | M0 基线库兼容 smoke;`cats-mock` 是更高层的业务 mock,层级在上 |
| 16 service `tests/smoke.rs` | 全部 `use cats_mock::smoke::*;` 替代 boilerplate |

---

## 第 3 章 接口规约

### 3.1 公开 API (按模块)

#### 3.1.1 `cats_mock::data`

| 类型 | 说明 |
| --- | --- |
| `Factory` (trait) | 统一 `build()` / `build_many(count)` |
| `UserFactory` / `User` / `UserDbRow` | User 工厂 + 数据 + DB 行 |
| `ProjectFactory` / `Project` | Project 工厂 + 数据 |
| `TaskFactory` / `Task` | Task 工厂 + 数据 |
| `AuditEventFactory` / `AuditEvent` / `AuditOutcome` | AuditEvent 工厂 + 数据 + 结果枚举 |
| `ProjectStatus` / `TaskStatus` | 业务状态机 (with `all()` / `as_str()`) |
| `now_utc()` / `random_past_within_days(n)` / `pick_random(&[T])` | 时间/随机工具 |
| `invalid_emails()` / `weak_passwords()` | 边界用例 fixture |

#### 3.1.2 `cats_mock::db`

| 类型 | 说明 |
| --- | --- |
| `SchemaSet` | DDL 集合,`push()` / `merge()` / `all_common()` |
| `users_schema()` / `projects_schema()` / `tasks_schema()` / `audit_log_schema()` / `translation_unit_schema()` | 5 类预制 DDL |
| `SchemaSet::pgvector_ext()` | pgvector 扩展 |
| `SeedSet` / `SeedEntry` / `SeedParam` | 插入语句 + 参数 |
| `users_default()` / `projects_default()` / `tasks_default()` / `audit_events_default()` | 4 类预制 seed |
| `users_from(&[User])` / `projects_from(&[Project])` / `tasks_from(&[Task])` / `audit_events_from(&[AuditEvent])` | 从 factory 产出转 seed |
| `DbFixture` (trait) | 统一 `apply_schema()` / `apply_seed()` / `truncate_all()` / `shutdown()` |
| `InMemoryDbFixture` | 默认实现,无外部依赖 |
| `PgDbFixture` | testcontainers stub, CI 启用 |
| `in_memory()` / `pg(dsn)` | 工厂入口 |

#### 3.1.3 `cats_mock::infra`

| 类型 | 说明 |
| --- | --- |
| `MockKafka` / `KafkaMessage` | `produce(topic, key, value)` / `consume(topic)` / `peek(topic)` / `queue_len(topic)` / `topic_list()` / `produced()` / `consumed()` / `clear()` |
| `MockRedis` | KV: `set` / `get` / `del` / `exists` / `incr` / `decr` / `incr_by` / `expire` + SET: `sadd` / `smembers` / `srem` / `scard` + ZSET: `zadd` / `zscore` / `zrange` + `clear` |

#### 3.1.4 `cats_mock::http`

| 类型 | 说明 |
| --- | --- |
| `MockServer` / `MockServerConfig` | 启动 actix-web server, 0.0.0.0:0 随机端口 |
| `ResponseBuilder` | `ok()` / `created()` / `no_content()` / `err(e)` / `err_with_msg(e, msg)` |
| `MockError` | `BadRequest` / `Unauthorized` / `Forbidden` / `NotFound` / `Conflict` / `InternalServerError` / `ServiceUnavailable` |
| `ErrorBody` | 统一错误体 (per 实施前 QA §3.4) |
| `auth_routes()` / `user_routes()` / `project_routes()` / `task_routes()` / `audit_routes()` / `healthz_routes()` | 6 组预制业务路由 |

### 3.2 入口 (lib.rs)

```rust
// 4 大模块 + 通用
pub mod data;
pub mod db;
pub mod http;
pub mod infra;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");
pub fn version() -> &'static str;
pub fn name() -> &'static str;
```

### 3.3 用法示例

#### 3.3.1 业务数据

```rust
use cats_mock::data::{UserFactory, Factory};

let alice = UserFactory::new()
    .with_username("alice")
    .with_email("alice@example.com")
    .build();

let users = UserFactory::new().build_many(10);
```

#### 3.3.2 DB fixture (in-memory)

```rust
use cats_mock::db::{in_memory, SchemaSet, seed};

let db = in_memory();
db.apply_schema(SchemaSet::all_common()).await.unwrap();
db.apply_seed(seed::users_default()).await.unwrap();

assert_eq!(db.row_count("users_credential"), 1);
db.truncate_all().await.unwrap();
```

#### 3.3.3 基础设施 mock

```rust
use cats_mock::infra::{MockKafka, MockRedis};

let kafka = MockKafka::new();
kafka.produce("audit.event", Some("user-1"), b"{\"x\":1}");
let msg = kafka.consume("audit.event").expect("one msg");

let redis = MockRedis::new();
redis.set("counter", "0");
assert_eq!(redis.incr("counter"), 1);
```

#### 3.3.4 HTTP e2e

```rust
use cats_mock::http::{MockServer, MockServerConfig};

let server = MockServer::start(MockServerConfig::all()).await.unwrap();
let base = server.base_url();

let client = reqwest::Client::new();
let resp = client
    .post(format!("{base}/v1/auth/login"))
    .json(&serde_json::json!({"username": "alice", "password": "p"}))
    .send()
    .await
    .unwrap();
assert_eq!(resp.status().as_u16(), 200);
```

---

## 第 4 章 数据设计

### 4.1 业务对象 schema (与各 service 模型对齐)

| 对象 | 字段 | 对齐 |
| --- | --- | --- |
| `User` | `id` (UUID) / `username` (UNIQUE) / `email` / `password_hash` (argon2 形) / `is_active` / `created_at` / `updated_at` | `auth-service::models::UserCredential` + `user-service::models::User` |
| `Project` | `id` / `owner_id` / `name` / `description?` / `source_lang` / `target_lang` / `status` (ProjectStatus) / `created_at` / `updated_at` | `project-service::models::Project` |
| `Task` | `id` / `project_id` / `task_type` / `payload` (JSON) / `status` (TaskStatus) / `attempts` (INT) / `last_error?` / `created_at` / `updated_at` | `task-service::models::Task` |
| `AuditEvent` | `event_id` (PK) / `user_id?` / `event_type` / `outcome` (AuditOutcome) / `detail?` (JSON) / `source_ip?` (INET) / `user_agent?` / `occurred_at` | `auth-service::models::AuditEvent` |

### 4.2 状态机枚举

#### 4.2.1 `ProjectStatus`

| 变体 | 字符串 | 说明 |
| --- | --- | --- |
| `Draft` | `draft` | 初始态, owner 还没提交 |
| `Active` | `active` | 已激活, 接受新 task |
| `Archived` | `archived` | 归档, 只读 |
| `Deleted` | `deleted` | 软删除, 30 天后清理 |

#### 4.2.2 `TaskStatus`

| 变体 | 字符串 | 说明 |
| --- | --- | --- |
| `Pending` | `pending` | 客户端已创建, 等 worker 拉 |
| `Queued` | `queued` | 已入队, 等 worker |
| `Running` | `running` | 正在执行 |
| `Succeeded` | `succeeded` | 成功 |
| `Failed` | `failed` | 失败, `last_error` 必填 |
| `Cancelled` | `cancelled` | 用户取消 |

#### 4.2.3 `AuditOutcome`

| 变体 | 字符串 |
| --- | --- |
| `Success` | `success` |
| `Failure` | `failure` |

### 4.3 DB schema (DDL 摘要)

| 表 | 关键列 | 索引 |
| --- | --- | --- |
| `users_credential` | `id` (PK) / `username` (UNIQUE) / `password_hash` / `is_active` | `idx_users_credential_username` |
| `user_profile` | `user_id` (PK, FK→users_credential) | — |
| `project` | `id` (PK) / `owner_id` / `status` | `idx_project_owner_id` / `idx_project_status` |
| `task` | `id` (PK) / `project_id` / `status` | `idx_task_project_id` / `idx_task_status` |
| `audit_log` | `event_id` (PK) / `user_id` / `event_type` / `occurred_at` | `idx_audit_log_user_id` / `idx_audit_log_event_type` / `idx_audit_log_occurred_at` |
| `translation_unit` | `id` (PK) / `project_id` / `embedding` (vector(384)) | — (pgvector 扩展) |

> **DDL 同步约束**: 每次 service migration 变更, `cats-mock` 的 `db::schema` 必须同步更新。由 Test Quality Lead 兜底 (per 2026-08-31 UT 审查报告 §4)。

### 4.4 基础设施数据

| 模块 | 数据结构 | 持久化 |
| --- | --- | --- |
| `MockKafka` | `HashMap<Topic, VecDeque<KafkaMessage>>` | `Mutex` |
| `MockRedis` | `HashMap<String, (String, Option<Duration>)>` + `HashMap<String, BTreeSet<String>>` + `HashMap<String, BTreeMap<String, f64>>` | `Mutex` |
| `InMemoryDbFixture` | `HashMap<Table, Vec<Vec<String>>>` | `Mutex` |

---

## 第 5 章 质量与测试

### 5.1 cats-mock 自身的单测覆盖

| 模块 | 测试函数 | 关键断言 |
| --- | --- | --- |
| `lib` | 3 | version / name / 模块 re-export 存在性 |
| `data::user` | 8 | id 唯一 / fixed_id / inactive / 自定义字段 / 批量 / 时间范围 / 边界 email / DB row 一致 |
| `data::project` | 6 | 默认 Draft / owned_by / 状态机 / lang_pair / 批量 / 字符串回环 |
| `data::task` | 6 | 默认 translate / failed / of_type / for_project / 批量 / 字符串回环 |
| `data::audit` | 7 | 默认 login_success / failed / by_user / with_detail / 自定义 IP+UA / 批量 / 字符串回环 |
| `data` 通用 | 4 | ProjectStatus/TaskStatus 字符串 / pick_random / 边界 email |
| `db::schema` | 5 | 4 表齐全 / pgvector ext / merge 顺序 / UNIQUE / PK |
| `db::seed` | 8 | 各 default 1 条 / 批量 / ON CONFLICT / placeholder 对齐 / 空 seed / outcome 字符串 |
| `db::fixture` | 6 | schema 4 表 / seed 行数 / truncate / 提取表名 / pg stub |
| `infra::kafka` | 7 | produce+consume / offset 递增 / 空 consume / peek 不消费 / topic_list / counter / clear |
| `infra::redis` | 10 | set+get / del / exists / incr+decr / incr_by / sadd+smembers+srem / zadd+zscore+zrange / zrange 部分 / expire / clear |
| `http::response` | 8 | ErrorBody JSON / detail / status 码 / 名称规范 / builder 5 个状态 |
| `http::routes` | 7 | auth login / user get 200 / user get 404 / project list / task create 201 / healthz / 全部注册 |
| `http::server` | 4 | start healthz / start all+routes / config all / config default |
| **合计** | **89** | — |

### 5.2 与 16 service 的集成方式

| service | 当前 smoke.rs 行数 | 改后 | 引用方式 |
| --- | --- | --- | --- |
| asr / audit / cats-bff / file / ingestion / notification / ocr / office-converter / project / render-writer / report / subtitle / task / translation-core / worker | 16 | 0 (删除) | `use cats_mock::smoke::*;` (M1 加 smoke 模块) |
| auth | 12 | 0 | 同上, 业务单测用 `cats_mock::data::UserFactory` |
| user | 2 | 0 | 同上, e2e 用 `cats_mock::http::MockServer` |
| common | 3 | 0 | — (common 自身) |
| m1-s0-smoke | 5 | 0 | — (smoke crate 自身) |

### 5.3 CI 集成

```yaml
# ci/github-actions/ci-rust-test.yaml (建议追加)
- name: Mock self-test
  run: cargo test -p cats-mock --locked -- --nocapture

- name: Coverage gate
  if: matrix.os == 'ubuntu-latest'
  run: |
    cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info
    cargo llvm-cov --workspace --all-features --fail-under-lines 40
```

### 5.4 验收门槛

| 项 | 门槛 |
| --- | --- |
| cats-mock 自身单测 | 89/89 通过 (本文件交付时实测) |
| workspace 单测 (替换 boilerplate 后) | 16 个 smoke.rs 全部 0 行, 全 workspace `cargo test` 仍 ≥ 95% 通过 |
| 行覆盖率 | `cargo llvm-cov --fail-under-lines 40` 通过 |
| `cargo clippy -p cats-mock --all-targets -- -D warnings` | 通过 |
| `cargo fmt -p cats-mock --check` | 通过 |

---

## 第 6 章 部署与演进

### 6.1 当前交付 (v0.1 / M0)

- 4 大模块 in-memory 全部就位
- 89 个单测覆盖
- 0 个外部依赖 (默认)
- workspace 成员注册完毕

### 6.2 路线图

| 版本 | 范围 | 触发条件 | 负责人 |
| --- | --- | --- | --- |
| v0.2 | 接入 testcontainers, `PgDbFixture` 真 PG 实现 | M1 启动, CI 有 docker | Test Quality Lead |
| v0.3 | 引入 `proptest` 做 property-based factory | M1-Sprint 2 | Test Quality Lead |
| v0.4 | 接入 K3s 真实 Kafka / Redis (per OI-3) | M2 | SRE Lead + Test Quality Lead |
| v1.0 | 16 service smoke.rs 全部删除, 替换为 `cats_mock::smoke::*` | M1-Sprint 3 | 5 域 Lead 联合 |

### 6.3 风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
| --- | --- | --- | --- |
| 16 service migration 变更, `db::schema` 漏同步 | 中 | 测试用错表, 假阳/假阴 | Test Quality Lead 守门, PR review 必查 |
| `reqwest` / `fake` 升版 API 破坏 | 中 | cats-mock 编译失败 | `Cargo.lock` + `cargo update -p fake` 单独 PR, 不批量升 |
| 真实 PG / Kafka 在 K3s 阶段二与 mock 行为偏离 | 低 | e2e 跑过但 prod 挂 | 关键路径 (audit 幂等 / JWT verify) 在 K3s 跑前先 mock 测再真测 |
| in-memory mock 性能退化 (大 fixture) | 低 | 单测变慢 | factory 默认 `build_many(10)`, 性能测试独立 fixture |
| 5 域 service 各自造私有 mock, 绕过 cats-mock | 中 | mock 重复, 维护分散 | 5 域 Lead 协议: 任何新增 mock 必须先 review "是否能进 cats-mock" |

### 6.4 跨域 RACI

| 活动 | 架构师 (Mavis) | SRE Lead | 5 域 Lead (auth/user/project/task/audit/...) | Test Quality Lead | PM |
| --- | --- | --- | --- | --- | --- |
| cats-mock 公共 API 设计 | A | C | C | R | I |
| service migration 同步到 db::schema | I | — | R | A | I |
| testcontainers 启用 | I | A | — | R | I |
| K3s 真 Kafka/Redis 接入 | I | A | — | R | I |
| 各 service smoke.rs 替换 | I | — | R | A | I |
| 覆盖率门槛 CI 化 | A | C | I | R | I |

> R = Responsible / A = Accountable / C = Consulted / I = Informed

---

## 附录 A:修订历史

| 版本 | 日期 | 修订内容 | 修订人 | 审批 |
| --- | --- | --- | --- | --- |
| v1.0 | 2026-08-31 | 初版:4 大模块 + 89 单测 + 路线图 v0.2-v1.0 | Ulysses (一人公司 12 角色 per DEC-008)— Mavis 接手 | 架构师 (Mavis 接手 agent per DEC-008) + 自审 |

## 附录 B:已知缺口

- [ ] `db::schema::translation_unit_schema()` 已在, 但没在 `all_common()` 里 — 翻译 core 单独引
- [ ] `MockKafka` 单 partition 简化, 多 partition 留给 v0.2
- [ ] `MockRedis` 的 TTL 用 "插入时记 ttl" 简化, 真用时改 wall-clock 校验
- [ ] `PgDbFixture` 是 stub, 真 PG impl 留 v0.2
- [ ] `http::routes` 默认返回固定 mock JSON, dynamic factory (用 data::UserFactory 产出) 留 v0.3
- [ ] `http::response` 的 `ErrorBody` 与 `auth-service::models::ErrorBody` 字段一致但未共享, 留 v0.2 抽到 `cats-common`
