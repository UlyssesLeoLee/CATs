-- CATs auth_db schema 增量 (per GET /v1/auth/me 端点要求)
-- 引用: doc/02-基础设计/接口设计/CATs_接口设计书_v2.0.md §3.2 (MeResponse)
-- 引用: api/openapi/cats-openapi-v1.yaml
--
-- 变更:
-- - users_credential 加 email TEXT (nullable, 无 UNIQUE 约束, 后续可补)
--   选 nullable: 允许已存在的种子用户不被强制重置
--
-- 不可逆性: 低 (仅加列, 不破坏既有数据)

ALTER TABLE users_credential
    ADD COLUMN IF NOT EXISTS email TEXT;
