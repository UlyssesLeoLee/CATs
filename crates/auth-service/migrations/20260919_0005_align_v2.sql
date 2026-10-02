-- =====================================================================
-- CATs auth_db schema v2.0 alignment  (per 数据库设计书 v2.0 §4.1)
-- 引用:
--   doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §4.1
--   doc/05-其他/安全/CATs_安全要件定义书_v1.0.md §3 (Argon2id 密码哈希)
-- 角色矩阵: auth_db → migrator_auth (DDL) / svc_auth (CRUD)
-- 背景: 0001~0004 是 Sprint 1 T-01 实战深化落地 (username+is_active schema).
--       本文件 (0005) 把现有 schema 增列对齐到 v2.0 §4.1 (email CITEXT UNIQUE +
--       org_id + status enum + mfa_enabled), 不破坏既有数据.
--       不可逆性: 中 (加列/加索引可逆; DROP username 不可逆, 但 SELECT 仍兼容)
-- =====================================================================

-- 角色扩展 (per v2.0 §4.1)
ALTER TABLE users_credential
    ADD COLUMN IF NOT EXISTS org_id      UUID,
    ADD COLUMN IF NOT EXISTS status      TEXT NOT NULL DEFAULT 'active',
    ADD COLUMN IF NOT EXISTS mfa_enabled BOOLEAN NOT NULL DEFAULT false;

-- status 上的 CHECK (per v2.0 §4.1 'active','locked','disabled')
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'users_credential_status_check'
    ) THEN
        ALTER TABLE users_credential
            ADD CONSTRAINT users_credential_status_check
            CHECK (status IN ('active','locked','disabled'));
    END IF;
END $$;

-- email 列 (per v2.0 §4.1 email CITEXT NOT NULL UNIQUE).
-- 0002 已经加过 email TEXT (nullable).
-- 这里升到 CITEXT NOT NULL, 仅在尚无任何 email 冲突时安全.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'citext') THEN
        CREATE EXTENSION IF NOT EXISTS citext;
    END IF;
END $$;

-- email NOT NULL 化前先填默认占位 (per "缺标比错标" 安全侧)
UPDATE users_credential
   SET email = COALESCE(NULLIF(email, ''), 'legacy+' || replace(id::text, '-', '') || '@cats.local')
 WHERE email IS NULL;
ALTER TABLE users_credential
    ALTER COLUMN email TYPE CITEXT USING email::CITEXT,
    ALTER COLUMN email SET NOT NULL;

-- 唯一索引 (v2.0 要求 email UNIQUE)
CREATE UNIQUE INDEX IF NOT EXISTS uq_users_credential_email
    ON users_credential (email);

-- org_id 索引 (per v2.0 §4.1)
CREATE INDEX IF NOT EXISTS idx_users_credential_org_id_v2
    ON users_credential (org_id);

-- sessions 表新增 (per v2.0 §4.1): 与 0003 refresh_token_revoke 共存
-- v2.0 要求 refresh_token_hash UNIQUE, 与既有 jti-revoke 模型并存
-- (refresh token 是 JWT 自包含, DB 仅记录撤销场景; sessions 表是 v2.0 标准化)
CREATE TABLE IF NOT EXISTS sessions (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID NOT NULL REFERENCES users_credential(id) ON DELETE CASCADE,
    refresh_token_hash  TEXT NOT NULL UNIQUE,
    issued_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at          TIMESTAMPTZ NOT NULL,
    revoked_at          TIMESTAMPTZ,
    client_kind         TEXT NOT NULL CHECK (client_kind IN ('tauri','web'))
);
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions (user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at
    ON sessions (expires_at) WHERE revoked_at IS NULL;

-- roles + role_bindings (per v2.0 §4.1)
CREATE TABLE IF NOT EXISTS roles (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code        TEXT NOT NULL UNIQUE,
    description TEXT
);

CREATE TABLE IF NOT EXISTS role_bindings (
    id          BIGSERIAL PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users_credential(id) ON DELETE CASCADE,
    role_id     UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    org_id      UUID NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, role_id, org_id)
);
CREATE INDEX IF NOT EXISTS idx_role_bindings_user_id ON role_bindings (user_id);
