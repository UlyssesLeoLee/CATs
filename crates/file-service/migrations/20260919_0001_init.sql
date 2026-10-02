-- =====================================================================
-- CATs file_db schema  (per 数据库设计书 v2.0 §4.5)
-- 引用:
--   doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §4.5
-- 角色矩阵: file_db → migrator_file (DDL) / svc_file (CRUD)
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS files (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id          UUID NOT NULL,
    project_id      UUID,
    filename        TEXT NOT NULL,
    content_type    TEXT NOT NULL,
    size_bytes      BIGINT NOT NULL,
    storage_backend TEXT NOT NULL DEFAULT 'minio' CHECK (storage_backend IN ('minio','nfs')),
    storage_key     TEXT NOT NULL,
    purpose         TEXT NOT NULL DEFAULT 'task_source'
                        CHECK (purpose IN ('task_source','task_derived','other')),
    status          TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','deleted')),
    uploaded_by     UUID NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at      TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_files_org_id ON files (org_id);
CREATE INDEX IF NOT EXISTS idx_files_project_id ON files (project_id);

CREATE TABLE IF NOT EXISTS file_versions (
    id                  BIGSERIAL PRIMARY KEY,
    file_id             UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    version             INT NOT NULL,
    storage_key         TEXT NOT NULL,
    size_bytes          BIGINT NOT NULL,
    created_by_service  TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (file_id, version)
);

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
