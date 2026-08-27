-- CATs auth_db schema 增量 (per T-01 auth-service 实战深化)
-- 引用: doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md §2 T-01
-- 引用: doc/02-基础设计/接口设计/CATs_接口设计书_v2.0.md §3 (refresh 端点)
--
-- 变更:
-- - 新增 refresh_token_revoke 表 (jti 撤销, JWT 轮换用)
--   选 jti 而非整个 token: 减少存储, 验证 O(1) 查
--   不设外键到 users_credential: 用户删除时审计仍保留
--
-- 不可逆性: 中 (新表, 涉及 schema 演进, 但独立表无破坏)
-- 性能: 撤销检查为 PK 查, 索引自动; 不影响其他表

CREATE TABLE IF NOT EXISTS refresh_token_revoke (
    jti         UUID PRIMARY KEY,
    user_id     UUID NOT NULL,
    revoked_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    reason      TEXT NOT NULL  -- 'rotated' | 'logout' | 'admin_revoke' | 'expired'
);

-- 索引 (per 150 任务 #48 性能建议: 按 user_id 查询某用户全部撤销记录)
CREATE INDEX IF NOT EXISTS idx_refresh_token_revoke_user_id
    ON refresh_token_revoke (user_id);

-- 索引 (cleanup 作业: 删除过期撤销记录, 7 天前)
CREATE INDEX IF NOT EXISTS idx_refresh_token_revoke_revoked_at
    ON refresh_token_revoke (revoked_at);
