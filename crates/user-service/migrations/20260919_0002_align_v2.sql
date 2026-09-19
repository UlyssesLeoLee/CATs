-- =====================================================================
-- CATs user_db schema v2.0 alignment  (per 数据库设计书 v2.0 §4.2)
-- 引用:
--   doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §4.2
--   doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §5 (索引)
-- 角色矩阵: user_db → migrator_user (DDL) / svc_user (CRUD)
-- 背景: 0001_init_user_db.sql (T-02 脚手架) 落地了 user_profile.
--       本文件 (0002) 补上 v2.0 §4.2 其余三张表: orgs / org_members /
--       subscriptions / outbox_event.
-- =====================================================================

-- orgs
CREATE TABLE IF NOT EXISTS orgs (
    id                              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                            TEXT NOT NULL,
    plan                            TEXT NOT NULL DEFAULT 'free'
                                        CHECK (plan IN ('free','team','enterprise')),
    seats_limit                     INT NOT NULL DEFAULT 5,
    monthly_media_minutes_quota     INT NOT NULL DEFAULT 60,
    created_at                      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- users_profile (id 重定义: 与 auth_db.users_credential.id 同值, 应用层保证一致)
-- 0001 已有 user_profile (字段 id UUID + user_id ...). 此处补 v2.0 字段:
--   - avatar_url 已有
--   - locale / updated_at 已有
DO $$
BEGIN
    -- 若 user_profile 表已存在但缺 updated_at trigger, 加 trigger
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'user_profile')
       AND NOT EXISTS (
           SELECT 1 FROM pg_trigger WHERE tgname = 'user_profile_set_updated_at'
       ) THEN
        CREATE OR REPLACE FUNCTION trg_set_updated_at() RETURNS trigger AS $$
        BEGIN
            NEW.updated_at = now();
            RETURN NEW;
        END;
        $$ LANGUAGE plpgsql;

        CREATE TRIGGER user_profile_set_updated_at
            BEFORE UPDATE ON user_profile
            FOR EACH ROW EXECUTE FUNCTION trg_set_updated_at();
    END IF;
END $$;

-- org_members
CREATE TABLE IF NOT EXISTS org_members (
    org_id      UUID NOT NULL REFERENCES orgs(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL,
    role        TEXT NOT NULL DEFAULT 'member',
    invited_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    joined_at   TIMESTAMPTZ,
    PRIMARY KEY (org_id, user_id)
);

-- subscriptions
CREATE TABLE IF NOT EXISTS subscriptions (
    org_id                          UUID PRIMARY KEY REFERENCES orgs(id) ON DELETE CASCADE,
    plan                            TEXT NOT NULL,
    seats_used                      INT NOT NULL DEFAULT 0,
    monthly_media_minutes_used      NUMERIC(10,2) NOT NULL DEFAULT 0,
    renews_at                       TIMESTAMPTZ NOT NULL,
    updated_at                      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- outbox_event (per 架构设计书 §7.2 模式, 各库一份)
CREATE TABLE IF NOT EXISTS outbox_event (
    id              BIGSERIAL PRIMARY KEY,
    event_id        UUID NOT NULL DEFAULT gen_random_uuid(),
    aggregate_type  TEXT NOT NULL,
    aggregate_id    TEXT NOT NULL,
    event_type      TEXT NOT NULL,
    payload         JSONB NOT NULL,
    schema_version  INT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_user_db_outbox_created_at ON outbox_event (created_at);
