-- CATs file_db schema (per ULYS-152 切片 B-3 + 微服务架构书 v1.0 §4.1 file-service)
-- 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1 (file-service)
-- 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.1 (file_db 接口契约 v1.0.0)
-- 引用: doc/05-其他/管理/CATs_错误码表_v1.0.md §3 (error enum)
--
-- 设计选择 (per 缺标比错标安全, 守门 #11):
-- - 接口设计书 v2.0 整份不存在 (per Sprint 1 §6.1 缺口)
--   → DDL 基于微服务架构书 §4.1 + 本切片 B-3 任务描述
-- - 与 user_db / auth_db 分离: 8 逻辑库边界清晰 (per 架构书 §5.1)
-- - 软删除 (status 列) 而非物理删除 (per 业务: 翻译工作流的文件需要审计追溯)
-- - storage_path 是抽象字段: M1 本地磁盘 / Sprint 3 S3 key
-- - sha256 用于去重 + 完整性校验 (per 安全基线)
--
-- 不可逆性: 中 (新表, 8 逻辑库第 5 个)
-- 性能: 主键 + workspace_id 索引, owner_user_id 索引

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS files (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id    UUID NOT NULL,
    owner_user_id   UUID NOT NULL,
    filename        TEXT NOT NULL,
    content_type    TEXT NOT NULL DEFAULT 'application/octet-stream',
    size_bytes      BIGINT NOT NULL CHECK (size_bytes >= 0),
    storage_path    TEXT NOT NULL,
    sha256          TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'active'
                    CHECK (status IN ('active','deleted')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 索引: 按 workspace_id 过滤活跃文件 (per list 场景)
CREATE INDEX IF NOT EXISTS idx_files_workspace_status
    ON files (workspace_id, status)
    WHERE status = 'active';

-- 索引: 按 owner_user_id 查找自己文件
CREATE INDEX IF NOT EXISTS idx_files_owner_user_id
    ON files (owner_user_id);

-- 索引: 按 sha256 去重查询 (per upload 幂等)
CREATE INDEX IF NOT EXISTS idx_files_sha256
    ON files (workspace_id, sha256)
    WHERE status = 'active';

-- updated_at 自动维护 trigger (per auth_db 0001 模式)
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