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

-- projects 表已停用，权威 schema 见 20260920_0001_init.sql。
-- 本文件的 projects 用 org_id 多租户隔离，而 project-service 代码只用
-- workspace_id（src/db.rs 中出现 18 次，org_id 零引用）。
-- 本文件版本号最小、最先执行，会抢先建表，导致权威版被静默跳过、
-- 其索引 ON projects (workspace_id, ...) 报 column "workspace_id" does not exist。
-- 即在全新数据库上 project_db 初始化无法完成，生产首次部署同样会踩到。
-- 下方 project_members 不与权威版冲突，保留。
-- updated_at trigger 与 projects 一并停用：权威版 20260920_0001_init.sql
-- 自带等价 trigger，此处保留会在表尚未建出时报 relation "projects" does not exist。

-- project_members (per 接口设计 §6.3.5)
-- projects(id) 由 20260920_0001_init.sql 建立（版本号更大、更晚执行），
-- 故此处不能用内联 REFERENCES；等权威版建出 projects 后再补外键。
CREATE TABLE IF NOT EXISTS project_members (
    project_id  UUID NOT NULL,
    user_id     UUID NOT NULL,
    role        TEXT NOT NULL DEFAULT 'member',  -- owner / lead / member / viewer
    added_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_project_members_user_id ON project_members (user_id);

DO $fk$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'projects')
       AND NOT EXISTS (
           SELECT 1 FROM pg_constraint WHERE conname = 'project_members_project_id_fkey'
       ) THEN
        ALTER TABLE project_members
            ADD CONSTRAINT project_members_project_id_fkey
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE;
    END IF;
END
$fk$;