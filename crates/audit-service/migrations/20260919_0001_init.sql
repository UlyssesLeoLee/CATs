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

-- ⚠ 分区已移除，原因见下（这是一次**显式降级**，不是疏漏）
--
-- 本文件原本按 数据库设计书 v2.0 §4.8/§7 写成
-- `PARTITION BY RANGE (occurred_at)` + 按月子分区。那份 schema **从未在任何
-- 空库上成功应用过**，它有两处硬伤：
--
--   1. 分区表的主键/唯一约束必须包含全部分区键列。`id BIGSERIAL PRIMARY KEY`
--      和 `event_id UUID NOT NULL UNIQUE` 都会被 PostgreSQL 直接拒绝：
--        ERROR:  unique constraint on partitioned table must include all
--                partitioning columns
--
--   2. 修成 `UNIQUE (event_id, occurred_at)` 之后，生产代码
--      （src/db.rs:39 与 src/consumer.rs:103）里的
--          ON CONFLICT (event_id) DO UPDATE SET ingested_at = now()
--      会运行期报错：
--        ERROR:  there is no unique or exclusion constraint matching the
--                ON CONFLICT specification
--      这段 ON CONFLICT 是 Kafka 至少一次投递下的幂等键，丢掉它等于允许
--      重复审计记录——那是合规问题，不是性能问题。
--
-- 两条都实测过（见 deploy/COMPOSE_UP_DEFECTS_v1.0.md）。**没有"只改 schema
-- 又保留分区"的办法**：PostgreSQL 不支持分区表上的跨分区全局唯一约束。
--
-- 因此这里选择去掉分区、保住生产代码依赖的幂等语义。要恢复按月分区，
-- 需要配套改造（届时是一条独立的、需要拍板的变更）：
--   a) 拆一张非分区的 audit_event_ids(event_id UUID PRIMARY KEY) 去重表，
--      插入顺序改为"先去重表再入分区表"；或
--   b) 把 ON CONFLICT 改成 (event_id, occurred_at)——但这会把幂等语义
--      弱化为"同一事件且同一时间戳"，重复投递若时间戳有偏差就会漏判。
--
-- 另注：原注释写"滚动由 worker-service 维护"，但 worker-service 里搜
-- partition / audit_logs 是 0 匹配——那个维护任务并不存在。即使分区表建得
-- 起来，也不会有任何东西去建下一个月的分区。
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
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_org_occurred
    ON audit_logs (org_id, occurred_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_resource
    ON audit_logs (resource_type, resource_id);
