-- =====================================================================
-- CATs report_db schema  (per 数据库设计书 v2.0 §4.7)
-- 引用:
--   doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §4.7
--   doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §6 (分区)
-- 角色矩阵: report_db → migrator_report (DDL) / svc_report (CRUD)
-- 备注: usage_daily 按月 RANGE 分区; 当前落地 2026-08/2026-09 两个示例分区,
--       生产滚动由 worker-service 定时任务补 (per §6 末段)
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- usage_daily (按月 RANGE 分区)
CREATE TABLE IF NOT EXISTS usage_daily (
    org_id              UUID NOT NULL,
    project_id          UUID NOT NULL,
    usage_date          DATE NOT NULL,
    media_minutes       NUMERIC(10,2) NOT NULL DEFAULT 0,
    task_count          INT NOT NULL DEFAULT 0,
    PRIMARY KEY (org_id, project_id, usage_date)
) PARTITION BY RANGE (usage_date);

CREATE TABLE IF NOT EXISTS usage_daily_2026_08 PARTITION OF usage_daily
    FOR VALUES FROM ('2026-08-01') TO ('2026-09-01');
CREATE TABLE IF NOT EXISTS usage_daily_2026_09 PARTITION OF usage_daily
    FOR VALUES FROM ('2026-09-01') TO ('2026-10-01');

-- billing_items (计费明细)
CREATE TABLE IF NOT EXISTS billing_items (
    id              BIGSERIAL PRIMARY KEY,
    org_id          UUID NOT NULL,
    period_start    DATE NOT NULL,
    period_end      DATE NOT NULL,
    item_kind       TEXT NOT NULL,
    quantity        NUMERIC(10,2) NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_billing_items_org_period
    ON billing_items (org_id, period_start);

-- qa_stats
CREATE TABLE IF NOT EXISTS qa_stats (
    id              BIGSERIAL PRIMARY KEY,
    project_id      UUID NOT NULL,
    stat_date       DATE NOT NULL,
    qa_pass_count   INT NOT NULL DEFAULT 0,
    qa_block_count  INT NOT NULL DEFAULT 0,
    tm_hit_rate     REAL,
    UNIQUE (project_id, stat_date)
);
