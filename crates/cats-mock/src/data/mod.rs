//! 业务对象 factory
//!
//! 引用: 设计书 §4.1
//!
//! 4 类核心业务对象: User / Project / Task / AuditEvent
//! 每个对象提供:
//! - 默认构造: `UserFactory::new().build()` (走 fake crate 生成真实感数据)
//! - 固定构造: `UserFactory::fixed_id(uuid)` (可重现)
//! - 批量构造: `UserFactory::new().count(10).build_many()`
//! - 校验构造: `UserFactory::invalid_email()` (边界用例)

use chrono::{DateTime, Utc};
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};

pub mod audit;
pub mod project;
pub mod task;
pub mod user;

pub use audit::*;
pub use project::*;
pub use task::*;
pub use user::*;

/// 工厂 trait (统一接口, 防每个对象都写一份 boilerplate)
pub trait Factory: Sized + Clone {
    /// 输出类型
    type Output;

    /// 用默认配置构造一个
    fn build(&self) -> Self::Output;

    /// 批量构造 (默认走随机种子, 可重现场景改用 `with_seed`)
    fn build_many(&self, count: usize) -> Vec<Self::Output> {
        (0..count).map(|_| self.build()).collect()
    }
}

// =====================================================================
// 通用枚举: 业务状态机
// =====================================================================

/// 项目状态 (per project-service 状态机)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Draft,
    Active,
    Archived,
    Deleted,
}

impl ProjectStatus {
    /// 所有状态 (供 property-based 测试遍历用)
    pub fn all() -> &'static [ProjectStatus] {
        &[Self::Draft, Self::Active, Self::Archived, Self::Deleted]
    }

    /// 字符串表示 (与 DB enum 同步)
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Archived => "archived",
            Self::Deleted => "deleted",
        }
    }
}

impl Default for ProjectStatus {
    fn default() -> Self { Self::Draft }
}

/// 任务状态 (per task-service 状态机)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl TaskStatus {
    pub fn all() -> &'static [TaskStatus] {
        &[
            Self::Pending,
            Self::Queued,
            Self::Running,
            Self::Succeeded,
            Self::Failed,
            Self::Cancelled,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl Default for TaskStatus {
    fn default() -> Self { Self::Pending }
}

// =====================================================================
// 通用工具: 随机时间戳 / 边界值
// =====================================================================

/// 当前 UTC 时间 (便于在测试中重写 mock)
pub fn now_utc() -> DateTime<Utc> { Utc::now() }

/// 随机过去 N 天内的时间戳
pub fn random_past_within_days(days: i64) -> DateTime<Utc> {
    let secs = rand::thread_rng().gen_range(0..(days * 86_400));
    Utc::now() - chrono::Duration::seconds(secs)
}

/// 从切片随机选一个
pub fn pick_random<T: Clone>(items: &[T]) -> T {
    items
        .choose(&mut rand::thread_rng())
        .expect("slice must be non-empty")
        .clone()
}

/// 边界 email (用于校验失败场景)
pub fn invalid_emails() -> &'static [&'static str] {
    &[
        "",                  // empty
        "no-at-sign",        // 0 @s
        "no-domain",         // 0 @s
        "no-domain@",        // 1 @, 但空 domain (用 RFC 简化校验)
        "double@@at.com",    // 2 @s
        "triple@@@at.com",   // 3 @s
    ]
}

/// 边界 password (用于密码强度校验)
pub fn weak_passwords() -> &'static [&'static str] {
    &["", "123", "password", "short", "         "]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_status_all_round_trip() {
        for s in ProjectStatus::all() {
            assert!(!s.as_str().is_empty());
        }
    }

    #[test]
    fn task_status_all_round_trip() {
        for s in TaskStatus::all() {
            assert!(!s.as_str().is_empty());
        }
    }

    #[test]
    fn pick_random_returns_element() {
        let items = vec![1, 2, 3, 4, 5];
        for _ in 0..20 {
            let v = pick_random(&items);
            assert!(items.contains(&v));
        }
    }

    #[test]
    fn invalid_emails_are_incorrect() {
        // invalid_emails 中每条都不能是合法 email
        // 判定 invalid: 空 / 0 个 @ / ≥2 个 @ / 含空格 / 头尾 @ / 长度 < 5
        for e in invalid_emails() {
            let at_count = e.matches('@').count();
            let has_space = e.contains(' ');
            let starts_with_at = e.starts_with('@');
            let ends_with_at = e.ends_with('@');
            let is_invalid = e.is_empty() || at_count != 1 || has_space || starts_with_at || ends_with_at;
            assert!(is_invalid, "expected invalid, got: {e}");
        }
    }
}
