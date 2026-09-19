//! SQLite 离线数据库句柄（rusqlite bundled）
//!
//! MVP 简化:
//! - 直接持有 `Mutex<rusqlite::Connection>`（每连接非线程安全）
//! - 不分 pool（桌面单进程单连接足够）
//!
//! 已知缺口（per apps/cats-client/TODO.md）:
//! - 加密未启用（SQLite SEE / SQLCipher）
//! - 不支持多进程（实际不需要，桌面单进程）

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::schema::SCHEMA_SQL;

/// 离线数据库
pub struct OfflineDb {
    conn: Mutex<Connection>,
}

/// 离线状态摘要（暴露给前端）
#[derive(Debug, Clone, Serialize)]
pub struct OfflineStatus {
    pub online: bool,
    pub queue_length: i64,
    pub last_sync_at: Option<String>,
    pub db_path: String,
}

/// 离线队列动作（append-only）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineAction {
    pub action_type: String,
    pub payload_json: String,
}

impl OfflineDb {
    /// 打开 / 创建 SQLite 数据库
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        // 应用 schema
        conn.execute_batch(SCHEMA_SQL)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// 写一条离线动作到队列
    pub fn enqueue(&self, action: &OfflineAction) -> anyhow::Result<i64> {
        let conn = self.conn.lock().expect("offline db mutex poisoned");
        conn.execute(
            "INSERT INTO outbox_queue (action_type, payload_json) VALUES (?1, ?2)",
            params![action.action_type, action.payload_json],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 取最多 N 条 pending 动作（按 seq 升序）
    pub fn fetch_pending(&self, limit: i64) -> anyhow::Result<Vec<PendingAction>> {
        let conn = self.conn.lock().expect("offline db mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT seq, action_type, payload_json FROM outbox_queue \
             WHERE status = 'pending' ORDER BY seq ASC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit], |row| {
                Ok(PendingAction {
                    seq: row.get(0)?,
                    action_type: row.get(1)?,
                    payload_json: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// 标记同步结果
    pub fn mark_done(&self, seq: i64) -> anyhow::Result<()> {
        let conn = self.conn.lock().expect("offline db mutex poisoned");
        conn.execute(
            "UPDATE outbox_queue SET status = 'done' WHERE seq = ?1",
            params![seq],
        )?;
        Ok(())
    }

    /// 标记同步失败
    pub fn mark_failed(&self, seq: i64, err: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock().expect("offline db mutex poisoned");
        conn.execute(
            "UPDATE outbox_queue SET status = 'failed', last_error = ?1 WHERE seq = ?2",
            params![err, seq],
        )?;
        Ok(())
    }

    /// 当前离线状态（暴露给前端 Tauri command）
    pub fn status(&self, online: bool, db_path: &str) -> anyhow::Result<OfflineStatus> {
        let conn = self.conn.lock().expect("offline db mutex poisoned");
        let queue_length: i64 = conn.query_row(
            "SELECT COUNT(*) FROM outbox_queue WHERE status = 'pending'",
            [],
            |row| row.get(0),
        )?;
        let last_sync_at: Option<String> = conn
            .query_row(
                "SELECT MAX(enqueued_at) FROM outbox_queue WHERE status = 'done'",
                [],
                |row| row.get(0),
            )
            .ok();
        Ok(OfflineStatus {
            online,
            queue_length,
            last_sync_at,
            db_path: db_path.to_string(),
        })
    }

    /// 缓存一条 TM 候选（覆盖同 tm_id）
    pub fn cache_tm(&self, tm_id: &str, project_id: &str, source: &str, target: &str, sim: f32, is_exact: bool) -> anyhow::Result<()> {
        let conn = self.conn.lock().expect("offline db mutex poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO tm_cache \
             (tm_id, project_id, source_text, target_text, similarity, is_exact, cached_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
            params![tm_id, project_id, source, target, sim, is_exact as i32],
        )?;
        Ok(())
    }

    /// 读项目下的 TM 缓存（fallback 离线查表）
    pub fn read_tm_cache(&self, project_id: &str, limit: i64) -> anyhow::Result<Vec<CachedTm>> {
        let conn = self.conn.lock().expect("offline db mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT tm_id, source_text, target_text, similarity, is_exact \
             FROM tm_cache WHERE project_id = ?1 \
             ORDER BY similarity DESC LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![project_id, limit], |row| {
                Ok(CachedTm {
                    tm_id: row.get(0)?,
                    source_text: row.get(1)?,
                    target_text: row.get(2)?,
                    similarity: row.get(3)?,
                    is_exact: row.get::<_, i32>(4)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

/// 待同步动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAction {
    pub seq: i64,
    pub action_type: String,
    pub payload_json: String,
}

/// 缓存的 TM 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTm {
    pub tm_id: String,
    pub source_text: String,
    pub target_text: String,
    pub similarity: f32,
    pub is_exact: bool,
}