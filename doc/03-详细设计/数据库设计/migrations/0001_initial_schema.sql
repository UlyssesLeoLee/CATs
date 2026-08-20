-- ==============================================================================
-- CATs Platform: Initial PostgreSQL 16 DDL Migration (0001_initial_schema.sql)
-- Multi-database / Logical schemas for CATs microservice ecosystem
-- ==============================================================================

-- Enable Extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "vector";

-- ==============================================================================
-- 1. AUTH & USER SERVICE (auth_db / user_db)
-- ==============================================================================
CREATE TABLE IF NOT EXISTS tenants (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    plan_tier VARCHAR(32) NOT NULL DEFAULT 'standard',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    username VARCHAR(64) NOT NULL,
    email VARCHAR(128) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    role VARCHAR(32) NOT NULL DEFAULT 'linguist',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_tenant_username UNIQUE (tenant_id, username),
    CONSTRAINT uq_tenant_email UNIQUE (tenant_id, email)
);

-- ==============================================================================
-- 2. PROJECT & TM & TERM SERVICE (project_db)
-- ==============================================================================
CREATE TABLE IF NOT EXISTS projects (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name VARCHAR(128) NOT NULL,
    source_lang VARCHAR(16) NOT NULL,
    target_lang VARCHAR(16) NOT NULL,
    created_by UUID NOT NULL REFERENCES users(id),
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS tm_entries (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID REFERENCES projects(id) ON DELETE SET NULL,
    source_lang VARCHAR(16) NOT NULL,
    target_lang VARCHAR(16) NOT NULL,
    source_text TEXT NOT NULL,
    target_text TEXT NOT NULL,
    embedding vector(1536), -- OpenAI / Ollama embedding vector
    usage_count INT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- HNSW Vector Index for TM rapid similarity search
CREATE INDEX IF NOT EXISTS idx_tm_embedding_hnsw 
ON tm_entries USING hnsw (embedding vector_cosine_ops)
WITH (m = 16, ef_construction = 64);

CREATE TABLE IF NOT EXISTS terms (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID REFERENCES projects(id) ON DELETE SET NULL,
    source_term VARCHAR(255) NOT NULL,
    target_term VARCHAR(255) NOT NULL,
    is_forbidden BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_terms_tenant_source 
ON terms (tenant_id, source_term);

-- ==============================================================================
-- 3. TASK & MEDIA PROCESSING SERVICE (task_db)
-- ==============================================================================
CREATE TABLE IF NOT EXISTS tasks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    media_type VARCHAR(32) NOT NULL, -- TEXT, AUDIO, VIDEO, PDF, OFFICE, GAME
    status VARCHAR(32) NOT NULL DEFAULT 'PENDING',
    progress INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS task_media_items (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    item_index INT NOT NULL,
    time_start_ms BIGINT,
    time_end_ms BIGINT,
    source_raw TEXT NOT NULL,
    target_translated TEXT,
    qa_status VARCHAR(32) NOT NULL DEFAULT 'UNCHECKED',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_media_task_index 
ON task_media_items (task_id, item_index);