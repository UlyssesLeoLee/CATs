-- CATs notification_db schema (per 数据库设计书 v2.0 §6.7)
-- 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §6.7
--
-- MVP 范围 (per brief §1 notification-service):
-- - notifications 推送目标表 (in-app / email / webhook 三类 channel)
-- - 内部事件消费 Kafka topic cats.notifications.v1 → 写库 + 输出 tracing log (MVP 不接 SMTP/WebSocket)

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE IF NOT EXISTS notifications (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID NOT NULL,
    user_id     UUID NOT NULL,                                 -- 接收者
    channel     TEXT NOT NULL DEFAULT 'in_app',                -- in_app / email / webhook
    event_type  TEXT NOT NULL,                                  -- task.completed / project.created ...
    payload     JSONB NOT NULL,
    status      TEXT NOT NULL DEFAULT 'pending',               -- pending / sent / failed
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    sent_at     TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_notifications_org_id ON notifications (org_id);
CREATE INDEX IF NOT EXISTS idx_notifications_user_id ON notifications (user_id);
CREATE INDEX IF NOT EXISTS idx_notifications_status ON notifications (status);