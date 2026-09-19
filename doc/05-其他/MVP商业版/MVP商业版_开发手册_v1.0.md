# CATs MVP 商业版 — 开发手册 v1.0

> **文档编号**：CATs-MVP-DEV-001
> **版本**：v1.0
> **创建日**：2026-09-19
> **作者**：架构师(Mavis 接手 agent per DEC-008)
> **审批**：架构师(Mavis 接手 agent per DEC-008) + 自审
> **修订人**：Ulysses（一人公司 12 角色 per DEC-008）— Mavis 接手
> **状态**：DDD Review 草稿（6 角色 7 天评审待补）
> **密级**：仅社内（MVP 客户技术对接团队可读）

---

## 0. 阅读指南

本书是 CATs MVP 商业版的**开发人员入门手册**，供客户方 IT 集成工程师 / 二次开发工程师 / CATs 内部新成员快速上手 monorepo + 9 服务 + cats-mock 测试体系。本书不替代各 crate 的 README，而是给出全局视角的索引。

---

## 1. Monorepo 入门

### 1.1 仓库结构

```
cats-monorepo/
├── Cargo.toml                    # workspace 根（22 crate 成员 + 公共依赖）
├── Cargo.lock                    # 锁版本文件（守门 #9 v19 强制 git 跟踪）
├── README.md                     # 仓库入口
├── crates/                       # 21 crate（16 业务 + cats-common + cats-proto + cats-mock + cats-rbac + cats-bff + m1-s0-smoke + translation-core）
│   ├── common/                   # cats-common: tracing / config / error trait 公共层
│   ├── proto/                    # cats-proto: gRPC protobuf 定义（commit 阶段锁版本）
│   ├── auth-service/             # 服务 1: 认证 + JWT 签发
│   ├── user-service/             # 服务 2: 用户 + 组织 + 订阅
│   ├── project-service/          # 服务 3: 项目 + 术语库 + TM
│   ├── task-service/             # 服务 4: 任务编排 + 媒体处理
│   ├── file-service/             # 服务 5: 文件存取 + MinIO 集成
│   ├── notification-service/     # 服务 6: WebSocket + 邮件推送
│   ├── report-service/           # 服务 7: 用量 / 计费 / QA 报表
│   ├── audit-service/            # 服务 8: 审计日志
│   ├── worker-service/           # 服务 9 (附): Cron / 批量导入 / 对账
│   ├── translation-core/         # 翻译核心（LangGraph 编排 + ai-gateway 集成）
│   ├── ingestion-service/        # 阶段二: 媒体文件识别
│   ├── asr-service/              # 阶段二: ASR (faster-whisper)
│   ├── ocr-service/              # 阶段二: OCR (PaddleOCR + Tesseract)
│   ├── subtitle-service/         # 阶段二: 字幕格式转换
│   ├── office-converter-service/ # 阶段二: Office 文档转换
│   ├── render-writer-service/    # 阶段二: 渲染 + 烧录
│   ├── cats-bff/                 # BFF 层：聚合客户端请求
│   ├── cats-rbac/                # RBAC 中间件（16 域共享）
│   ├── cats-mock/                # 测试 Mock 项目（4 大模块 in-memory）
│   └── m1-s0-smoke/              # M0 基线 smoke 兼容库
├── proto/                        # 顶层 proto 定义（与 crates/proto 同步）
├── doc/                          # 全文档树（设计 / 测试 / 评审 / 治理）
├── ci/                           # GitHub Actions / Gitea Actions 配置
├── k3s/                          # K3s 部署 YAML（main + mock-only/）
└── apps/                         # （预留）桌面客户端 + Web 控制台源码
```

### 1.2 环境要求

| 工具 | 版本 | 验证命令 |
|---|---|---|
| Rust | **1.98.0**（per 技术基线 v1.0）| `rustc --version` |
| PostgreSQL | **18.6** + pgvector **0.8.6** | `psql --version` |
| K3s | **v1.30+**（per 部署架构 v1.0）| `k3s --version` |
| Docker | 24.0+（testcontainers 可选）| `docker --version` |
| sqlx-cli | 0.8+ | `cargo install sqlx-cli --version 0.8.* --no-default-features --features rustls,postgres` |
| protoc | 25.0+ | `protoc --version` |
| Node.js | 20 LTS（apps/ 控制台）| `node --version` |

### 1.3 初始化

```bash
# 1. 克隆仓库
git clone https://harbor.cats.internal/cats/monorepo.git
cd monorepo

# 2. 安装 Rust 工具链
rustup toolchain install 1.98.0
rustup default 1.98.0

# 3. 验证 workspace
cargo check --workspace --all-features --locked

# 4. 启动 PostgreSQL（本地 docker）
docker run -d --name cats-pg \
  -e POSTGRES_PASSWORD=cats \
  -p 5432:5432 \
  -v cats-pg-data:/var/lib/postgresql/data \
  postgres:18.6

# 5. 安装 pgvector 扩展
docker exec cats-pg apt-get update
docker exec cats-pg apt-get install -y postgresql-18-pgvector

# 6. 创建数据库
psql -h localhost -U postgres -c "CREATE DATABASE cats_dev;"
psql -h localhost -U postgres -d cats_dev -c "CREATE EXTENSION pgvector;"

# 7. 启动 K3s（本地单节点）
curl -sfL https://get.k3s.io | sh -s - --cluster-init

# 8. 跑 mock 一键启动
bash crates/cats-mock/scripts/local-up.sh
```

### 1.4 常用 cargo 命令

```bash
# 单服务编译（最快）
cargo check -p auth-service --all-features

# 单服务测试（仅编译 + 单测）
cargo test -p auth-service --all-features --locked

# 单服务集成测试（含 cats-mock）
cargo test -p auth-service --test integration --all-features

# 整个 workspace 检查（守门 #1 L11 限 1 次，超时已知）
cargo check --workspace --all-features --locked

# 某个 crate 的 docs
cargo doc -p auth-service --no-deps --open

# 格式化（CI 强制）
cargo fmt --all

# Lint（CI 强制 -D warnings）
cargo clippy --workspace --all-features --locked -- -D warnings

# 覆盖率（CI 跑）
cargo llvm-cov --workspace --lcov --fail-under-lines 40
```

---

## 2. 9 服务 Owners + Crate 名 + 端口

| # | 服务 | Crate 路径 | Owner (角色) | REST 端口 | gRPC 端口 | 数据库 |
|---|---|---|---|---|---|---|
| 1 | auth-service | `crates/auth-service/` | RustLead + ArchitectLead | 8081 | 9091 | auth_db |
| 2 | user-service | `crates/user-service/` | RustLead + DBA | 8082 | 9092 | user_db |
| 3 | project-service | `crates/project-service/` | RustLead + DBA + ArchitectLead | 8083 | 9093 | project_db |
| 4 | task-service | `crates/task-service/` | RustLead + ArchitectLead | 8084 | 9094 | task_db |
| 5 | file-service | `crates/file-service/` | RustLead + PlatformLead | 8085 | 9095 | file_db + MinIO |
| 6 | notification-service | `crates/notification-service/` | RustLead | 8086 | 9096 | notification_db |
| 7 | report-service | `crates/report-service/` | RustLead + DBA | 8087 | 9097 | report_db |
| 8 | audit-service | `crates/audit-service/` | RustLead + QualityLead | 8088 | 9098 | audit_db |
| 9 | translation-core (+ ai-gateway) | `crates/translation-core/` | RustLead + ArchitectLead | 8089 | 9099 | project_db (共享) |

> 服务间调用规则：所有业务服务通过 gRPC `AuthCheck` 调用 auth-service；同步 SQL 直连 PG（不跨库）；异步通过 Kafka topic（阶段二启用）。

---

## 3. cats-mock 测试体系（9/4 17:47 JST 守门落地）

### 3.1 4 大模块

| 模块 | 入口 | 典型用例 |
|---|---|---|
| `cats_mock::data` | 业务对象 factory | User / Project / Task / AuditEvent |
| `cats_mock::db` | DB fixture | PG schema/seed (in-memory + pg stub) |
| `cats_mock::infra` | 基础设施 mock | Kafka / Redis in-memory 替身 |
| `cats_mock::http` | HTTP API mock | actix-web 路由 + 响应工厂 + MockServer |

### 3.2 5 分钟上手（参考 `crates/cats-mock/README.md`）

```rust
use cats_mock::{
    data::{UserFactory, ProjectFactory, TaskFactory, AuditEventFactory, Factory},
    db::{in_memory, SchemaSet, SeedSet},
    infra::{MockKafka, MockRedis},
    http::{MockServer, MockServerConfig},
};

#[tokio::test]
async fn e2e_user_create_login_audit() {
    // 1) 业务数据
    let user = UserFactory::new().with_username("alice").build();
    let project = ProjectFactory::new().owned_by(user.id).build();
    let task = TaskFactory::new().for_project(project.id).build();
    let audit = AuditEventFactory::new().by_user(user.id).of_type("login").build();

    // 2) DB fixture (in-memory)
    let db = in_memory();
    db.apply_schema(SchemaSet::all_common()).await.unwrap();
    db.apply_seed(SeedSet::from_user(&user)).await.unwrap();

    // 3) 基础设施 mock
    let kafka = MockKafka::new();
    let redis = MockRedis::new();

    // 4) HTTP server
    let server = MockServer::start(MockServerConfig::all()).await.unwrap();
    let base = server.base_url();

    // 5) 用 reqwest 跑 e2e
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/v1/auth/login"))
        .json(&serde_json::json!({"username": "alice", "password": "p"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
}
```

### 3.3 16 域 smoke 升级（per commit `2fc3d96`）

原本每个 service 的 `tests/smoke.rs` 16 行 boilerplate：

```rust
// 旧版（16 行 × 16 service = 256 行）
#[test]
fn smoke() {
    assert!(env!("CARGO_PKG_NAME").len() > 0);
    assert!(env!("CARGO_PKG_VERSION").len() > 0);
    let n = env!("CARGO_PKG_NAME");
    let v = env!("CARGO_PKG_VERSION");
    assert!(n.starts_with("auth"));
    assert!(v.starts_with("0."));
    assert!(v.contains("."));
    let _ = format!("{n}-{v}");
    let parts: Vec<&str> = v.split('.').collect();
    assert!(parts.len() >= 2);
    let major: u32 = parts[0].parse().unwrap();
    let minor: u32 = parts[1].parse().unwrap();
    assert!(major < 100);
    assert!(minor < 100);
    let s = format!("{n}@{v}");
    assert!(s.len() > 0);
}
```

现在每 service 的 `tests/smoke.rs` 5 行 boilerplate：

```rust
// 新版（5 行 × 16 service = 80 行）
use cats_mock::smoke::*;
name_matches_crate!(env!("CARGO_PKG_NAME"));
```

### 3.4 mock_data 7 fixtures 引用

`crates/cats-mock/mock_data/` 收纳 7 个 JSON fixture（per 9/4 17:47 JST 守门 + 守门 #11 缺标比错标）：

| 文件 | 数量 | 用途 |
|---|---|---|
| `tenants.json` | 5 个 org | 多租户测试 |
| `users.json` | 10 个用户 | RBAC 测试（Sponsor + 5 域 Lead + 4 User）|
| `projects.json` | 3 个项目 | 项目生命周期 |
| `tasks.json` | 10 个任务 | 翻译任务状态机 |
| `tm_segs.json` | 20 个 TM 段 | pgvector 语义召回 |
| `glossary.json` | 50 个术语 | 术语命中 + enforcement |
| `audit.json` | 5 条审计 | audit-service 事件回放 |

### 3.5 5 个 mock 脚本（`crates/cats-mock/scripts/`）

| 脚本 | 用途 | 触发 |
|---|---|---|
| `smoke-test.sh` | 9 服务 + AI GW + Kafka + PG 端到端 smoke | 每次 commit |
| `regression-test-backend.sh` | worker#2 集成 tests | Sprint 收口 |
| `regression-test-aigw.sh` | worker#4 集成 tests | Sprint 收口 |
| `regression-test-app.sh` | worker#1 `cargo check` | 每次 commit |
| `db-migrate.sh` | sqlx-migrate 全部 16 域 schema | 部署前 |
| `local-up.sh` / `local-down.sh` | 本地一键起停 | 开发日常 |

> 详细见各脚本顶部注释。

---

## 4. 编码规范

### 4.1 Rust 规范

- **Lint**：CI 强制 `cargo clippy --workspace --all-features --locked -- -D warnings`
- **格式**：CI 强制 `cargo fmt --all --check`
- **unsafe**：workspace 强制 `unsafe_code = "forbid"`（per Cargo.toml）
- **注释**：每个 `pub` 项必须有 `///` doc 注释（missing_docs warn，CI `-D warnings` 升 error）
- **错误处理**：用 `thiserror` 定义业务错误，用 `anyhow` 包装上下文；禁止 `unwrap()` 在业务代码（非测试）
- **async**：所有 IO 用 `tokio`，禁止 blocking IO 在 async context

### 4.2 数据库规范

- **迁移**：每个 crate 自有 `migrations/` 目录，命名 `*_<description>.sql` + 对应 `*_down.sql`
- **Schema 命名**：`<domain>.<table>`，例：`auth.users`、`project.terms`
- **ID 类型**：统一 `uuid` v4，列名 `id UUID PRIMARY KEY`
- **时间戳**：统一 `TIMESTAMPTZ DEFAULT NOW()`，列名 `created_at` / `updated_at`
- **JSONB**：灵活字段用 JSONB（per 数据库设计书 v2.0），必须有 GIN 索引
- **向量**：pgvector 维度固定（768 / 1024 / 1536），列名 `embedding vector(N)`

### 4.3 API 规范

- **REST**：URL 路径版本化 `/v1/...`，破坏性变更发 `/v2/...`
- **gRPC**：package 版本化 `cats.<domain>.v1`
- **Kafka Event**：JSON Schema + `schema_version` 字段
- **错误信封**：统一格式（per 接口设计书 §1.3）
- **幂等**：所有写操作支持 `Idempotency-Key` 请求头

### 4.4 Git 提交规范

```
<type>(<scope>): <subject>

<body>

<footer>
```

**type**：`feat` / `fix` / `docs` / `test` / `refactor` / `perf` / `chore`

**scope**：`auth` / `user` / `project` / `task` / `file` / `notify` / `report` / `audit` / `worker` / `translation-core` / `mock` / `infra` / `docs` / `release`

**示例**：

```
feat(auth): 添加 JWT refresh token 端点

- POST /v1/auth/refresh
- 接受 refresh_token (30 天有效)
- 返回新 access_token (24h) + 新 refresh_token
- 旧 refresh_token 立即失效

per 接口设计书 v2.0+2 §3.1 + ADR-008 JWT 密钥轮换与刷新
```

---

## 5. CI/CD 流程

### 5.1 GitHub Actions / Gitea Actions（per `ci/github-actions/ci-rust-test.yaml`）

```
PR 触发流程:
1. cargo fmt --all --check            # 格式
2. cargo clippy --workspace -D warnings  # lint
3. cargo check --workspace --all-features --locked  # 编译（M0 阶段）
4. cargo test -p cats-mock --all-features  # cats-mock 自测（per 2fc3d96）
5. cargo llvm-cov --workspace --lcov --fail-under-lines 40  # 覆盖率（>= 40%）
6. ./crates/cats-mock/scripts/smoke-test.sh  # 端到端 smoke
7. buf breaking proto/cats-proto/  # gRPC 兼容性
8. openapi-diff openapi/v1.0.1.yaml openapi/v1.0.2-rc.yaml  # OpenAPI 兼容性
9. 6 角色 DDD Review 标记（人工）
```

### 5.2 镜像构建 + 推送

```
main 分支:
1. 触发 CI（通过）
2. 构建镜像: docker build -t harbor.cats.internal/cats/<service>:${COMMIT_SHA} .
3. 推送: docker push harbor.cats.internal/cats/<service>:${COMMIT_SHA}
4. Argo CD 检测新镜像 → 自动同步到 cats-core namespace
```

### 5.3 部署

```
Argo CD:
- ApplicationSet: cats-edge / cats-core / cats-data / cats-media / cats-infra
- 同步策略: auto-sync + prune + self-heal
- 回滚: argocd app rollback cats-core --revision <commit>
```

---

## 6. 调试技巧

### 6.1 单服务本地调试

```bash
# 1. 启动依赖（PG / MinIO / Valkey）
docker compose -f crates/<service>/docker-compose.dev.yml up -d

# 2. 跑迁移
./crates/cats-mock/scripts/db-migrate.sh <service>

# 3. 启动服务（带 debug 日志）
RUST_LOG=debug cargo run -p <service>

# 4. 在 VSCode 启动调试（launch.json 已配置）
#    F5 启动，attach 到 running process
```

### 6.2 集成测试调试

```bash
# 1. 跑单个集成 test（带 println 输出）
cargo test -p auth-service --test integration -- --nocapture test_login_success

# 2. 跑全部集成 tests
cargo test -p auth-service --test integration --all-features

# 3. 集成测试自动启 in-memory mock（无需 docker）
#    详细见 cats-mock/README.md
```

### 6.3 链路追踪（OpenTelemetry，阶段二）

```bash
# 1. 启动 OTel Collector
docker compose -f deploy/otel/docker-compose.yml up -d

# 2. 服务自动上报 trace 到 OTel → Tempo

# 3. 在 Grafana 查询 trace_id
#    Tempo datasource → Search → trace_id=4bf92f3577b34da6a3ce929d0e0e4736
```

---

## 7. 已知缺口（v1.0 MVP 开发）

| # | 缺口 | 临时方案 | GA 时间 |
|---|---|---|---|
| D-1 | 16 域 service crate 集成 RBAC 中间件未全量跑通 | 部分 service 临时用 `User` 角色硬编码 | Sprint 2 W3-W4 (9/14-9/27) |
| D-2 | 客户端 iOS/Android/Unity SDK 仅有 stub | 客户需等 | v1.1（Q4 2026）|
| D-3 | Tauri 客户端 UI 与服务端 mock 集成测试覆盖率 < 40% | 阶段二补 | Sprint 3 |
| D-4 | gRPC `tonic-build` 0.13 编译慢（首次 ~ 60s）| 增量编译 + `sccache` 缓存 | 持续优化 |
| D-5 | sqlx-cli 离线模式需手动 prepare | 跑前 `cargo sqlx prepare` | v1.1 改用 sea-orm |

---

## 8. 关联文档（git 实证）

| 引用文档 | 路径 | commit hash | 用途 |
|---|---|---|---|
| 微服务架构设计书 v1.0 | `doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md` | `2910f3d` | §4 9 服务 + §14 阶段一/二 |
| 接口设计书 v2.0+2 | `doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md` | `1f3c94d` | §1-§3 通用约定 + 9 服务 API |
| 模块设计书 v2.2 | `doc/03-详细设计/模块设计/CATs_模块设计书_v2.0.md` | `f8ac021` | §4 模块边界 + §5 翻译管道 |
| 数据库设计书 v2.0 | `doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md` | — | 16 域 schema + pgvector |
| Rust 技术选型书 v1.0 | `doc/02-基础设计/技术选型/CATs_Rust技术选型书_v1.0.md` | — | §4-§11 选型 |
| 技术基线 v1.0 | `doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md` | `d26f81f` | Rust 1.98 + PG 18.6 + pgvector 0.8.6 |
| cats-rbac README | `crates/cats-rbac/README.md` | `f417407` | RBAC 中间件 |
| 测试 Mock 项目设计书 v1.0 | `doc/02-基础设计/测试/CATs_测试Mock项目设计书_v1.0.md` | `2fc3d96` | §6 cats-mock 4 大模块 |
| ADR 集合 | `doc/02-基础设计/决策/CATs_ADR-*.md` | — | 10 个 ADR 决策记录 |
| cats-mock README | `crates/cats-mock/README.md` | `2fc3d96` | 5 分钟上手 |
| CI/CD 设计 v1.0 | `doc/05-其他/管理/CATs_CI_CD流水线设计_v1.0.md` | — | §5 CI 流程 |

---

## 9. 修订历史

| 版本 | 日期 | 修订人 | 修订内容 | 触发 |
|---|---|---|---|---|
| v1.0 | 2026-09-19 | 架构师(Mavis 接手 agent per DEC-008) | 初版：monorepo 结构 + 9 服务 owners + cats-mock 4 大模块 + 16 域 smoke 升级 + 5 个 mock 脚本 + 7 个 mock_data fixture 引用 + Rust / DB / API / Git 4 套编码规范 + CI/CD 3 阶段 + 5 已知缺口 + 11 条引用 git 实证 | V1.1-PLAN §2.1 Sprint 2 MVP 商业版落地 |