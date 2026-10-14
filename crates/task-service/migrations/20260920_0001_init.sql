-- CATs task_db schema (per ULYS-151 切片 B-2 task-service 业务 + SSE)
-- 引用: doc/05-其他/MVP商业版/_slice_b2_task.md
-- 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1 (task-service)
-- 引用: doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md §3.4 (task-service)
--
-- 不可逆性: 中 (新表, 第 8 个逻辑库边界, per 架构书 §5.1 8 逻辑库清单)
-- 性能: 主键 UUID + project_id 索引 + status 索引, 配合分页 (limit + offset)
--
-- 设计选择 (per 缺标比错标安全):
-- - task_type TEXT + CHECK 约束 (与微服务架构书 §4.1 / 接口设计书 §3.4 状态机一致)
-- - status 默认 'pending', 与枚举 (pending / running / completed / failed / cancelled) 绑定
-- - input_payload / output_payload JSONB (per 接口设计书 §3.4 业务 payload)
-- - progress INT 0..=100 CHECK (与 SSE 上报的 progress 字段一致)
-- - error_message TEXT (status='failed' 时非空)
-- - created_at / updated_at / started_at / completed_at TIMESTAMPTZ (per 用户组织服务 §5.1 时间戳约定)
-- - project_id **不**做 FK 到 project.project: 跨服务不直连 (per 架构书 §1.2 原则 4);
--   调用方负责传入合法 project_id; project 不存在时在 handler 层返回 404 project_not_found

CREATE TABLE IF NOT EXISTS tasks (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id      UUID NOT NULL,                                  -- 关联 project-service; 不做 FK, 跨服务边界
    task_type       TEXT NOT NULL CHECK (task_type IN ('translate','review','export')),
    status          TEXT NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending','running','completed','failed','cancelled')),
    input_payload   JSONB NOT NULL DEFAULT '{}'::jsonb,
    output_payload  JSONB NOT NULL DEFAULT '{}'::jsonb,
    progress        INTEGER NOT NULL DEFAULT 0 CHECK (progress BETWEEN 0 AND 100),
    error_message   TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at      TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ
);

-- 索引: 按 project_id 过滤 (per GET /v1/tasks?project_id=)
CREATE INDEX IF NOT EXISTS idx_tasks_project_id ON tasks (project_id);

-- 索引: 按 status 过滤 (per pending/running 队列查询)
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks (status);

-- 索引: 按 project_id + status 复合 (per "某项目下 pending 任务" 常见查询)
CREATE INDEX IF NOT EXISTS idx_tasks_project_id_status ON tasks (project_id, status);

-- 索引: 按 created_at DESC (per 列表分页排序)
CREATE INDEX IF NOT EXISTS idx_tasks_created_at ON tasks (created_at DESC);

-- 索引: 按 updated_at DESC (per 后台任务监控, SSE 终态同步)
CREATE INDEX IF NOT EXISTS idx_tasks_updated_at ON tasks (updated_at DESC);

-- updated_at 自动维护 trigger (per auth_db 0001 模式 / user_db 0001 模式)
CREATE OR REPLACE FUNCTION trg_set_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS tasks_set_updated_at ON tasks;
CREATE TRIGGER tasks_set_updated_at
    BEFORE UPDATE ON tasks
    FOR EACH ROW
    EXECUTE FUNCTION trg_set_updated_at();