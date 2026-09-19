//! 项目管理调用封装
//!
//! 端点（per 接口设计书 §3.3 project-service）:
//! - GET  /v1/projects         — 项目列表
//! - POST /v1/projects         — 创建项目（Idempotency-Key 透传）
//!
//! MVP 客户端不实现项目详情 / 删除 / 成员管理（per apps/cats-client/TODO.md）。

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

use super::client::ApiClient;
use super::error::ApiError;

/// 项目（摘要字段）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub source_lang: Option<String>,
    #[serde(default)]
    pub target_lang: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

/// 项目列表响应
#[derive(Debug, Deserialize)]
pub struct ListProjectsResponse {
    pub projects: Vec<Project>,
    #[serde(default)]
    pub total: Option<i64>,
}

/// 调 GET /v1/projects
pub async fn list_projects(state: Arc<AppState>) -> Result<ListProjectsResponse, ApiError> {
    let client = ApiClient::new(state);
    client.get_authed("/v1/projects").await
}

/// 创建项目请求
#[derive(Debug, Serialize)]
pub struct CreateProjectRequest<'a> {
    pub name: &'a str,
    pub source_lang: &'a str,
    pub target_lang: &'a str,
}

/// 调 POST /v1/projects
///
/// 生成 UUID 作为 Idempotency-Key（per 接口设计书 §3.3 + 整体 §1.5）。
pub async fn create_project(
    state: Arc<AppState>,
    req: CreateProjectRequest<'_>,
) -> Result<Project, ApiError> {
    let client = ApiClient::new(state);
    let url = client.url("/v1/projects");
    let idem_key = Uuid::new_v4().to_string();
    let resp = client
        .state
        .http
        .post(&url)
        .header("Idempotency-Key", idem_key)
        .json(&req)
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(ApiError::from_response(resp).await);
    }
    let p: Project = super::client::ApiClient::parse_json(resp).await?;
    Ok(p)
}