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

    // ---- 切片 D: 本地术语库 (GlossaryPage) ----

    /// 新增一条术语 (同 source_term+target_term+project_id 唯一)
    pub fn add_glossary(
        &self,
        entry_id: &str,
        source_term: &str,
        target_term: &str,
        domain: Option<&str>,
        notes: Option<&str>,
        project_id: Option<&str>,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock().expect("offline db mutex poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO local_glossary \
             (entry_id, source_term, target_term, domain, notes, project_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![entry_id, source_term, target_term, domain, notes, project_id],
        )?;
        Ok(())
    }

    /// 列出术语 (可按 project_id 过滤; NULL project_id = 全局)
    pub fn list_glossary(
        &self,
        project_id: Option<&str>,
        limit: i64,
    ) -> anyhow::Result<Vec<LocalGlossaryEntry>> {
        let conn = self.conn.lock().expect("offline db mutex poisoned");
        // 全局 (NULL) 与项目内 同时返回, 按创建时间倒序
        let mut stmt = conn.prepare(
            "SELECT entry_id, source_term, target_term, domain, notes, project_id, created_at \
             FROM local_glossary \
             WHERE (?1 IS NULL) OR (project_id = ?1) OR (project_id IS NULL) \
             ORDER BY created_at DESC LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![project_id, limit], |row| {
                Ok(LocalGlossaryEntry {
                    entry_id: row.get(0)?,
                    source_term: row.get(1)?,
                    target_term: row.get(2)?,
                    domain: row.get(3)?,
                    notes: row.get(4)?,
                    project_id: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    // ---- 切片 D: 本地任务 + 事件流 (TasksPage) ----

    /// 写入或更新本地任务 (dispatch 成功后调用)
    pub fn upsert_local_task(
        &self,
        task_id: &str,
        project_id: &str,
        status: &str,
        source_text: &str,
        target_text: Option<&str>,
        media_type: Option<&str>,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock().expect("offline db mutex poisoned");
        conn.execute(
            "INSERT INTO tasks (task_id, project_id, status, source_text, target_text, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now')) \
             ON CONFLICT(task_id) DO UPDATE SET \
                project_id = excluded.project_id, \
                status = excluded.status, \
                source_text = excluded.source_text, \
                target_text = excluded.target_text, \
                updated_at = datetime('now')",
            params![task_id, project_id, status, source_text, target_text],
        )?;
        // media_type 暂存 task_events (因为 tasks 表 schema 没这列, M1 不改 schema)
        let media_payload = serde_json::json!({ "media_type": media_type }).to_string();
        conn.execute(
            "INSERT INTO task_events (task_id, event_type, payload_json) VALUES (?1, 'media_type_set', ?2)",
            params![task_id, media_payload],
        )?;
        Ok(())
    }

    /// 列出本地任务 (按 updated_at DESC)
    pub fn list_local_tasks(
        &self,
        project_id: Option<&str>,
        limit: i64,
    ) -> anyhow::Result<Vec<LocalTask>> {
        let conn = self.conn.lock().expect("offline db mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT task_id, project_id, status, source_text, target_text, updated_at \
             FROM tasks \
             WHERE (?1 IS NULL) OR (project_id = ?1) \
             ORDER BY updated_at DESC LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![project_id, limit], |row| {
                // 拿最新的 media_type (从 task_events 反查)
                Ok(LocalTask {
                    task_id: row.get(0)?,
                    project_id: row.get(1)?,
                    status: row.get(2)?,
                    source_text: row.get(3)?,
                    target_text: row.get(4)?,
                    media_type: None, // 简化: 前端不强依赖
                    updated_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// 更新本地任务状态 + 记一条 status_changed 事件
    pub fn update_local_task_status(&self, task_id: &str, new_status: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock().expect("offline db mutex poisoned");
        conn.execute(
            "UPDATE tasks SET status = ?1, updated_at = datetime('now') WHERE task_id = ?2",
            params![new_status, task_id],
        )?;
        let payload = serde_json::json!({ "status": new_status }).to_string();
        conn.execute(
            "INSERT INTO task_events (task_id, event_type, payload_json) VALUES (?1, 'status_changed', ?2)",
            params![task_id, payload],
        )?;
        Ok(())
    }

    /// 记录一条任务事件 (dispatched / sse_received 等)
    pub fn record_task_event(
        &self,
        task_id: &str,
        event_type: &str,
        payload_json: Option<&str>,
    ) -> anyhow::Result<i64> {
        let conn = self.conn.lock().expect("offline db mutex poisoned");
        conn.execute(
            "INSERT INTO task_events (task_id, event_type, payload_json) VALUES (?1, ?2, ?3)",
            params![task_id, event_type, payload_json],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 列出一个任务的所有事件 (按时间 ASC)
    pub fn list_task_events(&self, task_id: &str) -> anyhow::Result<Vec<TaskEvent>> {
        let conn = self.conn.lock().expect("offline db mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT event_id, task_id, event_type, payload_json, recorded_at \
             FROM task_events WHERE task_id = ?1 ORDER BY recorded_at ASC, event_id ASC",
        )?;
        let rows = stmt
            .query_map(params![task_id], |row| {
                Ok(TaskEvent {
                    event_id: row.get(0)?,
                    task_id: row.get(1)?,
                    event_type: row.get(2)?,
                    payload_json: row.get(3)?,
                    recorded_at: row.get(4)?,
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

/// 本地术语条目 (per ULYS-154 切片 D §GlossaryPage)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalGlossaryEntry {
    pub entry_id: String,
    pub source_term: String,
    pub target_term: String,
    pub domain: Option<String>,
    pub notes: Option<String>,
    pub project_id: Option<String>,
    pub created_at: String,
}

/// 本地任务 (per ULYS-154 切片 D §TasksPage)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalTask {
    pub task_id: String,
    pub project_id: String,
    pub status: String,
    pub source_text: String,
    pub target_text: Option<String>,
    pub media_type: Option<String>,
    pub updated_at: String,
}

/// 任务事件 (per ULYS-154 切片 D §TasksPage 事件流)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvent {
    pub event_id: i64,
    pub task_id: String,
    pub event_type: String,
    pub payload_json: Option<String>,
    pub recorded_at: String,
}