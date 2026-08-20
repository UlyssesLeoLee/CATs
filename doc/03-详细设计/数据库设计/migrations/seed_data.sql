-- ==============================================================================
-- CATs Platform: Default System Seed Data (seed_data.sql)
-- ==============================================================================

-- 1. Default System Tenant
INSERT INTO tenants (id, name, plan_tier, is_active)
VALUES ('00000000-0000-0000-0000-000000000001', 'Default Enterprise Tenant', 'enterprise', true)
ON CONFLICT (id) DO NOTHING;

-- 2. Default Administrator (password: admin123 -> argon2 hash example)
INSERT INTO users (id, tenant_id, username, email, password_hash, role, is_active)
VALUES (
    '00000000-0000-0000-0000-000000000002',
    '00000000-0000-0000-0000-000000000001',
    'admin',
    'admin@cats.internal',
    '$argon2id$v=19$m=65536,t=3,p=4$dGVzdHNhbHQ$q38mYx4T1F9z5f5S1j1c8Q',
    'admin',
    true
)
ON CONFLICT DO NOTHING;

-- 3. Base Terms Seed Example
INSERT INTO terms (tenant_id, source_term, target_term, is_forbidden)
VALUES 
('00000000-0000-0000-0000-000000000001', 'Translation Memory', '翻译记忆库', false),
('00000000-0000-0000-0000-000000000001', 'Termbase', '术语库', false),
('00000000-0000-0000-0000-000000000001', 'Fail-Closed', '合规阻断', false)
ON CONFLICT DO NOTHING;