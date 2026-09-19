//! `cats-client` 入口
//!
//! 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1（客户端：Tauri 2.x + Svelte 5）
//! 引用: doc/02-基础设计/技术选型/CATs_技术选型书_v2.0.md ADR-004
//!
//! MVP 阶段（Sprint 2 W3-W4 起点）:
//! - 启动 Tauri 应用, 装载前端 dist/
//! - 暴露 #\[tauri::command\] 给 Svelte 调用本地能力:
//!   - `auth_login` / `auth_refresh` — 调 auth-service (via BFF) JWT 注入
//!   - `fetch_translation_lookup` — 调 BFF /v1/translate/lookup 查 TM 候选
//!   - `list_projects` / `create_project` — 调 BFF 项目列表 / 创建
//!   - `enqueue_offline_action` — 离线模式把请求写本地 SQLite 队列
//!   - `sync_offline_queue` — 联网后批量同步队列
//!   - `get_offline_status` — 返回当前在线/离线状态 + 队列长度
//!
//! 已知缺口（per apps/cats-client/TODO.md）:
//! - OAuth/SSO 完整集成未实现（仅 JWT + refresh）
//! - 真实 TM 写入未实现（只读 lookup, 写入走 BFF → translation-core）

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use cats_client::run;
use tracing::error;

fn main() {
    cats_client::init_tracing();
    if let Err(e) = run() {
        error!(error = %e, "cats-client 启动失败");
        std::process::exit(1);
    }
}