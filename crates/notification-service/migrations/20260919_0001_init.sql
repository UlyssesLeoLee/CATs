-- =====================================================================
-- CATs notification_db schema  (per 数据库设计书 v2.0 §4.6)
-- 引用:
--   doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §4.6
-- 角色矩阵: notification_db → migrator_notify (DDL) / svc_notify (CRUD)
-- 备注: 事件终点消费方, 不建 Outbox/CDC 通道 (per §8.1)
--
-- ---------------------------------------------------------------------
-- 已停用 (per 2026-10-03 真实 PG 验证)
--
-- 本文件缺 payload / status / updated_at 三列，而 src/db.rs 的
-- INSERT ... RETURNING 明确读写这三列。权威 schema 是
-- 20260920_0001_init.sql（版本号最大，最后执行）。
--
-- 本文件排在权威版之前且同样 CREATE TABLE IF NOT EXISTS notifications，
-- 会抢先建出缺列的表，导致权威版的 CREATE TABLE 被静默跳过、其索引
-- 引用不存在的列而失败（ERROR: column "status" does not exist）。
-- 即在全新数据库上 schema 初始化无法完成，生产首次部署同样会踩到。
--
-- 处理：不再建表，权威 schema 见 20260920_0001_init.sql。
-- ---------------------------------------------------------------------
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- 故意留空 notifications：权威 schema 见 20260920_0001_init.sql
-- （下方 notification_prefs 与 updated_at trigger 不与权威版冲突，故保留）

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
