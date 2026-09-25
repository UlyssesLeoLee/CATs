//! 业务模型 + API DTO
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1 (project-service)
//! 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md (projects 表)
//! 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.2 (project_db 接口契约 v1.0.0)
//! 引用: doc/05-其他/管理/CATs_错误码表_v1.0.md §3-§4 (error enum 复用)
//! 引用: ULYS-150 切片 B-1 任务书 §"数据模型" + §"5 endpoint"
//!
//! 设计选择 (per 缺标比错标安全):
//! - 接口设计书 v2.0 整份不存在 (per Sprint 1 §6.1 缺口)
//!   → DTO 设计基于微服务架构书 §4.1 + Baseline §5.2 端点清单
//!   → 详细 request/response schema 留 T-07 启动时升接口设计书 v2.0
//! - ErrorBody 直接复用 user-service 错误码表 §3 枚举值
//!   → 不重复定义, 错误码一致性通过引用错误码表 v1.0 保证

use serde::{Deserialize, Serialize};

// =====================================================================
// 1. 项目状态 (per 数据库设计书 v2.0 + 切片 B-1 §数据模型 CHECK 约束)
// =====================================================================

/// 项目状态 (per projects.status CHECK 约束)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Active,
    Archived,
    Completed,
}

impl ProjectStatus {
    /// DB 字符串 (snake_case)
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectStatus::Active => "active",
            ProjectStatus::Archived => "archived",
            ProjectStatus::Completed => "completed",
        }
    }

    /// 从 DB 字符串解析 (找不到则 None, 让 handler 返回 400)
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "active" => Some(ProjectStatus::Active),
            "archived" => Some(ProjectStatus::Archived),
            "completed" => Some(ProjectStatus::Completed),
            _ => None,
        }
    }
}

// =====================================================================
// 2. DB 实体: project_db.projects
// =====================================================================

/// DB 实体: project_db.projects
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Project {
    pub id: uuid::Uuid,
    pub workspace_id: uuid::Uuid,
    pub name: String,
    pub source_lang: String,
    pub target_lang: String,
    pub status: String,
    pub owner_user_id: uuid::Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Project {
    /// 转为 API 响应 DTO
    pub fn status_enum(&self) -> ProjectStatus {
        ProjectStatus::parse(&self.status).unwrap_or(ProjectStatus::Active)
    }
}

// =====================================================================
// 3. API DTO (per 接口设计书 v2.0 §/projects)
// =====================================================================

/// GET /v1/projects/{id} 响应 (per Baseline §5.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetProjectResponse {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub source_lang: String,
    pub target_lang: String,
    pub status: ProjectStatus,
    pub owner_user_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<Project> for GetProjectResponse {
    fn from(p: Project) -> Self {
        // 先把 status 解析走 (借用 p.status), 再 move 其余字段
        let status = p.status_enum();
        Self {
            id: p.id.to_string(),
            workspace_id: p.workspace_id.to_string(),
            name: p.name,
            source_lang: p.source_lang,
            target_lang: p.target_lang,
            status,
            owner_user_id: p.owner_user_id.to_string(),
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

/// GET /v1/projects 列表响应项 (per Baseline §5.2 + 切片 B-1 §5 endpoint list)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProjectsItem {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub source_lang: String,
    pub target_lang: String,
    pub status: ProjectStatus,
    pub owner_user_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<Project> for ListProjectsItem {
    fn from(p: Project) -> Self {
        let status = p.status_enum();
        Self {
            id: p.id.to_string(),
            workspace_id: p.workspace_id.to_string(),
            name: p.name,
            source_lang: p.source_lang,
            target_lang: p.target_lang,
            status,
            owner_user_id: p.owner_user_id.to_string(),
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

/// GET /v1/projects 列表响应 (per Batch metadata: total + items, 简化版 list 包装)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProjectsResponse {
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub items: Vec<ListProjectsItem>,
}

/// POST /v1/projects 请求 (per Baseline §5.2 + 切片 B-1 §5 endpoint)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub workspace_id: uuid::Uuid,
    pub name: String,
    pub source_lang: String,
    pub target_lang: String,
    pub owner_user_id: uuid::Uuid,
}

/// PATCH /v1/projects/{id} 请求 (per Baseline §5.2, 部分字段更新)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateProjectRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_lang: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_lang: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ProjectStatus>,
}

/// GET /v1/projects 查询参数 (per 切片 B-1 §5 endpoint 列表)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProjectsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<uuid::Uuid>,
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    20
}

impl Default for ListProjectsQuery {
    fn default() -> Self {
        Self {
            workspace_id: None,
            page: default_page(),
            page_size: default_page_size(),
        }
    }
}

/// 错误响应 (与 user-service 错误码表 §3 枚举值保持一致, per 错误码表 v1.0 §6.2 跨服务一致性)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_roundtrip() {
        for s in [
            ProjectStatus::Active,
            ProjectStatus::Archived,
            ProjectStatus::Completed,
        ] {
            assert_eq!(ProjectStatus::parse(s.as_str()), Some(s));
        }
        assert_eq!(ProjectStatus::parse("unknown"), None);
    }

    #[test]
    fn status_serializes_as_snake_case() {
        let json = serde_json::to_string(&ProjectStatus::Active).unwrap();
        assert_eq!(json, "\"active\"");
        let json = serde_json::to_string(&ProjectStatus::Completed).unwrap();
        assert_eq!(json, "\"completed\"");
    }

    #[test]
    fn list_query_defaults() {
        let q = ListProjectsQuery::default();
        assert_eq!(q.page, 1);
        assert_eq!(q.page_size, 20);
        assert!(q.workspace_id.is_none());
    }
}
