-- =====================================================================
-- CATs notification_db schema  (per 数据库设计书 v2.0 §4.6)
-- 引用:
--   doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §4.6
-- 角色矩阵: notification_db → migrator_notify (DDL) / svc_notify (CRUD)
-- 备注: 事件终点消费方, 不建 Outbox/CDC 通道 (per §8.1)
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS notifications (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL,
    type        TEXT NOT NULL,                          -- 'task.completed' 等
    title       TEXT NOT NULL,
    body        TEXT,
    read_at     TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_notifications_user_unread
    ON notifications (user_id, created_at DESC)
    WHERE read_at IS NULL;

CREATE TABLE IF NOT EXISTS notification_prefs (
    user_id         UUID PRIMARY KEY,
    email_enabled   BOOLEAN NOT NULL DEFAULT true,
    ws_enabled      BOOLEAN NOT NULL DEFAULT true,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE OR REPLACE FUNCTION trg_set_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS notification_prefs_set_updated_at ON notification_prefs;
CREATE TRIGGER notification_prefs_set_updated_at
    BEFORE UPDATE ON notification_prefs
    FOR EACH ROW EXECUTE FUNCTION trg_set_updated_at();
