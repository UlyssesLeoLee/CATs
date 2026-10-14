-- CATs file_db schema (per 数据库设计书 v2.0 §6.5 + 接口设计 v2.0 §6.5)
-- 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §6.5
-- 引用: doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md §6.5
--
-- MVP 范围:
-- - files 元数据表 (id, org_id, name, content_type, size, sha256, storage_path, status)
-- - 本地落盘路径: ./var/files/{org_id}/{file_id} (per brief §1 file-service)
--
-- ---------------------------------------------------------------------
-- 已停用 (per 2026-10-03 真实 PG 验证)
--
-- 本文件是 org_id / project_id / name / uploaded_by 租户模型；
-- file-service 实际代码用的是 workspace_id / owner_user_id / filename 模型
-- (src/db.rs 的 INSERT 明确写 workspace_id, owner_user_id, filename)。
-- 权威 schema 是 20260920_0001_init.sql。
--
-- 本文件版本号最小、最先执行，会抢先建出 org_id 模型的 files 表，
-- 权威版的 CREATE TABLE 被静默跳过，随后其索引
-- ON files (workspace_id, status) 引用不存在的列而失败：
--     ERROR: column "workspace_id" does not exist
-- 即在全新数据库上 file_db 的 schema 初始化无法完成，生产首次部署同样会踩到。
--
-- 处理：不再建表与相关索引，权威 schema 见 20260920_0001_init.sql。
-- ---------------------------------------------------------------------

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- 故意留空：权威 schema 见 20260920_0001_init.sql
-- （updated_at trigger 已随该表一并停用——权威版自带等价 trigger，
--   此处保留会在表尚未建出时因 relation "files" does not exist 而失败）