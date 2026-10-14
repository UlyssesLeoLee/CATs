-- CATs notification_db schema (per ULYS-152 切片 B-4 + 微服务架构书 v1.0 §4.1 notification-service)
-- 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1 (notification-service)
-- 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.1 (notification_db 接口契约 v1.0.0)
-- 引用: doc/05-其他/管理/CATs_错误码表_v1.0.md §3 (error enum)
--
-- 设计选择 (per 缺标比错标安全, 守门 #11):
-- - 接口设计书 v2.0 整份不存在 (per Sprint 1 §6.1 缺口)
--   → DDL 基于微服务架构书 §4.1 + 本切片 B-4 任务描述
-- - 8 逻辑库各自独立 schema, 不直连 user_db / auth_db
-- - payload JSONB 留扩展点 (per 业务: 通知类型可自定义结构)
-- - read_at NULL → 未读, NOT NULL → 已读时间戳
-- - 软删除通过 status='deleted' (per 8 逻辑库统一原则)
--
-- 不可逆性: 中 (新表, 8 逻辑库第 6 个)
-- 性能: 主键 + (user_id, created_at DESC) 索引 + (user_id, read_at IS NULL) 部分索引

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS notifications (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL,
    type            TEXT NOT NULL,  -- task_completed / task_failed / project_invited / etc
    title           TEXT NOT NULL,
    body            TEXT NOT NULL,
    payload         JSONB NOT NULL DEFAULT '{}'::jsonb,
    read_at         TIMESTAMPTZ,
    status          TEXT NOT NULL DEFAULT 'active'
                    CHECK (status IN ('active','deleted')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 索引: 按 user_id + 未读优先 + 时间倒序 (per 列表场景, 未读优先)
CREATE INDEX IF NOT EXISTS idx_notifications_user_created
    ON notifications (user_id, created_at DESC)
    WHERE status = 'active';

-- 索引: 按 user_id 找未读 (per 通知中心"未读" tab)
CREATE INDEX IF NOT EXISTS idx_notifications_user_unread
    ON notifications (user_id)
    WHERE read_at IS NULL AND status = 'active';

-- updated_at 自动维护 trigger (per auth_db 0001 模式)
CREATE OR REPLACE FUNCTION trg_set_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS notifications_set_updated_at ON notifications;
CREATE TRIGGER notifications_set_updated_at
    BEFORE UPDATE ON notifications
    FOR EACH ROW
    EXECUTE FUNCTION trg_set_updated_at();