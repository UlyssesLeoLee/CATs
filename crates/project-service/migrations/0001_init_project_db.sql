-- CATs project_db schema (per 数据库设计书 v2.0 §6.4 + 微服务架构设计书 §5.1)
-- 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §5.1
-- 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §6.4 (project_db 章节)
-- 引用: doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md §6.3 (project-service)
--
-- MVP 范围 (per 接口设计 v2.0 §6.3.6 Sprint 1 完成判据):
-- - projects 主表 (id, org_id, name, description, status, created_by)
-- - project_members 项目成员表 (per §6.3.5)
-- 多租户最简隔离 (per §1.2 + 微服务架构 §14)

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE IF NOT EXISTS projects (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID NOT NULL,                -- 多租户隔离 (per §14)
    name        TEXT NOT NULL,
    description TEXT,
    status      TEXT NOT NULL DEFAULT 'active',  -- active / archived
    source_lang TEXT,                         -- BCP-47 (per 接口设计 §6.3.5)
    target_lang TEXT,
    created_by  UUID NOT NULL,                -- user_id
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_projects_org_id ON projects (org_id);
CREATE INDEX IF NOT EXISTS idx_projects_created_by ON projects (created_by);

-- updated_at 自动维护 trigger (per auth-service/migrations/0001 复用)
CREATE OR REPLACE FUNCTION trg_set_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS projects_set_updated_at ON projects;
CREATE TRIGGER projects_set_updated_at
    BEFORE UPDATE ON projects
    FOR EACH ROW
    EXECUTE FUNCTION trg_set_updated_at();

-- project_members (per 接口设计 §6.3.5)
CREATE TABLE IF NOT EXISTS project_members (
    project_id  UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL,
    role        TEXT NOT NULL DEFAULT 'member',  -- owner / lead / member / viewer
    added_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_project_members_user_id ON project_members (user_id);