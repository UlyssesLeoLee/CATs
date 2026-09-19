//! 业务模型 + API DTO (project-service)
//!
//! 引用: doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md §6.3
//! 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §6.4
//! 引用: doc/05-其他/管理/CATs_错误码表_v1.0.md §3
//!
//! 错误码 (per 错误码表 v1.0 + 接口设计 §6.3.4):
//! - 200 / 201 成功
//! - 400 invalid_request (字段空 / 长度超限 / lang 格式)
//! - 403 forbidden (跨 org_id 访问)
//! - 404 resource_not_found / project_not_found
//! - 409 project_name_conflict (同 org 下 name 唯一约束)
//! - 500 server_error

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 项目主表 row (per 数据库 v2.0 §6.4 projects)
#[derive(Debug, Clone, FromRow)]
pub struct ProjectRow {
    pub id: uuid::Uuid,
    pub org_id: uuid::Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
    pub created_by: uuid::Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 项目成员 row
#[derive(Debug, Clone, FromRow)]
pub struct ProjectMemberRow {
    pub project_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub role: String,
    pub added_at: chrono::DateTime<chrono::Utc>,
}

/// POST /v1/projects 请求 (per 接口设计 §6.3.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub org_id: uuid::Uuid,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub source_lang: Option<String>,
    #[serde(default)]
    pub target_lang: Option<String>,
}

/// PUT /v1/projects/{id} 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProjectRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

/// GET /v1/projects/{id} 响应 + POST /v1/projects 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectResponse {
    pub id: uuid::Uuid,
    pub org_id: uuid::Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
    pub created_by: uuid::Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<ProjectRow> for ProjectResponse {
    fn from(p: ProjectRow) -> Self {
        Self {
            id: p.id,
            org_id: p.org_id,
            name: p.name,
            description: p.description,
            status: p.status,
            source_lang: p.source_lang,
            target_lang: p.target_lang,
            created_by: p.created_by,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

/// GET /v1/projects?org_id=... 列表分页响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectListResponse {
    pub items: Vec<ProjectResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// POST /v1/projects/{id}/members 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddMemberRequest {
    pub user_id: uuid::Uuid,
    #[serde(default = "default_member_role")]
    pub role: String,
}

fn default_member_role() -> String {
    "member".to_string()
}

/// 成员响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberResponse {
    pub project_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub role: String,
    pub added_at: chrono::DateTime<chrono::Utc>,
}

impl From<ProjectMemberRow> for MemberResponse {
    fn from(m: ProjectMemberRow) -> Self {
        Self {
            project_id: m.project_id,
            user_id: m.user_id,
            role: m.role,
            added_at: m.added_at,
        }
    }
}

// cats_common::ErrorBody 公开,通过 cats_common::ErrorBody 使用 (无私有 re-export)