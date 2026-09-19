//! `cats-client` lib 入口
//!
//! 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1（客户端：Tauri 2.x + Svelte 5）
//! 引用: doc/02-基础设计/接口设计/CATs_接口设计书_v2.0.md §1.3（统一错误信封）
//!
//! MVP（Sprint 2 W3-W4 起点）模块划分:
//! - `api`        — HTTP client 包装（reqwest）+ JWT 注入 + 错误信封解析
//! - `offline`    — SQLite 本地缓存 + 离线队列（schema 详见 offline/schema.sql）
//! - `commands`   — Tauri #\[command\] 绑定（暴露给 Svelte 前端）
//! - `state`      — 应用全局状态（在线/离线 + token + DB 连接）
//!
//! 已知缺口（per apps/cats-client/TODO.md）:
//! - OAuth/SSO 完整集成未实现
//! - 真实 TM 写入走 BFF → translation-core（M1 阶段落地）

pub mod api;
pub mod commands;
pub mod offline;
pub mod state;

use std::sync::Arc;
use tauri::Manager;
use tracing::info;

/// 初始化 tracing subscriber（M0 占位实现，统一从 cats-common 风格）
pub fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,cats_client=debug"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}

/// 启动 Tauri 应用入口
///
/// # Errors
///
/// 当 Tauri builder 启动失败时返回错误（如窗口创建失败、capability 不合法）。
pub fn run() -> anyhow::Result<()> {
    init_tracing();
    info!("starting cats-client tauri app");

    let app_state = Arc::new(state::AppState::new()?);

    tauri::Builder::default()
        .manage(app_state)
        .setup(|app| {
            // 启动时尝试打开 SQLite 离线数据库（路径走 app_local_data_dir）
            let handle = app.handle().clone();
            let state: tauri::State<Arc<state::AppState>> = handle.state();
            if let Err(e) = state.ensure_db(&handle) {
                tracing::warn!(error = %e, "离线数据库初始化失败，将以纯在线模式启动");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_offline_status,
            commands::auth_login,
            commands::auth_refresh,
            commands::fetch_translation_lookup,
            commands::list_projects,
            commands::create_project,
            commands::enqueue_offline_action,
            commands::sync_offline_queue,
        ])
        .run(tauri::generate_context!())
        .map_err(Into::into)
}