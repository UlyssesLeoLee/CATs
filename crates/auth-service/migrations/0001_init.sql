-- CATs auth_db schema (per 微服务架构设计书 v1.1 §5.1 + Baseline §5.1)
-- 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §5.1
-- 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §6.2 (auth_db 章节)
-- 引用: doc/05-其他/安全/CATs_安全要件定义书_v1.0.md §3 (认证)

-- 启用 pgcrypto 用于 gen_random_uuid()
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- users_credential 主表 (per CATs_Baseline一览 §5.1 auth_db 章节)
CREATE TABLE IF NOT EXISTS users_credential (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username    TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    is_active   BOOLEAN NOT NULL DEFAULT true,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- updated_at 自动维护 trigger
CREATE OR REPLACE FUNCTION trg_set_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS users_credential_set_updated_at ON users_credential;
CREATE TRIGGER users_credential_set_updated_at
    BEFORE UPDATE ON users_credential
    FOR EACH ROW
    EXECUTE FUNCTION trg_set_updated_at();

-- 索引 (per 数据库设计书 §6.2)
CREATE INDEX IF NOT EXISTS idx_users_credential_username ON users_credential (username);
CREATE INDEX IF NOT EXISTS idx_users_credential_is_active ON users_credential (is_active) WHERE is_active = true;
