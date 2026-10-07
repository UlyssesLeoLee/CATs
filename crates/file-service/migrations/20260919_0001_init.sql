-- =====================================================================
-- CATs file_db schema  (per 数据库设计书 v2.0 §4.5)
-- 引用:
--   doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §4.5
-- 角色矩阵: file_db → migrator_file (DDL) / svc_file (CRUD)
--
-- ---------------------------------------------------------------------
-- files 表已停用 (per 2026-10-03 真实 PG 验证)
--
-- 本文件的 files 是 org_id + storage_backend/storage_key + purpose + deleted_at
-- 模型；而 file-service 实际代码用 workspace_id + owner_user_id + filename +
-- storage_path（src/db.rs 中 workspace_id 出现 13 次、storage_path 6 次、
-- filename 5 次、owner_user_id 5 次；本文件的四个列名一个都没被代码引用）。
-- 权威 schema 是 20260920_0001_init.sql。
--
-- 三份 init 都 CREATE TABLE IF NOT EXISTS files，本文件排在权威版之前，
-- 会抢先建出这套模型，权威版随后被静默跳过，其索引
-- ON files (workspace_id, status) 报 column "workspace_id" does not exist。
-- 即在全新数据库上 file_db 初始化无法完成，生产首次部署同样会踩到。
--
-- 下方 file_versions / outbox_event 不与权威版冲突，故保留。
-- ---------------------------------------------------------------------
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- files 表：故意留空，权威 schema 见 20260920_0001_init.sql
-- （file_versions 引用 files(id)，故在权威版建表后由下方语句执行）

-- files 表已停用，权威 schema 见 20260920_0001_init.sql。

CREATE TABLE IF NOT EXISTS file_versions (
    id                  BIGSERIAL PRIMARY KEY,
    file_id             UUID NOT NULL,
    version             INT NOT NULL,
    storage_key         TEXT NOT NULL,
    size_bytes          BIGINT NOT NULL,
    created_by_service  TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (file_id, version)
);

-- files(id) 由 20260920_0001_init.sql 建立（版本号更大、更晚执行），
-- 故此处不能用内联 REFERENCES；等权威版建出 files 后再补外键。
DO $fk$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'files')
       AND NOT EXISTS (
           SELECT 1 FROM pg_constraint WHERE conname = 'file_versions_file_id_fkey'
       ) THEN
        ALTER TABLE file_versions
            ADD CONSTRAINT file_versions_file_id_fkey
            FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE;
    END IF;
END
$fk$;

-- file_db 的 outbox_event (per §8 CDC 对应)
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
CREATE INDEX IF NOT EXISTS idx_file_db_outbox_created_at ON outbox_event (created_at);
