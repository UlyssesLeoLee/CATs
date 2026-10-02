-- =====================================================================
-- CATs task_db schema  (per 数据库设计书 v2.0 §4.4)
-- 引用:
--   doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §4.4
--   doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §5.1
-- 引擎: PostgreSQL 18.6 (per 技术基线 v1.0 §1)
-- 角色矩阵: task_db → migrator_task (DDL) / svc_task (CRUD) / svc_report_ro (只读 tasks+task_media_items)
-- 备注: task_db 同时承载媒体处理派生数据 (asr_transcripts / subtitle_segments / media_assets)
--       per §1 "media_job_db 不新增, 统一归口 task_db"
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- tasks 主表
CREATE TABLE IF NOT EXISTS tasks (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id      UUID NOT NULL,
    org_id          UUID NOT NULL,
    media_type      TEXT NOT NULL CHECK (media_type IN
                       ('text','audio','video','pdf','docx','xlsx','pptx',
                        'odt','ods','odp','gif','webp')),
    source_lang     TEXT NOT NULL,
    target_lang     TEXT NOT NULL,
    source_file_id  UUID NOT NULL,                      -- 逻辑外键 → file_db.files.id
    status          TEXT NOT NULL DEFAULT 'queued' CHECK (status IN
                       ('queued','ingesting','processing','rendering',
                        'completed','failed','canceled','partially_failed')),
    priority        TEXT NOT NULL DEFAULT 'normal' CHECK (priority IN ('low','normal','high')),
    output_formats  TEXT[] NOT NULL DEFAULT '{}',
    created_by      UUID NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at    TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_tasks_project_status_created
    ON tasks (project_id, status, created_at);
CREATE INDEX IF NOT EXISTS idx_tasks_org_status
    ON tasks (org_id, status);
CREATE INDEX IF NOT EXISTS idx_tasks_stalled
    ON tasks (status, updated_at)
    WHERE status IN ('ingesting','processing','rendering');

-- task_media_items (per-task 媒体子阶段, asr/ocr/subtitle/office/render)
CREATE TABLE IF NOT EXISTS task_media_items (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id         UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    stage           TEXT NOT NULL CHECK (stage IN
                       ('ingest','asr','ocr','subtitle','office','render')),
    status          TEXT NOT NULL DEFAULT 'pending' CHECK (status IN
                       ('pending','running','completed','failed','skipped')),
    result_file_id  UUID,                               -- 逻辑外键 → file_db.files.id
    error_code      TEXT,
    error_message   TEXT,
    metrics         JSONB,
    started_at      TIMESTAMPTZ,
    finished_at     TIMESTAMPTZ,
    UNIQUE (task_id, stage)
);
CREATE INDEX IF NOT EXISTS idx_task_media_items_task_id ON task_media_items (task_id);

-- media_assets
CREATE TABLE IF NOT EXISTS media_assets (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id             UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    file_id             UUID NOT NULL,                  -- 逻辑外键 → file_db.files.id
    asset_kind          TEXT NOT NULL CHECK (asset_kind IN
                           ('source','asr_transcript','ocr_result','subtitle',
                            'office_translated','render_output')),
    media_type          TEXT NOT NULL,
    duration_seconds    NUMERIC(10,2),
    page_count          INT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_media_assets_task_id ON media_assets (task_id);
CREATE INDEX IF NOT EXISTS idx_media_assets_kind ON media_assets (task_id, asset_kind);

-- asr_transcripts
CREATE TABLE IF NOT EXISTS asr_transcripts (
    id              BIGSERIAL PRIMARY KEY,
    task_id         UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    media_asset_id  UUID NOT NULL REFERENCES media_assets(id) ON DELETE CASCADE,
    seq             INT NOT NULL,
    start_ms        INT NOT NULL,
    end_ms          INT NOT NULL,
    text            TEXT NOT NULL,
    confidence      REAL,
    UNIQUE (media_asset_id, seq)
);
CREATE INDEX IF NOT EXISTS idx_asr_transcripts_task_id ON asr_transcripts (task_id);

-- subtitle_segments (翻译单元)
CREATE TABLE IF NOT EXISTS subtitle_segments (
    id              BIGSERIAL PRIMARY KEY,
    task_id         UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    media_asset_id  UUID NOT NULL REFERENCES media_assets(id) ON DELETE CASCADE,
    seq             INT NOT NULL,
    start_ms        INT NOT NULL,
    end_ms          INT NOT NULL,
    source_text     TEXT NOT NULL,
    target_text     TEXT,
    tm_level        TEXT CHECK (tm_level IN ('L0','L1','MT','MISS')),
    qa_pass         BOOLEAN,
    UNIQUE (media_asset_id, seq)
);
CREATE INDEX IF NOT EXISTS idx_subtitle_segments_task_id ON subtitle_segments (task_id);

-- task_events_outbox (架构设计书 §7.2 原始示例)
CREATE TABLE IF NOT EXISTS task_events_outbox (
    id              BIGSERIAL PRIMARY KEY,
    event_id        UUID NOT NULL DEFAULT gen_random_uuid(),
    aggregate_type  TEXT NOT NULL,
    aggregate_id    TEXT NOT NULL,
    event_type      TEXT NOT NULL,
    payload         JSONB NOT NULL,
    schema_version  INT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_task_events_outbox_created_at
    ON task_events_outbox (created_at);

-- updated_at trigger (复用同一个函数体, 各库各自 IF NOT EXISTS 防重定义)
CREATE OR REPLACE FUNCTION trg_set_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS tasks_set_updated_at ON tasks;
CREATE TRIGGER tasks_set_updated_at
    BEFORE UPDATE ON tasks
    FOR EACH ROW EXECUTE FUNCTION trg_set_updated_at();
