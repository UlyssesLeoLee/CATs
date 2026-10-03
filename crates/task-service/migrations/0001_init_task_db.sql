-- CATs task_db schema (per 数据库设计书 v2.0 §6.6 + 接口设计 v2.0 §6.4)
-- 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §5.1
-- 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §6.6
-- 引用: doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md §6.4
--
-- MVP 范围 (per §6.4.6 Sprint 1 完成判据):
-- - tasks 主表 (id, project_id, status 状态机, src/tgt lang, media_type)
-- - 状态机: pending → in_progress → completed → qa_blocked (per §6.4.5 + 错误码表 §6 QA_BLOCKED)
-- - task_events_outbox (per §2 Outbox 模式, K3s 阶段二使用)

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE IF NOT EXISTS tasks (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id    UUID NOT NULL,
    org_id        UUID NOT NULL,                          -- 多租户隔离
    title         TEXT NOT NULL,
    description   TEXT,
    status        TEXT NOT NULL DEFAULT 'pending',        -- pending / in_progress / completed / qa_blocked / failed
    media_type    TEXT NOT NULL DEFAULT 'document',       -- document / video / audio / image
    source_lang   TEXT,                                    -- BCP-47
    target_lang   TEXT,
    assigned_to   UUID,                                    -- translator user_id
    qa_violations JSONB DEFAULT '[]'::jsonb,               -- QA 阻断原因 (per §6.4.5 + 错误码表 §6)
    created_by    UUID NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_tasks_project_id ON tasks (project_id);
CREATE INDEX IF NOT EXISTS idx_tasks_org_id ON tasks (org_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks (status);
CREATE INDEX IF NOT EXISTS idx_tasks_assigned_to ON tasks (assigned_to);

CREATE OR REPLACE FUNCTION trg_set_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS tasks_set_updated_at ON tasks;
CREATE TRIGGER tasks_set_updated_at
    BEFORE UPDATE ON tasks
    FOR EACH ROW
    EXECUTE FUNCTION trg_set_updated_at();

-- task_events_outbox (per §2 Outbox, MVP schema 已建, K3s 阶段二 Debezium 启用)
CREATE TABLE IF NOT EXISTS task_events_outbox (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id     UUID NOT NULL,
    event_type  TEXT NOT NULL,                              -- task.created / task.assigned / task.completed ...
    payload     JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    processed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_task_events_unprocessed
    ON task_events_outbox (created_at) WHERE processed_at IS NULL;