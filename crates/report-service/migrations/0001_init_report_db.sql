-- CATs report_db schema (per 数据库设计书 v2.0 §6.8)
-- 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §6.8
--
-- MVP: report-service 通过只读账号 svc_report_ro 跨库查询 task_db.tasks + task_db.task_media_items
-- 本地 report_db 仅存缓存表 (per §15.3 迁移策略, K3s 阶段二启用)

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE IF NOT EXISTS report_cache (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID NOT NULL,
    report_key  TEXT NOT NULL,
    payload     JSONB NOT NULL,
    computed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at  TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_report_cache_org_id ON report_cache (org_id);
CREATE INDEX IF NOT EXISTS idx_report_cache_org_key ON report_cache (org_id, report_key);