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

-- 4. 本地术语库 (Work, per ULYS-154 切片 D)
--    后端 BFF 当前不暴露 glossary browse endpoint (translation-core 仅
--    glossary_match), 客户端先把用户新增的术语条目落到本地, 等后端 expose
--    后批量同步 (per ULYS-154 §honest scope).
CREATE TABLE IF NOT EXISTS local_glossary (
    entry_id       TEXT PRIMARY KEY,     -- UUID, 前端生成
    source_term    TEXT NOT NULL,
    target_term    TEXT NOT NULL,
    domain         TEXT,                 -- 自由标签 (产品名 / 品牌 / 法律 / ...)
    notes          TEXT,
    project_id     TEXT,                 -- 可选, NULL = 全局
    created_at     TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (source_term, target_term, project_id)
);

CREATE INDEX IF NOT EXISTS idx_local_glossary_project
    ON local_glossary (project_id, created_at DESC);

-- 5. 任务事件流 (Work, per ULYS-154 切片 D)
--    客户端 POST /v1/tasks dispatch 成功后, 记录 task_id + 后续由客户端手动
--    状态变更 (mark_local_task_status) 或本地 SSE 镜像.
--    真 SSE 由 task-service 暴露, 但 BFF 当前不代理 (M2 范畴).
CREATE TABLE IF NOT EXISTS task_events (
    event_id       INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id        TEXT NOT NULL,
    event_type     TEXT NOT NULL,        -- 'dispatched' | 'progress' | 'status_changed' | 'completed' | 'failed' | 'sse_received'
    payload_json   TEXT,                 -- 事件细节 (progress, status, error 等)
    recorded_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_task_events_task
    ON task_events (task_id, recorded_at ASC);