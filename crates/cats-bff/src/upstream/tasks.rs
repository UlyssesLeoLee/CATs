//! task-service + translation-core 上游客户端 (per 切片 A _slice_a_bff.md)
//!
//! POST /v1/tasks — BFF 透传到 task-service
//!
//! task-service 当前是 M0 stub (per ULYS-149 §2.2), 但 BFF 业务 endpoint 不依赖
//! task-service 的实现进度 — 上游 stub 返回 502 时 BFF 包成统一错误信封

use crate::config::Config;
use crate::error::{BffError, BffResult};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct TasksClient {
    http: Client,
    base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchTaskRequest {
    pub project_id: String,
    pub file_id: String,
    pub media_type: String, // TEXT | AUDIO | VIDEO | PDF | OFFICE | GAME
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchTaskResponse {
    pub task_id: String,
    pub project_id: String,
    pub status: String, // queued | running | succeeded | failed
}

impl TasksClient {
    pub fn new(cfg: &Config) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(cfg.upstream_timeout_secs))
            .build()
            .expect("reqwest client build");
        Self {
            http,
            base_url: cfg.task_service_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn dispatch(
        &self,
        access_token: &str,
        req: &DispatchTaskRequest,
    ) -> BffResult<DispatchTaskResponse> {
        let url = format!("{}/v1/tasks", self.base_url);
        let resp = self
            .http
            .post(&url)
            .bearer_auth(access_token)
            .json(req)
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
}
