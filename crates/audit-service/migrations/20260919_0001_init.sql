-- =====================================================================
-- CATs audit_db schema  (per 数据库设计书 v2.0 §4.8)
-- 引用:
--   doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §4.8
--   doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §5 (索引 §5.1)
--   doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §7 (保留 180 天在线 + 3 年归档)
-- 角色矩阵: audit_db → migrator_audit (DDL) / svc_audit (CRUD)
-- 备注: 按 occurred_at 月 RANGE 分区, 滚动由 worker-service 维护
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS audit_logs (
    id              BIGSERIAL PRIMARY KEY,
    event_id        UUID NOT NULL UNIQUE,
    org_id          UUID NOT NULL,
    actor_user_id   UUID,
    action          TEXT NOT NULL,
    resource_type   TEXT NOT NULL,
    resource_id     TEXT NOT NULL,
    before_state    JSONB,
    after_state     JSONB,
    ip              INET,
    occurred_at     TIMESTAMPTZ NOT NULL,
    ingested_at     TIMESTAMPTZ NOT NULL DEFAULT now()
) PARTITION BY RANGE (occurred_at);

CREATE TABLE IF NOT EXISTS audit_logs_2026_08 PARTITION OF audit_logs
    FOR VALUES FROM ('2026-08-01') TO ('2026-09-01');
CREATE TABLE IF NOT EXISTS audit_logs_2026_09 PARTITION OF audit_logs
    FOR VALUES FROM ('2026-09-01') TO ('2026-10-01');

CREATE INDEX IF NOT EXISTS idx_audit_logs_org_occurred
    ON audit_logs (org_id, occurred_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_resource
    ON audit_logs (resource_type, resource_id);
