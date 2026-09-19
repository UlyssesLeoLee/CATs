//! 认证相关 Tauri 命令

use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use crate::api;
use crate::state::AppState;

use super::super::api::ApiError;

/// 登录响应（前端可见字段，去除敏感信息）
#[derive(Debug, Serialize)]
pub struct AuthLoginResponse {
    pub user_id: Option<String>,
    pub org_id: Option<String>,
    pub roles: Vec<String>,
    pub access_token_expires_at: String,
}

impl From<api::auth::LoginResponse> for AuthLoginResponse {
    fn from(r: api::auth::LoginResponse) -> Self {
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(r.expires_in);
        Self {
            user_id: r.user_id,
            org_id: r.org_id,
            roles: r.roles,
            access_token_expires_at: expires_at.to_rfc3339(),
        }
    }
}

/// `auth_login(username, password)` — Tauri 命令
#[tauri::command]
pub async fn auth_login(
    state: State<'_, Arc<AppState>>,
    username: String,
    password: String,
) -> Result<AuthLoginResponse, String> {
    let arc = state.inner().clone();
    api::auth::login(arc, &username, &password)
        .await
        .map(Into::into)
        .map_err(|e| match e {
            ApiError::Server { code, message, .. } => {
                serde_json::json!({"code": code, "message": message}).to_string()
            }
            other => format!("auth_login failed: {other}"),
        })
}

/// `auth_refresh()` — 强制 refresh 当前 token
#[tauri::command]
pub async fn auth_refresh(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    // 复用 ApiClient.try_refresh 逻辑：手动构造一次空 refresh 调用
    let arc = state.inner().clone();
    let client = api::ApiClient::new(arc.clone());
    client
        .send_with_auth::<(), serde_json::Value>(reqwest::Method::POST, "/v1/auth/refresh", None)
        .await
        .map(|_| ())
        .map_err(|e| format!("auth_refresh failed: {e}"))
}