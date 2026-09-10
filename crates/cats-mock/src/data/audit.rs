//! AuditEvent factory
//!
//! 引用: 设计书 §4.1.4
//!
//! AuditEvent 字段对齐 auth-service::models::AuditEvent

use super::{random_past_within_days, Factory};
use serde::{Deserialize, Serialize};

/// Audit 事件结果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    Success,
    Failure,
}

impl AuditOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
        }
    }
}

impl Default for AuditOutcome {
    fn default() -> Self { Self::Success }
}

/// AuditEvent 数据
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_id: uuid::Uuid,
    pub user_id: Option<uuid::Uuid>,
    pub event_type: String,
    pub outcome: AuditOutcome,
    pub detail: Option<serde_json::Value>,
    pub source_ip: Option<String>,
    pub user_agent: Option<String>,
    pub occurred_at: chrono::DateTime<chrono::Utc>,
}

/// AuditEvent 工厂
#[derive(Debug, Clone)]
pub struct AuditEventFactory {
    fixed_event_id: Option<uuid::Uuid>,
    user_id: Option<uuid::Uuid>,
    event_type: Option<String>,
    outcome: Option<AuditOutcome>,
    with_detail: bool,
    source_ip: Option<String>,
    user_agent: Option<String>,
    count: usize,
}

impl AuditEventFactory {
    pub fn new() -> Self {
        Self {
            fixed_event_id: None,
            user_id: None,
            event_type: None,
            outcome: None,
            with_detail: false,
            source_ip: None,
            user_agent: None,
            count: 1,
        }
    }

    pub fn fixed_event_id(mut self, id: uuid::Uuid) -> Self {
        self.fixed_event_id = Some(id);
        self
    }

    pub fn by_user(mut self, user_id: uuid::Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn of_type(mut self, event_type: impl Into<String>) -> Self {
        self.event_type = Some(event_type.into());
        self
    }

    pub fn failed(mut self) -> Self {
        self.outcome = Some(AuditOutcome::Failure);
        self
    }

    pub fn with_detail(mut self) -> Self {
        self.with_detail = true;
        self
    }

    pub fn from_ip(mut self, ip: impl Into<String>) -> Self {
        self.source_ip = Some(ip.into());
        self
    }

    pub fn with_user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    pub fn count(mut self, n: usize) -> Self {
        self.count = n;
        self
    }

    fn build_one(&self) -> AuditEvent {
        let event_id = self.fixed_event_id.unwrap_or_else(uuid::Uuid::new_v4);
        let event_type = self
            .event_type
            .clone()
            .unwrap_or_else(|| "login".to_string());
        let outcome = self.outcome.unwrap_or_default();
        let detail = if self.with_detail {
            Some(serde_json::json!({"reason": "test"}))
        } else {
            None
        };
        let source_ip = self
            .source_ip
            .clone()
            .unwrap_or_else(|| "127.0.0.1".to_string());
        let user_agent = self
            .user_agent
            .clone()
            .unwrap_or_else(|| "cats-mock/1.0".to_string());

        AuditEvent {
            event_id,
            user_id: self.user_id,
            event_type,
            outcome,
            detail,
            source_ip: Some(source_ip),
            user_agent: Some(user_agent),
            occurred_at: random_past_within_days(1),
        }
    }
}

impl Default for AuditEventFactory {
    fn default() -> Self { Self::new() }
}

impl Factory for AuditEventFactory {
    type Output = AuditEvent;
    fn build(&self) -> AuditEvent { self.build_one() }
    fn build_many(&self, count: usize) -> Vec<AuditEvent> {
        (0..count).map(|_| self.build_one()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_event_is_login_success() {
        let e = AuditEventFactory::new().build();
        assert_eq!(e.event_type, "login");
        assert_eq!(e.outcome, AuditOutcome::Success);
        assert!(e.detail.is_none());
    }

    #[test]
    fn failed_event() {
        let e = AuditEventFactory::new().failed().of_type("login_failed").build();
        assert_eq!(e.outcome, AuditOutcome::Failure);
        assert_eq!(e.event_type, "login_failed");
    }

    #[test]
    fn by_user() {
        let uid = uuid::Uuid::new_v4();
        let e = AuditEventFactory::new().by_user(uid).build();
        assert_eq!(e.user_id, Some(uid));
    }

    #[test]
    fn with_detail_populates() {
        let e = AuditEventFactory::new().with_detail().build();
        assert!(e.detail.is_some());
    }

    #[test]
    fn custom_source_ip_and_ua() {
        let e = AuditEventFactory::new()
            .from_ip("10.0.0.1")
            .with_user_agent("Mozilla/5.0")
            .build();
        assert_eq!(e.source_ip.as_deref(), Some("10.0.0.1"));
        assert_eq!(e.user_agent.as_deref(), Some("Mozilla/5.0"));
    }

    #[test]
    fn batch_count() {
        let es = AuditEventFactory::new().build_many(3);
        assert_eq!(es.len(), 3);
    }

    #[test]
    fn outcome_string_round_trip() {
        // Success
        let e1 = AuditEventFactory::new().build();
        assert_eq!(e1.outcome.as_str(), "success");
        // Failure
        let e2 = AuditEventFactory::new().failed().build();
        assert_eq!(e2.outcome.as_str(), "failure");
        // 通用: as_str 非空
        assert!(!AuditOutcome::Success.as_str().is_empty());
        assert!(!AuditOutcome::Failure.as_str().is_empty());
    }
}
