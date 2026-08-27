# CATs

**AI 化的计算机辅助翻译（CAT）软件 —— 全媒体 AI 辅助翻译 SaaS 平台**

> **文档编号**：BASE-README
> **版本**：v1.1（基线升级：Rust 1.98.0 + PostgreSQL 18.6 + pgvector 0.8.6）
> **创建日**：2026-08-17
> **更新**：v1.0 → v1.1（2026-08-26 锁定 Rust 1.98.0 + PG 18.6 + pgvector 0.8.6，见 [`CATs_技术基线_v1.0`](./doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md)）
> **作者**：架构师 + Rust Lead + DBA（worker 代签 per DEC-008）

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|---|---|---|---|
| 1.0 | 2026-08-17 | 架构师 | 初版。CATs 项目主入口：特性 / 文档导航 / 技术栈概览 |
| **1.1** | **2026-08-26** | **架构师 + Rust Lead + DBA（worker 代签 per DEC-008）** | **基线升级：技术栈引用统一到 `CATs_技术基线_v1.0`（Rust 1.98.0 + PostgreSQL 18.6 + pgvector 0.8.6）** |

---

CATs 把传统 CAT 工具「复制 / 粘贴 / 导入 / 导出 / 上传 / 下载」的多步流程，压缩为
「看到 → 选中 → 翻译 → 确认」的极简操作，并以 AI 驱动术语一致性、翻译记忆复用与质量校验。

## 特性

- **多形态翻译**：网页划词翻译、PDF/文档双语对照、截图 OCR 翻译、字幕/音视频翻译、Jira 等协作工具内容翻译
- **翻译记忆（TM）**：100% / 模糊匹配自动复用，减少重复翻译
- **术语库优先**：术语强制统一，AI 提示词自动注入，译后强制校验
- **标签 / 占位符保护**：富文本标签、`{placeholder}`、复数规则在翻译过程中原样保留或按目标语言规则重排
- **多模型协同**：支持 GPT / Claude / Gemini / DeepSeek 等模型并行调用与评审取优
- **游戏本地化**：Unity / Unreal / Godot 引擎适配器，支持二进制资源抽取、回写与 UI 溢出验证
- **合规与私有化**：AI Gateway 合规开关，支持云端与本地解析两种数据流模式

## 项目文档

完整的需求、架构与模块设计文档见 [`doc/`](./doc) 目录：

```
doc/
├── 01-需求/           # 原始需求与需求规格说明
├── 02-基础设计/        # 总体架构、技术选型、Rust 选型、可热插拔运维设计
├── 03-详细设计/        # 模块设计、游戏本地化模块、接口设计、数据库设计
├── 04-测试/           # 测试设计书（JIS X 0129 / ISO 29119 标准）
└── 05-其他/           # 实施前 QA 登记册、150 任务工程工作流、评审记录
```

详见 [`doc/README.md`](./doc/README.md)。

## 技术栈概览

> 基线版本引用 [`CATs_技术基线_v1.0`](./doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md)（Rust 1.98.0 + PostgreSQL 18.6 + pgvector 0.8.6）

- **客户端**：Tauri（Rust 核心 + Svelte 5 WebView）
- **后台管理**：Next.js
- **后端**：微服务架构（Rust / Python），Kafka 事件驱动，AI 网关统一接入多模型
- **存储**：PostgreSQL 18.6（业务数据 + pgvector 0.8.6 向量检索）、SQLite（客户端本地缓存/离线队列）
- **媒体处理**：ASR、OCR、字幕、办公文档转换、渲染写回等可插拔处理管线

## 代码架构与目录约定（M0 脚手架阶段）

> 本节由 M0 worker 追加（2026-08-27），描述 monorepo 代码布局与 18 crate 拓扑。
> 引用：[`CATs_微服务架构设计书_v1.0` §4.1](./doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md)

### Monorepo 布局

```
.
├── Cargo.toml                # workspace 根（18 crate 成员列表）
├── Cargo.lock                # 锁定 transitive deps
├── rust-toolchain.toml       # Rust 1.98.0
├── .cargo/config.toml        # 共享 cargo 配置
├── .gitignore                # Rust / IDE / 系统 / 密钥
├── .gitattributes            # LF/CRLF 归一化
├── cargo-deny.toml           # 许可证白名单
│
├── crates/                   # 18 个 Rust crate
│   ├── common/               # cats-common：共享类型/错误/日志
│   ├── proto/                # cats-proto：tonic-build 编译 proto/cats/v1/*.proto
│   ├── auth-service/         # MVP 8 核心（auth/user/project/task/file/notification/report/audit/worker）
│   ├── user-service/
│   ├── project-service/
│   ├── task-service/
│   ├── file-service/
│   ├── notification-service/
│   ├── report-service/
│   ├── audit-service/
│   ├── worker-service/
│   ├── translation-core/     # 翻译核心（独立服务，不与 task-service 合并）
│   ├── ingestion-service/    # 阶段二 媒体处理 6 服务
│   ├── asr-service/
│   ├── ocr-service/
│   ├── subtitle-service/
│   ├── office-converter-service/
│   ├── render-writer-service/
│   └── cats-bff/             # BFF 聚合层（客户端 → 微服务桥接）
│
├── apps/
│   └── cats-client/          # Tauri 2.x 客户端（M0 占位，M2 实装）
│
├── deploy/
│   ├── docker/               # Dockerfile.rust（多阶段 distroless）+ Dockerfile.client（M0 占位）
│   ├── helm/                 # 18 chart：cats-common + 17 service chart
│   ├── kustomize/            # 占位（per 任务：可选，不实施）
│   └── argocd/               # ApplicationSet（17 service + 1 bff = 18 deploy unit）
│
├── ci/
│   ├── github-actions/       # 8 份 Gitea Actions / GitHub Actions 模板
│   └── scripts/              # 5 份 PowerShell 开发脚本
│
├── proto/                    # 既有 — 不动
│   └── cats/v1/
│       ├── auth.proto
│       ├── common.proto
│       ├── media.proto
│       └── translation_core.proto
│
├── api/                      # 既有 — 不动
│   └── openapi/
│       └── cats-openapi-v1.yaml
│
└── doc/                      # 既有 — 不动（除本 README.md 追加 1 节）
```

### 18 Crate 拓扑（per 微服务架构书 §4.1）

| 类别 | 数量 | Crate 列表 |
|---|---|---|
| 共享层 | 2 | `cats-common`, `cats-proto` |
| MVP 核心 | 8 | `auth-service`, `user-service`, `project-service`, `task-service`, `file-service`, `notification-service`, `report-service`, `audit-service` |
| 异步执行 | 1 | `worker-service` |
| 翻译核心 | 1 | `translation-core` |
| 阶段二 媒体 | 6 | `ingestion-service`, `asr-service`, `ocr-service`, `subtitle-service`, `office-converter-service`, `render-writer-service` |
| BFF | 1 | `cats-bff` |
| **总计** | **19** | （注：任务说"18 crate"=16 service + cats-bff + cats-common，proto 是额外 lib 共享 crate） |

### Crate 统一约定

- `Cargo.toml`：`name = "cats-<x>"`, `version.workspace = true`, `edition = "2021"`, `rust-version = "1.98.0"`, workspace 共享 deps
- `src/main.rs`：actix-web 4，`GET /healthz` 返回 `200 OK + JSON`，`BIND_ADDR` 走 env
- `src/lib.rs`：暴露 `pub fn version() -> &'static str` + `pub fn name()`
- `tests/smoke.rs`：至少 1 个 test 验证 `version()` / `name()`
- `README.md`：概述 + API 端点 + 数据边界 + 上下游 + 基线引用

### 强约束（M0 阶段不变）

1. **不写业务逻辑**：仅脚手架 + 占位 + 路径
2. **无 `unsafe`**：`workspace.lints.rust.unsafe_code = "forbid"`
3. **不引入新 crate 依赖**：仅 workspace 共享的 tokio / actix-web / anyhow / tracing / serde / sqlx / tonic / tonic-build / prost
4. **不写 SQL / 迁移**：M1 阶段引入 `sqlx migrate`
5. **不写死编码**：所有端口/路径走 env var
6. **proto/api/doc 文件不动**：仅脚手架可写

### 验证

```powershell
# 编译（18 crate + tonic-build 编译 proto）
cargo build --workspace

# 格式
cargo fmt --all -- --check

# 严格 clippy
$env:RUSTFLAGS = '-D warnings'
cargo clippy --workspace --all-features --all-targets
Remove-Item Env:RUSTFLAGS

# 测试
cargo test --workspace --all-features

# 许可证（无需网络）
cargo deny check licenses
```

## 许可证

见 [LICENSE](./LICENSE)。
