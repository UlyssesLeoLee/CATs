//! Seed 数据: 把 data factory 产出的对象转成 SQL INSERT
//!
//! 引用: 设计书 §4.2.2

use crate::data::{AuditEvent, Project, Task, User};

/// 一组 INSERT SQL (待执行的语句, 已转义)
#[derive(Debug, Clone, Default)]
pub struct SeedSet {
    /// 描述 (便于调试)
    pub label: String,
    /// 已转义的参数化 SQL (per statement: sql + params)
    pub entries: Vec<SeedEntry>,
}

/// 单条 seed
#[derive(Debug, Clone)]
pub struct SeedEntry {
    /// 表名
    pub table: String,
    /// INSERT SQL (含占位符 $1, $2, ...)
    pub sql: String,
    /// 绑定参数 (按顺序对应 $1, $2, ...)
    pub params: Vec<SeedParam>,
}

/// 参数 (简单类型, 实际由调用方转 sqlx::types)
#[derive(Debug, Clone)]
pub enum SeedParam {
    Uuid(uuid::Uuid),
    Text(String),
    Bool(bool),
    Int(i32),
    Json(serde_json::Value),
    Timestamp(chrono::DateTime<chrono::Utc>),
    OptUuid(Option<uuid::Uuid>),
    OptText(Option<String>),
    OptJson(Option<serde_json::Value>),
}

impl SeedSet {
    /// 空
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), entries: vec![] }
    }

    /// 加一条 entry
    pub fn push(mut self, entry: SeedEntry) -> Self {
        self.entries.push(entry);
        self
    }

    /// 总条数
    pub fn total(&self) -> usize { self.entries.len() }
}

// =====================================================================
// 预制 seed 集
// =====================================================================

/// 1 个默认 user (admin)
pub fn users_default() -> SeedSet {
    let u = User {
        id: uuid::Uuid::new_v4(),
        username: "admin".to_string(),
        email: "admin@example.com".to_string(),
        password_hash: "$argon2id$v=19$m=19456,t=2,p=1$mock$mock".to_string(),
        is_active: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    users_from(&[u])
}

/// 多个 user (批量)
pub fn users_from(users: &[User]) -> SeedSet {
    let entries = users
        .iter()
        .map(|u| SeedEntry {
            table: "users_credential".to_string(),
            sql: r#"
                INSERT INTO users_credential
                    (id, username, email, password_hash, is_active, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#.to_string(),
            params: vec![
                SeedParam::Uuid(u.id),
                SeedParam::Text(u.username.clone()),
                SeedParam::OptText(Some(u.email.clone())),
                SeedParam::Text(u.password_hash.clone()),
                SeedParam::Bool(u.is_active),
                SeedParam::Timestamp(u.created_at),
                SeedParam::Timestamp(u.updated_at),
            ],
        })
        .collect();

    SeedSet { label: format!("users(count={})", users.len()), entries }
}

/// 1 个默认 project (归属 admin)
pub fn projects_default() -> SeedSet {
    let owner = uuid::Uuid::new_v4();
    let p = Project {
        id: uuid::Uuid::new_v4(),
        owner_id: owner,
        name: "Demo Project".to_string(),
        description: Some("seed project for tests".to_string()),
        source_lang: "zh-CN".to_string(),
        target_lang: "en-US".to_string(),
        status: crate::data::ProjectStatus::Active,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    projects_from(&[p])
}

/// 多个 project (批量)
pub fn projects_from(projects: &[Project]) -> SeedSet {
    let entries = projects
        .iter()
        .map(|p| SeedEntry {
            table: "project".to_string(),
            sql: r#"
                INSERT INTO project
                    (id, owner_id, name, description, source_lang, target_lang, status, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#.to_string(),
            params: vec![
                SeedParam::Uuid(p.id),
                SeedParam::Uuid(p.owner_id),
                SeedParam::Text(p.name.clone()),
                SeedParam::OptText(p.description.clone()),
                SeedParam::Text(p.source_lang.clone()),
                SeedParam::Text(p.target_lang.clone()),
                SeedParam::Text(p.status.as_str().to_string()),
                SeedParam::Timestamp(p.created_at),
                SeedParam::Timestamp(p.updated_at),
            ],
        })
        .collect();

    SeedSet { label: format!("projects(count={})", projects.len()), entries }
}

/// 1 个默认 task
pub fn tasks_default() -> SeedSet {
    let t = Task {
        id: uuid::Uuid::new_v4(),
        project_id: uuid::Uuid::new_v4(),
        task_type: "translate".to_string(),
        payload: serde_json::json!({"source": "hello"}),
        status: crate::data::TaskStatus::Pending,
        attempts: 0,
        last_error: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    tasks_from(&[t])
}

/// 多个 task (批量)
pub fn tasks_from(tasks: &[Task]) -> SeedSet {
    let entries = tasks
        .iter()
        .map(|t| SeedEntry {
            table: "task".to_string(),
            sql: r#"
                INSERT INTO task
                    (id, project_id, task_type, payload, status, attempts, last_error, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#.to_string(),
            params: vec![
                SeedParam::Uuid(t.id),
                SeedParam::Uuid(t.project_id),
                SeedParam::Text(t.task_type.clone()),
                SeedParam::Json(t.payload.clone()),
                SeedParam::Text(t.status.as_str().to_string()),
                SeedParam::Int(t.attempts),
                SeedParam::OptText(t.last_error.clone()),
                SeedParam::Timestamp(t.created_at),
                SeedParam::Timestamp(t.updated_at),
            ],
        })
        .collect();

    SeedSet { label: format!("tasks(count={})", tasks.len()), entries }
}

/// 1 个 audit event
pub fn audit_events_default() -> SeedSet {
    let e = AuditEvent {
        event_id: uuid::Uuid::new_v4(),
        user_id: Some(uuid::Uuid::new_v4()),
        event_type: "login".to_string(),
        outcome: crate::data::AuditOutcome::Success,
        detail: None,
        source_ip: Some("127.0.0.1".to_string()),
        user_agent: Some("cats-mock/1.0".to_string()),
        occurred_at: chrono::Utc::now(),
    };
    audit_events_from(&[e])
}

/// 多个 audit event (批量)
pub fn audit_events_from(events: &[AuditEvent]) -> SeedSet {
    let entries = events
        .iter()
        .map(|e| SeedEntry {
            table: "audit_log".to_string(),
            sql: r#"
                INSERT INTO audit_log
                    (event_id, user_id, event_type, outcome, detail, source_ip, user_agent, occurred_at)
                VALUES ($1, $2, $3, $4, $5, $6::inet, $7, $8)
                ON CONFLICT (event_id) DO NOTHING
            "#.to_string(),
            params: vec![
                SeedParam::Uuid(e.event_id),
                SeedParam::OptUuid(e.user_id),
                SeedParam::Text(e.event_type.clone()),
                SeedParam::Text(e.outcome.as_str().to_string()),
                SeedParam::OptJson(e.detail.clone()),
                SeedParam::OptText(e.source_ip.clone()),
                SeedParam::OptText(e.user_agent.clone()),
                SeedParam::Timestamp(e.occurred_at),
            ],
        })
        .collect();

    SeedSet { label: format!("audit_events(count={})", events.len()), entries }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::audit::AuditOutcome;

    #[test]
    fn users_default_has_one_entry() {
        let s = users_default();
        assert_eq!(s.total(), 1);
        assert_eq!(s.entries[0].table, "users_credential");
    }

    #[test]
    fn users_from_preserves_count() {
        use crate::data::Factory;
        let us = crate::data::UserFactory::new().build_many(5);
        let s = users_from(&us);
        assert_eq!(s.total(), 5);
    }

    #[test]
    fn projects_default_has_one_entry() {
        let s = projects_default();
        assert_eq!(s.total(), 1);
        assert_eq!(s.entries[0].table, "project");
    }

    #[test]
    fn tasks_default_has_one_entry() {
        let s = tasks_default();
        assert_eq!(s.total(), 1);
    }

    #[test]
    fn audit_events_default_has_one_entry() {
        let s = audit_events_default();
        assert_eq!(s.total(), 1);
        assert_eq!(s.entries[0].table, "audit_log");
        // ON CONFLICT DO NOTHING 是审计幂等关键
        assert!(s.entries[0].sql.to_uppercase().contains("ON CONFLICT"));
    }

    #[test]
    fn seed_entry_param_count_matches_sql_placeholders() {
        // 防 SQL placeholder 数量 vs params 数量不匹配
        let s = users_default();
        let entry = &s.entries[0];
        // 占位符格式: $1, $2, ... 简单计数
        let dollar_count = entry.sql.matches('$').count();
        // $1..$7 共 7 个占位符
        assert!(dollar_count >= 7, "expected ≥7 placeholders, got {dollar_count}");
        // param count 必须与占位符数量匹配 (这里手工对齐, 不动态解析)
        assert_eq!(entry.params.len(), 7, "params must match placeholders");
    }

    #[test]
    fn empty_seed_has_zero_total() {
        let s = SeedSet::new("empty");
        assert_eq!(s.total(), 0);
    }

    #[test]
    fn audit_outcome_round_trip() {
        let e = AuditEvent {
            event_id: uuid::Uuid::new_v4(),
            user_id: None,
            event_type: "x".to_string(),
            outcome: AuditOutcome::Failure,
            detail: None,
            source_ip: None,
            user_agent: None,
            occurred_at: chrono::Utc::now(),
        };
        let s = audit_events_from(&[e]);
        let outcome_param = &s.entries[0].params[3];
        match outcome_param {
            SeedParam::Text(t) => assert_eq!(t, "failure"),
            _ => panic!("expected Text"),
        }
    }
}
