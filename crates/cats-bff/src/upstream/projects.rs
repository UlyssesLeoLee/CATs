//! project-service 上游客户端封装 (per 切片 A _slice_a_bff.md)
//!
//! 转发 GET /v1/projects + POST /v1/projects
//! Authorization 头透传 (BFF 已通过 RBAC 校验)

use crate::config::Config;
use crate::error::{BffError, BffResult};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ProjectsClient {
    http: Client,
    base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProjectsQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

fn default_page() -> u32 {
    1
}
fn default_page_size() -> u32 {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub source_lang: String,
    pub target_lang: String,
    #[serde(default)]
    pub tm_id: Option<String>,
    #[serde(default)]
    pub termbase_id: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProjectsResponse {
    pub items: Vec<Project>,
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub source_lang: String,
    pub target_lang: String,
    #[serde(default)]
    pub tm_id: Option<String>,
    #[serde(default)]
    pub termbase_id: Option<String>,
}

impl ProjectsClient {
    pub fn new(cfg: &Config) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(cfg.upstream_timeout_secs))
            .build()
            .expect("reqwest client build");
        Self {
            http,
            base_url: cfg.project_service_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn list(
        &self,
        access_token: &str,
        q: &ListProjectsQuery,
    ) -> BffResult<ListProjectsResponse> {
        let url = format!("{}/v1/projects", self.base_url);
        let resp = self
            .http
            .get(&url)
            .bearer_auth(access_token)
            .query(q)
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(BffError::UpstreamError {
                status: status.as_u16(),
                body,
            });
        }
        Ok(serde_json::from_str(&body)?)
    }

    pub async fn create(
        &self,
        access_token: &str,
        req: &CreateProjectRequest,
        idempotency_key: Option<&str>,
    ) -> BffResult<Project> {
        let url = format!("{}/v1/projects", self.base_url);
        let mut rb = self.http.post(&url).bearer_auth(access_token).json(req);
        if let Some(k) = idempotency_key {
            rb = rb.header("Idempotency-Key", k);
        }
        let resp = rb.send().await?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(BffError::UpstreamError {
                status: status.as_u16(),
                body,
            });
        }
        Ok(serde_json::from_str(&body)?)
    }
}
