-- CATs user_db schema (per Sprint 1 拆解 v1.0+1 §2 T-02 + 微服务架构书 v1.0 §5.1)
-- 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §5.1 (8 逻辑库)
-- 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.1 (user_db 接口契约 v1.0.0)
-- 引用: doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0+1.md §2 T-02
--
-- 设计选择 (per 缺标比错标安全):
-- - 接口设计书 v2.0 整份不存在 (per Sprint 1 §6.1 缺口)
--   → 本表 schema 基于微服务架构书 §4.1 + Baseline §5.1 端点清单
--   → 详细 request/response schema 留 T-07 启动时升接口设计书 v2.0
-- - 与 auth_db.users_credential 分离: 认证凭据 (auth_db) vs 用户画像 (user_db)
--   → user_profile.user_id 是 UUID, 但**不**做 FK 到 auth_db.users_credential
--   → 理由: 跨服务不直连, 8 逻辑库边界清晰; user_id 一致性由调用方保证
-- - email 字段 nullable, 唯一索引 (允许匿名用户)
-- - display_name 是业务展示字段, 与 auth_db.username 解耦
--
-- 不可逆性: 中 (新表, 8 逻辑库第 2 个)
-- 性能: 主键 + 唯一索引, 无 Seq Scan on > 1k 行表

CREATE TABLE IF NOT EXISTS user_profile (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL UNIQUE,                  -- 关联 auth_db.users_credential.id, 业务一致
    display_name    TEXT NOT NULL,
    email           TEXT,                                  -- 允许匿名 (无 email)
    avatar_url      TEXT,
    locale          TEXT NOT NULL DEFAULT 'ja-JP',         -- i18n 默认值
    timezone        TEXT NOT NULL DEFAULT 'Asia/Tokyo',
    is_active       BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 唯一索引: email 唯一 (允许 NULL 重复)
CREATE UNIQUE INDEX IF NOT EXISTS idx_user_profile_email_unique
    ON user_profile (email)
    WHERE email IS NOT NULL;

-- 索引: 按 is_active 过滤活跃用户
CREATE INDEX IF NOT EXISTS idx_user_profile_is_active
    ON user_profile (is_active) WHERE is_active = true;

-- updated_at 自动维护 trigger (per auth_db 0001 模式)
CREATE OR REPLACE FUNCTION trg_set_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS user_profile_set_updated_at ON user_profile;
CREATE TRIGGER user_profile_set_updated_at
    BEFORE UPDATE ON user_profile
    FOR EACH ROW
    EXECUTE FUNCTION trg_set_updated_at();
