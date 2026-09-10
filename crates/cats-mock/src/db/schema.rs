//! SQL DDL 集合 (per 16 service 的 schema)
//!
//! 引用: 设计书 §4.2.1
//!
//! 注意: 这是 DDL 的**测试快照**, 与各 service 的 migrations 同步
//! (每次 service migration 变更, 这里需要同步更新 — 由 Test Quality Lead 兜底)

/// Schema 集合: 一组表 + 索引的 DDL
#[derive(Debug, Clone, Default)]
pub struct SchemaSet {
    /// 所有 DDL 字符串 (按顺序执行)
    pub statements: Vec<String>,
}

impl SchemaSet {
    /// 空 schema
    pub fn new() -> Self { Self::default() }

    /// 加一条 DDL
    pub fn push(mut self, sql: impl Into<String>) -> Self {
        self.statements.push(sql.into());
        self
    }

    /// 合并两个 schema
    pub fn merge(mut self, other: SchemaSet) -> Self {
        self.statements.extend(other.statements);
        self
    }

    /// 公共 schema: users / projects / tasks / audit_log (跨 service)
    pub fn all_common() -> SchemaSet {
        users_schema()
            .merge(projects_schema())
            .merge(tasks_schema())
            .merge(audit_log_schema())
    }

    /// 仅 audit_log (auth-service 用)
    pub fn audit_only() -> SchemaSet { audit_log_schema() }

    /// 仅 users (user-service 用)
    pub fn users_only() -> SchemaSet { users_schema() }

    /// pgvector 扩展 (translation-core 用)
    pub fn pgvector_ext() -> SchemaSet {
        SchemaSet::new()
            .push("CREATE EXTENSION IF NOT EXISTS vector;")
    }
}

/// users / user_profile schema (per user-service migrations)
pub fn users_schema() -> SchemaSet {
    SchemaSet::new()
        .push(r#"
            CREATE TABLE IF NOT EXISTS users_credential (
                id            UUID PRIMARY KEY,
                username      TEXT NOT NULL UNIQUE,
                email         TEXT,
                password_hash TEXT NOT NULL,
                is_active     BOOLEAN NOT NULL DEFAULT TRUE,
                created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
        "#)
        .push(r#"
            CREATE TABLE IF NOT EXISTS user_profile (
                user_id    UUID PRIMARY KEY REFERENCES users_credential(id) ON DELETE CASCADE,
                display_name TEXT,
                avatar_url   TEXT,
                locale       TEXT NOT NULL DEFAULT 'zh-CN',
                timezone     TEXT NOT NULL DEFAULT 'Asia/Shanghai',
                created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
        "#)
        .push("CREATE INDEX IF NOT EXISTS idx_users_credential_username ON users_credential(username);")
}

/// projects schema (per project-service migrations)
pub fn projects_schema() -> SchemaSet {
    SchemaSet::new()
        .push(r#"
            CREATE TABLE IF NOT EXISTS project (
                id          UUID PRIMARY KEY,
                owner_id    UUID NOT NULL,
                name        TEXT NOT NULL,
                description TEXT,
                source_lang TEXT NOT NULL,
                target_lang TEXT NOT NULL,
                status      TEXT NOT NULL DEFAULT 'draft',
                created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
        "#)
        .push("CREATE INDEX IF NOT EXISTS idx_project_owner_id ON project(owner_id);")
        .push("CREATE INDEX IF NOT EXISTS idx_project_status ON project(status);")
}

/// tasks schema (per task-service migrations)
pub fn tasks_schema() -> SchemaSet {
    SchemaSet::new()
        .push(r#"
            CREATE TABLE IF NOT EXISTS task (
                id          UUID PRIMARY KEY,
                project_id  UUID NOT NULL,
                task_type   TEXT NOT NULL,
                payload     JSONB NOT NULL,
                status      TEXT NOT NULL DEFAULT 'pending',
                attempts    INT NOT NULL DEFAULT 0,
                last_error  TEXT,
                created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
        "#)
        .push("CREATE INDEX IF NOT EXISTS idx_task_project_id ON task(project_id);")
        .push("CREATE INDEX IF NOT EXISTS idx_task_status ON task(status);")
}

/// audit_log schema (per auth-service T-01 migrations)
pub fn audit_log_schema() -> SchemaSet {
    SchemaSet::new()
        .push(r#"
            CREATE TABLE IF NOT EXISTS audit_log (
                event_id    UUID PRIMARY KEY,
                user_id     UUID,
                event_type  TEXT NOT NULL,
                outcome     TEXT NOT NULL,
                detail      JSONB,
                source_ip   INET,
                user_agent  TEXT,
                occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
        "#)
        .push("CREATE INDEX IF NOT EXISTS idx_audit_log_user_id ON audit_log(user_id);")
        .push("CREATE INDEX IF NOT EXISTS idx_audit_log_event_type ON audit_log(event_type);")
        .push("CREATE INDEX IF NOT EXISTS idx_audit_log_occurred_at ON audit_log(occurred_at);")
}

/// pgvector translation_unit schema (per translation-core)
pub fn translation_unit_schema() -> SchemaSet {
    SchemaSet::new()
        .push("CREATE EXTENSION IF NOT EXISTS vector;")
        .push(r#"
            CREATE TABLE IF NOT EXISTS translation_unit (
                id         UUID PRIMARY KEY,
                project_id UUID NOT NULL,
                source     TEXT NOT NULL,
                target     TEXT,
                embedding  vector(384),
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
        "#)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_common_has_at_least_4_tables() {
        // 4 类表 (users / projects / tasks / audit_log) 各至少 1 个 CREATE TABLE
        let s = SchemaSet::all_common();
        let create_table_count = s
            .statements
            .iter()
            .filter(|s| s.to_uppercase().contains("CREATE TABLE"))
            .count();
        assert!(create_table_count >= 4, "got {create_table_count}");
    }

    #[test]
    fn pgvector_ext_has_extension() {
        let s = SchemaSet::pgvector_ext();
        assert!(s.statements[0].to_uppercase().contains("CREATE EXTENSION"));
    }

    #[test]
    fn merge_preserves_order() {
        let a = SchemaSet::new().push("A;");
        let b = SchemaSet::new().push("B;").push("C;");
        let merged = a.merge(b);
        assert_eq!(merged.statements, vec!["A;", "B;", "C;"]);
    }

    #[test]
    fn users_schema_has_unique_constraint() {
        let s = users_schema();
        let has_unique = s
            .statements
            .iter()
            .any(|s| s.to_uppercase().contains("UNIQUE"));
        assert!(has_unique, "username 必须 UNIQUE");
    }

    #[test]
    fn audit_log_has_event_id_primary_key() {
        let s = audit_log_schema();
        let has_pk = s
            .statements
            .iter()
            .any(|s| s.to_uppercase().contains("PRIMARY KEY"));
        assert!(has_pk, "event_id 必须 PRIMARY KEY (幂等 insert 依赖)");
    }
}
