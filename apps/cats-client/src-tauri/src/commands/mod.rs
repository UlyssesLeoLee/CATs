//! Tauri #\[command\] 绑定（暴露给 Svelte 前端调用）
//!
//! MVP 命令清单（per 任务 Sprint 2 W3-W4 起点）:
//! - `get_offline_status`       — 客户端离线状态 + 队列长度
//! - `auth_login`               — 调 /v1/auth/login
//! - `auth_refresh`             — 强制 refresh（自动 refresh 在 ApiClient 内）
//! - `fetch_translation_lookup` — 调 /v1/translate/lookup 查 TM
//! - `list_projects`            — 调 GET /v1/projects
//! - `create_project`           — 调 POST /v1/projects
//! - `enqueue_offline_action`   — 离线模式把请求写本地队列
//! - `sync_offline_queue`       — 联网后批量同步队列
//!
//! 已知缺口（per apps/cats-client/TODO.md）:
//! - 大段数据传输（如文档上传 / TM 导入）未实现

pub mod auth_cmd;
pub mod offline_cmd;
pub mod project_cmd;
pub mod translate_cmd;

pub use auth_cmd::{auth_login, auth_refresh};
pub use offline_cmd::{enqueue_offline_action, get_offline_status, sync_offline_queue};
pub use project_cmd::{create_project, list_projects};
pub use translate_cmd::fetch_translation_lookup;