# cats-client（Tauri 2.x + Svelte 5）

> CATs 桌面客户端（Rust 核心 + Svelte 5 UI）

| 项目 | 内容 |
|---|---|
| 框架 | Tauri 2.x |
| UI | Svelte 5 |
| 运行时 | Tauri 2 + WebView2 / WKWebView |
| 阶段 | M1 业务壳（M2 深化中，per ULYS-154 切片 D） |

## 当前阶段

**M1 业务壳**已落地（per 286ed8a + ULYS-149/150/151/152/153 切片 A/B/C 落地后）：

- 5 路由：`#/login` `#/projects` `#/translate` `#/tasks` `#/tm` `#/glossary`
- 8+ Tauri command 绑定（含切片 D 新增 6 个）
- 离线 SQLite 本地缓存 + outbox 队列
- Web / Tauri 双模式运行时

## M2 切片 D（ULYS-154）已实装

| 页面 / 功能 | 状态 | 后端依赖 |
|---|---|---|
| `TranslatePage` 派发为翻译任务按钮 | ✅ 实装 | `POST /v1/tasks`（BFF 已有） |
| `TranslatePage` 标签保护 (F5) UI | ✅ 静态检测 | 客户端 FNV hash, 无后端依赖 |
| `TasksPage` 任务列表 + 事件流 | ✅ 实装 | `POST /v1/tasks` dispatch 落地后本地 SQLite 镜像 |
| `TasksPage` 手动 markStatus | ✅ 实装 | 客户端镜像, SSE 待 BFF 升级 |
| `TmPage` 100% / 模糊分桶可视化 | ✅ 实装 | `GET /v1/translate/lookup`（已有） |
| `GlossaryPage` 术语浏览 + 新增 | ✅ 实装 (本地) | BFF 未暴露 browse endpoint，客户端先落本地 SQLite |

**M2 范畴遗留**：

- BFF 代理 `GET /v1/tasks` + `GET /v1/tasks/{id}/events` SSE 推送 — 切片 D 客户端用本地 SQLite 镜像 + 5 秒轮询过渡
- BFF / translation-core 暴露 `glossary browse` + `tm browse` endpoint — 切片 D 客户端先落本地 SQLite
- 真 TM 写回（POST /v1/tm via BFF → translation-core.tm_update）

## 目录结构

```
apps/cats-client/
├── src/                       # Svelte 5 前端
│   ├── App.svelte             # hash 路由分发 (6 routes)
│   ├── main.ts
│   ├── lib/
│   │   ├── api.ts             # Tauri invoke / web mock 分支
│   │   └── router.svelte.ts   # $state route 状态
│   └── routes/
│       ├── LoginPage.svelte
│       ├── ProjectsPage.svelte
│       ├── TranslatePage.svelte   # 切片 D 加了派发 + 标签保护
│       ├── TasksPage.svelte       # 切片 D 新增
│       ├── TmPage.svelte          # 切片 D 新增
│       └── GlossaryPage.svelte    # 切片 D 新增
├── src-tauri/                 # Rust 端（Tauri 2.x）
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs
│       ├── lib.rs                 # 注册 commands（含切片 D 6 个）
│       ├── state.rs               # AppState (HTTP + token + offline_db)
│       ├── api/                   # BFF 客户端 + 错误信封
│       │   ├── auth.rs
│       │   ├── client.rs
│       │   ├── projects.rs
│       │   ├── tasks.rs           # 切片 D 新增
│       │   ├── translate.rs
│       │   ├── error.rs
│       │   └── mod.rs
│       ├── commands/              # Tauri #\[command\] 绑定
│       │   ├── auth_cmd.rs
│       │   ├── offline_cmd.rs
│       │   ├── project_cmd.rs
│       │   ├── task_cmd.rs        # 切片 D 新增 (6 commands)
│       │   ├── translate_cmd.rs
│       │   └── mod.rs
│       └── offline/               # SQLite 本地缓存 + 队列
│           ├── schema.sql         # 5 张表 (per 切片 D)
│           ├── db.rs              # 切片 D 加 7 个方法
│           ├── schema.rs
│           └── mod.rs
├── index.html
├── package.json
├── svelte.config.js            # Svelte 5 runes + vitePreprocess
├── tsconfig.json
├── vite.config.ts
├── ui/                         # (M0 阶段占位, 已无内容)
└── README.md                   # 本文件
```

## 客户端职责

1. 文档导入（拖拽 / 文件选择）— M2 范畴
2. **任务追踪 (切片 D)**: 通过 `POST /v1/tasks` dispatch, 本地 SQLite 镜像, 5 秒轮询
3. **段落级翻译编辑 + 写回 (切片 D)**: 通过 `fetchTranslationLookup` + 标签保护提示
4. **TM / 术语浏览与新增 (切片 D)**: 本地 SQLite 先落地, 后端升级后批量同步
5. **离线缓存**: SQLite 三表 + 切片 D 加的两张 (local_glossary / task_events)

## 上下游服务

- **上游（被调用）**：无（终端用户应用）
- **下游（主动调用）**：
  - `cats-bff`（HTTP / SSE, per 架构书 §4.1）
  - `cats-ai-gateway`（间接, 通过 translation-core）

## 引用基线文档

- [CATs_技术基线_v1.0 §1（客户端：Tauri 2.x + Svelte 5）](../../doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md)
- [CATs_技术选型书_v2.0 §2](../../doc/02-基础设计/技术选型/CATs_技术选型书_v2.0.md)
- [CATs_Rust技术选型书_v1.0 §5.5](../../doc/02-基础设计/技术选型/CATs_Rust技术选型书_v1.0.md)
- [CATs_接口设计书_v2.0 §1.3](../../doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md)
- [ULYS-154 切片 D spec](../../doc/05-其他/MVP商业版/_slice_d_client.md)
