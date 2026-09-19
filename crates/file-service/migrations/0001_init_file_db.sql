-- CATs file_db schema (per 数据库设计书 v2.0 §6.5 + 接口设计 v2.0 §6.5)
-- 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §6.5
-- 引用: doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md §6.5
--
-- MVP 范围:
-- - files 元数据表 (id, org_id, name, content_type, size, sha256, storage_path, status)
-- - 本地落盘路径: ./var/files/{org_id}/{file_id} (per brief §1 file-service)

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE IF NOT EXISTS files (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id        UUID NOT NULL,                          -- 多租户隔离
    project_id    UUID,                                    -- 关联 project (nullable, 跨项目共享文件)
    task_id       UUID,                                    -- 关联 task (nullable)
    name          TEXT NOT NULL,                            -- 原始文件名
    content_type  TEXT,
    size_bytes    BIGINT NOT NULL DEFAULT 0,
    sha256        TEXT,
    storage_path  TEXT NOT NULL,                           -- 本地路径 (per brief §1 file-service)
    status        TEXT NOT NULL DEFAULT 'uploaded',         -- uploaded / processing / ready / deleted
    uploaded_by   UUID NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_files_org_id ON files (org_id);
CREATE INDEX IF NOT EXISTS idx_files_project_id ON files (project_id);
CREATE INDEX IF NOT EXISTS idx_files_sha256 ON files (sha256);

CREATE OR REPLACE FUNCTION trg_set_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS files_set_updated_at ON files;
CREATE TRIGGER files_set_updated_at
    BEFORE UPDATE ON files
    FOR EACH ROW
    EXECUTE FUNCTION trg_set_updated_at();