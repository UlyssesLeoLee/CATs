-- CATs auth_db schema 增量 (per T-01 audit_log DB 兜底)
-- 引用: doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md §2 T-01
-- 引用: doc/05-其他/安全/CATs_安全要件定义书_v1.0.md §6 (审计)
--
-- 变更:
-- - 新增 audit_log 表 (审计事件落库, 永不丢)
--   Kafka 物理落地推 K3s 阶段二 (per Sprint 1 §6.10 已知缺口)
--   当前用 InMemoryAuditSink + DB 兜底, T-01 范围内可验证
--
-- 不可逆性: 中 (新表)
-- 性能: 写入为主, 不查; 仅 PK + 索引 user_id/event_type/timestamp

CREATE TABLE IF NOT EXISTS audit_log (
    id              BIGSERIAL PRIMARY KEY,
    event_id        UUID NOT NULL UNIQUE,                -- 防重
    user_id         UUID,                                 -- nullable for 系统事件
    event_type      TEXT NOT NULL,                        -- 'login' | 'logout' | 'refresh' | 'refresh_revoked' | 'login_failed' 等
    outcome         TEXT NOT NULL,                        -- 'success' | 'failure'
    detail          JSONB,                                -- 结构化 detail (per §3 错误码表 v1.0)
    source_ip       TEXT,                                 -- 客户端 IP (per §6.1 安全要件, 改 TEXT 简化 bind, 后续可补 inet::text 索引)
    user_agent      TEXT,                                 -- 客户端 UA
    occurred_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 索引: 按 user_id + 时间倒序 (查询某用户审计历史)
CREATE INDEX IF NOT EXISTS idx_audit_log_user_id_occurred_at
    ON audit_log (user_id, occurred_at DESC);

-- 索引: 按 event_type 分类统计
CREATE INDEX IF NOT EXISTS idx_audit_log_event_type
    ON audit_log (event_type);

-- 索引: 按时间 (cleanup 90 天前的)
CREATE INDEX IF NOT EXISTS idx_audit_log_occurred_at
    ON audit_log (occurred_at);
