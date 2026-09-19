-- cats-client 离线模式 schema (per DB 三分类横展开原则 9/1 18:30 JST)
--
-- 三张表分门别类:
-- 1. tm_cache       — Master, slowly changing (TM 候选只读缓存)
-- 2. tasks          — Work, session-bound (未完成任务)
-- 3. outbox_queue   — Transaction, append-only (离线操作队列)

PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

-- 1. TM 候选缓存 (Master)
CREATE TABLE IF NOT EXISTS tm_cache (
    tm_id          TEXT PRIMARY KEY,
    project_id     TEXT NOT NULL,
    source_text    TEXT NOT NULL,
    target_text    TEXT NOT NULL,
    similarity     REAL NOT NULL,
    is_exact       INTEGER NOT NULL DEFAULT 0,
    cached_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_tm_cache_project
    ON tm_cache (project_id, cached_at DESC);

-- 2. 未完成任务 (Work, session-bound)
CREATE TABLE IF NOT EXISTS tasks (
    task_id        TEXT PRIMARY KEY,
    project_id     TEXT NOT NULL,
    status         TEXT NOT NULL DEFAULT 'pending',
    source_text    TEXT NOT NULL,
    target_text    TEXT,
    updated_at     TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_tasks_project_status
    ON tasks (project_id, status);

-- 3. 离线操作队列 (Transaction, append-only)
CREATE TABLE IF NOT EXISTS outbox_queue (
    seq            INTEGER PRIMARY KEY AUTOINCREMENT,
    action_type    TEXT NOT NULL,        -- 'create_project' | 'auth_login' | ...
    payload_json   TEXT NOT NULL,
    enqueued_at    TEXT NOT NULL DEFAULT (datetime('now')),
    status         TEXT NOT NULL DEFAULT 'pending',  -- 'pending' | 'syncing' | 'done' | 'failed'
    last_error     TEXT
);

CREATE INDEX IF NOT EXISTS idx_outbox_status
    ON outbox_queue (status, seq);