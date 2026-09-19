//! 离线模式 + SQLite 本地缓存 + 操作队列
//!
//! 设计（MVP 简化）:
//! - 单文件 SQLite（rusqlite + bundled 模式）
//! - 三张表 (per DB 三分类横展开原则 9/1 18:30 JST):
//!   - `tm_cache`         — TM 候选只读缓存（Master 性质，slowly changing）
//!   - `tasks`            — 未完成任务（Work 性质，session-bound）
//!   - `outbox_queue`     — 离线操作队列（Transaction 性质，append-only）
//!
//! - 启动时如果 BFF 不可达（5s timeout），降级到"离线工作模式"（per task）
//! - 联网后由 sync 模块批量消费 outbox_queue
//!
//! 已知缺口（per apps/cats-client/TODO.md）:
//! - 队列冲突解决（服务端 idempotency）简化版：仅 append + 一次重试
//! - DB 加密未启用（SQLite SEE / SQLCipher, 待合规评审）

pub mod db;
pub mod schema;

pub use db::{OfflineAction, OfflineDb, OfflineStatus};