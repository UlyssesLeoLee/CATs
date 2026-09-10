//! Project factory
//!
//! 引用: 设计书 §4.1.2

use super::{pick_random, random_past_within_days, Factory, ProjectStatus};
use fake::faker::lorem::en::Sentence;
use fake::Fake;
use serde::{Deserialize, Serialize};

/// Project 数据
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: uuid::Uuid,
    pub owner_id: uuid::Uuid,
    pub name: String,
    pub description: Option<String>,
    pub source_lang: String,
    pub target_lang: String,
    pub status: ProjectStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Project 工厂
#[derive(Debug, Clone)]
pub struct ProjectFactory {
    fixed_id: Option<uuid::Uuid>,
    owner_id: Option<uuid::Uuid>,
    status: Option<ProjectStatus>,
    source_lang: Option<String>,
    target_lang: Option<String>,
    count: usize,
}

impl ProjectFactory {
    pub fn new() -> Self {
        Self {
            fixed_id: None,
            owner_id: None,
            status: None,
            source_lang: None,
            target_lang: None,
            count: 1,
        }
    }

    pub fn fixed_id(mut self, id: uuid::Uuid) -> Self {
        self.fixed_id = Some(id);
        self
    }

    pub fn owned_by(mut self, owner_id: uuid::Uuid) -> Self {
        self.owner_id = Some(owner_id);
        self
    }

    pub fn with_status(mut self, status: ProjectStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn lang_pair(mut self, src: impl Into<String>, tgt: impl Into<String>) -> Self {
        self.source_lang = Some(src.into());
        self.target_lang = Some(tgt.into());
        self
    }

    pub fn count(mut self, n: usize) -> Self {
        self.count = n;
        self
    }

    fn build_one(&self) -> Project {
        let id = self.fixed_id.unwrap_or_else(uuid::Uuid::new_v4);
        let owner_id = self.owner_id.unwrap_or_else(uuid::Uuid::new_v4);
        let name: String = Sentence(3..6).fake();
        let description: Option<String> = Some(Sentence(8..15).fake());
        let source_lang = self.source_lang.clone().unwrap_or_else(|| "zh-CN".to_string());
        let target_lang = self.target_lang.clone().unwrap_or_else(|| "en-US".to_string());
        let status = self.status.unwrap_or(ProjectStatus::Draft);
        let created_at = random_past_within_days(30);

        Project {
            id,
            owner_id,
            name,
            description,
            source_lang,
            target_lang,
            status,
            created_at,
            updated_at: created_at,
        }
    }
}

impl Default for ProjectFactory {
    fn default() -> Self { Self::new() }
}

impl Factory for ProjectFactory {
    type Output = Project;
    fn build(&self) -> Project { self.build_one() }
    fn build_many(&self, count: usize) -> Vec<Project> {
        (0..count).map(|_| self.build_one()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_project_is_draft() {
        let p = ProjectFactory::new().build();
        assert_eq!(p.status, ProjectStatus::Draft);
        assert_eq!(p.source_lang, "zh-CN");
        assert_eq!(p.target_lang, "en-US");
        assert!(!p.name.is_empty());
    }

    #[test]
    fn owned_by_uses_given_owner() {
        let owner = uuid::Uuid::new_v4();
        let p = ProjectFactory::new().owned_by(owner).build();
        assert_eq!(p.owner_id, owner);
    }

    #[test]
    fn with_status_applied() {
        let p = ProjectFactory::new().with_status(ProjectStatus::Archived).build();
        assert_eq!(p.status, ProjectStatus::Archived);
    }

    #[test]
    fn lang_pair() {
        let p = ProjectFactory::new().lang_pair("ja-JP", "ko-KR").build();
        assert_eq!(p.source_lang, "ja-JP");
        assert_eq!(p.target_lang, "ko-KR");
    }

    #[test]
    fn batch_count() {
        let ps = ProjectFactory::new().build_many(5);
        assert_eq!(ps.len(), 5);
    }

    #[test]
    fn status_string_round_trip() {
        for s in ProjectStatus::all() {
            let p = ProjectFactory::new().with_status(*s).build();
            assert_eq!(p.status.as_str(), s.as_str());
        }
    }
}
