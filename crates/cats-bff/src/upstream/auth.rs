//! auth-service 上游客户端封装 (per 切片 A _slice_a_bff.md)
//!
//! 转发 POST /v1/auth/{login,refresh,logout} + GET /v1/auth/me
//! 所有调用透传 Authorization 头 (per 接口设计书 v2.0 §3 JWT 转发)

use crate::config::Config;
use crate::error::{BffError, BffResult};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// auth-service 上游客户端
#[derive(Debug, Clone)]
pub struct AuthClient {
    http: Client,
    base_url: String,
}

// ---- DTO (mirror auth-service/src/models.rs) ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub tenant_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    #[serde(default = "default_token_type")]
    pub token_type: String,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub username: String,
}

fn default_token_type() -> String {
    "Bearer".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    #[serde(default = "default_token_type")]
    pub token_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutResponse {
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeResponse {
    pub user_id: String,
    pub username: String,
    pub email: String,
}

impl AuthClient {
    pub fn new(cfg: &Config) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(cfg.upstream_timeout_secs))
            .build()
            .expect("reqwest client build");
        Self {
            http,
            base_url: cfg.auth_service_url.trim_end_matches('/').to_string(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn http(&self) -> &Client {
        &self.http
    }

    /// POST /v1/auth/login
    pub async fn login(&self, req: &LoginRequest) -> BffResult<LoginResponse> {
        let url = format!("{}/v1/auth/login", self.base_url());
        let resp = self.http().post(&url).json(req).send().await?;
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

    /// POST /v1/auth/refresh
    pub async fn refresh(&self, req: &RefreshRequest) -> BffResult<RefreshResponse> {
        let url = format!("{}/v1/auth/refresh", self.base_url());
        let resp = self.http().post(&url).json(req).send().await?;
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

    /// POST /v1/auth/logout
    pub async fn logout(&self, req: &LogoutRequest, access_token: Option<&str>) -> BffResult<LogoutResponse> {
        let url = format!("{}/v1/auth/logout", self.base_url());
        let mut rb = self.http().post(&url).json(req);
        if let Some(tok) = access_token {
            rb = rb.bearer_auth(tok);
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

    /// GET /v1/auth/me — 聚合 auth + user (per 切片 A §3: auth-service 拿到 user_id 后调 user-service)
    pub async fn me(&self, access_token: &str) -> BffResult<MeResponse> {
        let url = format!("{}/v1/auth/me", self.base_url());
        let resp = self
            .http()
            .get(&url)
            .bearer_auth(access_token)
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
