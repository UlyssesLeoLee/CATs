-- =====================================================================
-- CATs project_db schema  (per 数据库设计书 v2.0 §4.3)
-- 引用:
--   doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §4.3
--   doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §5 (索引)
--   doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §6 (分区)
--   doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1 (PG 18.6 + pgvector 0.8.6)
-- 引擎: PostgreSQL 18.6 + pgvector 0.8.6 (per ADR-30)
-- 迁移工具: sqlx-migrate (per §3 迁移工具选型)
-- 角色矩阵: project_db → migrator_project (DDL) / svc_project (CRUD) / svc_translation (TM+term)
-- =====================================================================

-- pgvector 扩展 (per 技术选型 ADR-30 + 技术基线 v1.0 §1)
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pgcrypto; -- gen_random_uuid() 兜底(不依赖 pgcrypto schema 路径)

-- projects 主表
CREATE TABLE IF NOT EXISTS projects (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id              UUID NOT NULL,
    name                TEXT NOT NULL,
    source_lang         TEXT NOT NULL,                  -- BCP-47
    target_lang         TEXT NOT NULL,
    domain              TEXT NOT NULL DEFAULT '',
    force_local_model   BOOLEAN NOT NULL DEFAULT false,
    status              TEXT NOT NULL DEFAULT 'active'
                            CHECK (status IN ('active','archived')),
    created_by          UUID NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_projects_org_id ON projects (org_id);

-- terms (项目级术语)
CREATE TABLE IF NOT EXISTS terms (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id      UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    source_term     TEXT NOT NULL,
    target_term     TEXT NOT NULL,
    domain_tag      TEXT NOT NULL DEFAULT '',
    forbidden       BOOLEAN NOT NULL DEFAULT false,
    version         INT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX IF NOT EXISTS uq_terms_project_source ON terms (project_id, source_term);

-- glossary_versions (术语变更历史)
CREATE TABLE IF NOT EXISTS glossary_versions (
    id              BIGSERIAL PRIMARY KEY,
    project_id      UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    version         INT NOT NULL,
    changed_term_id UUID NOT NULL,
    change_kind     TEXT NOT NULL
                        CHECK (change_kind IN ('created','updated','deleted')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_glossary_versions_project
    ON glossary_versions (project_id, version DESC);

-- translation_memory 主表 (按 project_id HASH 分区, 16 个分区)
CREATE TABLE IF NOT EXISTS translation_memory (
    id              BIGSERIAL,
    project_id      UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    source_lang     TEXT NOT NULL,
    target_lang     TEXT NOT NULL,
    source_text     TEXT NOT NULL,
    target_text     TEXT NOT NULL,
    source_hash     TEXT NOT NULL,                      -- sha256(normalize(source_text))
    origin          TEXT NOT NULL DEFAULT 'human'
                        CHECK (origin IN ('human','mt_confirmed','import')),
    quality_score   SMALLINT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, id)
) PARTITION BY HASH (project_id);

-- 16 个 HASH 分区 (per §5.1 / §6 分区策略)
DO $$
DECLARE i INT;
BEGIN
    FOR i IN 0..15 LOOP
        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS translation_memory_p%1$s '
            'PARTITION OF translation_memory '
            'FOR VALUES WITH (MODULUS 16, REMAINDER %1$s);',
            lpad(i::text, 2, '0')
        );
    END LOOP;
END $$;

CREATE INDEX IF NOT EXISTS idx_tm_exact_match
    ON translation_memory (project_id, source_lang, target_lang, source_hash);

-- tm_vectors (语义向量索引, 配套 translation_memory)
CREATE TABLE IF NOT EXISTS tm_vectors (
    project_id      UUID NOT NULL,
    tm_id           BIGINT NOT NULL,
    embedding       vector(1024) NOT NULL,             -- bge-m3 维度
    PRIMARY KEY (project_id, tm_id),
    FOREIGN KEY (project_id, tm_id)
        REFERENCES translation_memory (project_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_tm_vectors_hnsw
    ON tm_vectors USING hnsw (embedding vector_cosine_ops);

-- project_db 的 Outbox (架构设计书 §7.2)
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
CREATE INDEX IF NOT EXISTS idx_project_db_outbox_created_at
    ON outbox_event (created_at);

-- updated_at trigger
CREATE OR REPLACE FUNCTION trg_set_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS projects_set_updated_at ON projects;
CREATE TRIGGER projects_set_updated_at
    BEFORE UPDATE ON projects
    FOR EACH ROW EXECUTE FUNCTION trg_set_updated_at();

DROP TRIGGER IF EXISTS terms_set_updated_at ON terms;
CREATE TRIGGER terms_set_updated_at
    BEFORE UPDATE ON terms
    FOR EACH ROW EXECUTE FUNCTION trg_set_updated_at();
