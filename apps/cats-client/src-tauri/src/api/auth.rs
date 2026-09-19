//! auth-service 调用封装
//!
//! 端点（per 接口设计书 §3.1 auth-service）:
//! - POST /v1/auth/login        — 账号密码登录，返回 access/refresh
//! - POST /v1/auth/refresh      — refresh_token 换新 access（client.rs 已内部调用）
//!
//! MVP 客户端不实现 OAuth/SSO 完整集成（per apps/cats-client/TODO.md）。

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::state::{AppState, TokenPair};

use super::client::ApiClient;
use super::error::ApiError;

/// 登录请求
#[derive(Debug, Serialize)]
pub struct LoginRequest<'a> {
    pub username: &'a str,
    pub password: &'a str,
}

/// 登录响应
#[derive(Debug, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub org_id: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
}

/// 调 /v1/auth/login, 把返回的 token 写入 AppState
pub async fn login(
    state: Arc<AppState>,
    username: &str,
    password: &str,
) -> Result<LoginResponse, ApiError> {
    let client = ApiClient::new(state.clone());
    let req = LoginRequest { username, password };
    let resp: LoginResponse = client.post_json("/v1/auth/login", &req).await?;

    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(resp.expires_in);
    state.set_token(TokenPair {
        access_token: resp.access_token.clone(),
        refresh_token: resp.refresh_token.clone(),
        expires_at,
    });

    Ok(resp)
}