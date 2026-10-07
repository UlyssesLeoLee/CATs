-- CATs project_db schema (per ULYS-150 切片 B-1 project-service 业务实现)
-- 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1 (project-service)
-- 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md (projects 表)
-- 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.2 (project_db 接口契约 v1.0.0)
-- 引用: ULYS-150 切片 B-1 任务书 §"数据模型"
--
-- 设计选择 (per 缺标比错标安全):
-- - 本表 schema 基于切片 B-1 任务书 §数据模型 + 数据库设计书 v2.0
-- - workspace_id 是逻辑外键: 跨服务不直连, 一致性由调用方保证 (per user_db 0001 模式)
-- - owner_user_id 是逻辑外键: 跨服务 user_id 一致 (per user-service 8 逻辑库边界)
-- - status CHECK 约束: 'active' / 'archived' / 'completed' 三态
-- - 不直连 tenants/workspaces/users 表 (per 8 逻辑库边界清晰)
--
-- 不可逆性: 中 (新表, 8 逻辑库第 3 个 project_db)
-- 性能: 主键 + workspace_id 索引 + owner_user_id 索引

CREATE TABLE IF NOT EXISTS projects (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id    UUID NOT NULL,                            -- 关联 workspaces.id, 业务一致
    name            TEXT NOT NULL,
    source_lang     TEXT NOT NULL,
    target_lang     TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','archived','completed')),
    owner_user_id   UUID NOT NULL,                            -- 关联 user_service.user_profile.user_id
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 索引: 按 workspace_id 过滤 (per GET /v1/projects?workspace_id=...)
CREATE INDEX IF NOT EXISTS idx_projects_workspace_id
    ON projects (workspace_id);

-- 索引: 按 owner_user_id 过滤 (per 个人项目列表)
CREATE INDEX IF NOT EXISTS idx_projects_owner_user_id
    ON projects (owner_user_id);

-- 索引: 按 status 过滤 (per 排除 archived)
CREATE INDEX IF NOT EXISTS idx_projects_status
    ON projects (status) WHERE status = 'active';

-- updated_at 自动维护 trigger (per user_db 0001 模式)
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