# cats-client（Tauri 2.x 占位）

> CATs 桌面客户端（Rust 核心 + Svelte 5 UI）

| 项目 | 内容 |
|---|---|
| 框架 | Tauri 2.x |
| UI | Svelte 5 |
| 运行时 | Tauri 2 + WebView2 / WKWebView |
| 阶段 | M0 占位 → M2 阶段实装 |

## M0 阶段状态

**本目录在 M0 阶段是占位骨架**：不写 `Cargo.toml`、不写 `package.json`、不写 `src-tauri/`。

仅放 `README.md` + `src-tauri/.gitkeep` + `ui/.gitkeep` 占位，遵循任务"不引入新 crate 依赖"约束（客户端 M2 阶段再开工）。

## 未来目录（M2 阶段落地）

```
apps/cats-client/
├── src-tauri/         # Rust 端（Tauri 2.x + actix-web 不在内，桌面 app 自带路由）
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       └── main.rs
├── ui/                # Svelte 5 前端
│   ├── package.json
│   ├── svelte.config.js
│   └── src/
│       └── App.svelte
└── README.md
```

## 客户端职责（M2 阶段）

1. 文档导入（拖拽 / 文件选择）
2. 实时显示任务进度（通过 `cats-bff` 订阅 SSE / WebSocket）
3. 段落级编辑与译后审校
4. TM/术语条目浏览与新增
5. 离线缓存（最近 N 个任务的 segment 缓存）

## 上下游服务

- **上游（被调用）**：无（终端用户应用）
- **下游（主动调用）**：`cats-bff`（HTTP / WebSocket，per 架构书 §4.1）

## 引用基线文档

- [CATs_技术基线_v1.0 §1（客户端：Tauri 2.x + Svelte 5）](../../doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md)
- [CATs_技术选型书_v2.0 §2](../../doc/02-基础设计/技术选型/CATs_技术选型书_v2.0.md)
- [CATs_Rust技术选型书_v1.0 §5.5](../../doc/02-基础设计/技术选型/CATs_Rust技术选型书_v1.0.md)
