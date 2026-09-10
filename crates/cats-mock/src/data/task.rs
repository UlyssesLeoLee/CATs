//! Task factory
//!
//! 引用: 设计书 §4.1.3

use super::{pick_random, random_past_within_days, Factory, TaskStatus};
use serde::{Deserialize, Serialize};

/// Task 数据
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: uuid::Uuid,
    pub project_id: uuid::Uuid,
    pub task_type: String, // translate / asr / ocr / render ...
    pub payload: serde_json::Value,
    pub status: TaskStatus,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Task 工厂
#[derive(Debug, Clone)]
pub struct TaskFactory {
    fixed_id: Option<uuid::Uuid>,
    project_id: Option<uuid::Uuid>,
    task_type: Option<String>,
    status: Option<TaskStatus>,
    with_error: bool,
    attempts: Option<i32>,
    count: usize,
}

impl TaskFactory {
    pub fn new() -> Self {
        Self {
            fixed_id: None,
            project_id: None,
            task_type: None,
            status: None,
            with_error: false,
            attempts: None,
            count: 1,
        }
    }

    pub fn fixed_id(mut self, id: uuid::Uuid) -> Self {
        self.fixed_id = Some(id);
        self
    }

    pub fn for_project(mut self, project_id: uuid::Uuid) -> Self {
        self.project_id = Some(project_id);
        self
    }

    pub fn of_type(mut self, task_type: impl Into<String>) -> Self {
        self.task_type = Some(task_type.into());
        self
    }

    pub fn with_status(mut self, status: TaskStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// 构造一个失败任务 (status=Failed, last_error=Some)
    pub fn failed(mut self) -> Self {
        self.status = Some(TaskStatus::Failed);
        self.with_error = true;
        self.attempts = Some(3);
        self
    }

    pub fn count(mut self, n: usize) -> Self {
        self.count = n;
        self
    }

    fn build_one(&self) -> Task {
        let id = self.fixed_id.unwrap_or_else(uuid::Uuid::new_v4);
        let project_id = self.project_id.unwrap_or_else(uuid::Uuid::new_v4);
        let task_type = self
            .task_type
            .clone()
            .unwrap_or_else(|| "translate".to_string());
        let status = self.status.unwrap_or_else(|| pick_random(TaskStatus::all()));
        let attempts = self.attempts.unwrap_or(0);
        let last_error = if self.with_error {
            Some("simulated worker failure: timeout after 30s".to_string())
        } else {
            None
        };
        let created_at = random_past_within_days(7);

        let payload = serde_json::json!({
            "task_type": task_type,
            "source": "Hello, world.",
            "target": null,
        });

        Task {
            id,
            project_id,
            task_type,
            payload,
            status,
            attempts,
            last_error,
            created_at,
            updated_at: created_at,
        }
    }
}

impl Default for TaskFactory {
    fn default() -> Self { Self::new() }
}

impl Factory for TaskFactory {
    type Output = Task;
    fn build(&self) -> Task { self.build_one() }
    fn build_many(&self, count: usize) -> Vec<Task> {
        (0..count).map(|_| self.build_one()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_task_translate() {
        let t = TaskFactory::new().build();
        assert_eq!(t.task_type, "translate");
        assert_eq!(t.attempts, 0);
        assert!(t.last_error.is_none());
    }

    #[test]
    fn failed_task_has_error_and_attempts() {
        let t = TaskFactory::new().failed().build();
        assert_eq!(t.status, TaskStatus::Failed);
        assert!(t.last_error.is_some());
        assert_eq!(t.attempts, 3);
    }

    #[test]
    fn of_type_works() {
        let t = TaskFactory::new().of_type("asr").build();
        assert_eq!(t.task_type, "asr");
    }

    #[test]
    fn for_project_uses_given() {
        let pid = uuid::Uuid::new_v4();
        let t = TaskFactory::new().for_project(pid).build();
        assert_eq!(t.project_id, pid);
    }

    #[test]
    fn batch_count() {
        let tasks = TaskFactory::new().build_many(7);
        assert_eq!(tasks.len(), 7);
    }

    #[test]
    fn status_string_round_trip() {
        for s in TaskStatus::all() {
            let t = TaskFactory::new().with_status(*s).build();
            assert_eq!(t.status.as_str(), s.as_str());
        }
    }
}
