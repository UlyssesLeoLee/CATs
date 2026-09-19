//! HTTP 客户端封装（reqwest + JWT 注入 + 401 自动 refresh）
//!
//! 设计:
//! - 复用同一个 `reqwest::Client`（连接池）
//! - `bff_base` 持有 BFF 根路径（https://api.cats.internal）
//! - 调用 `with_auth` 时自动注入 `Authorization: Bearer <access_token>`,
//!   若响应 401 则用 refresh_token 调 /v1/auth/refresh, 重试一次
//! - 所有非 2xx 统一解析成 `ApiError::Server`
//!
//! MVP 简化:
//! - 不实现并发 refresh 锁（MVP 阶段用户操作串行；M1 阶段加 Mutex<()>
//!   防止 refresh 风暴）

use std::sync::Arc;

use reqwest::{Client, Method, RequestBuilder, StatusCode};
use serde::de::DeserializeOwned;

use crate::state::{AppState, TokenPair};

use super::error::ApiError;

/// API 客户端（持有 AppState 弱引用，便于传入子模块调用）
#[derive(Clone)]
pub struct ApiClient {
    state: Arc<AppState>,
}

impl ApiClient {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// 拼接 URL: `<bff_base>/<path>`
    pub fn url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.state.bff_base.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    /// 无认证 GET
    pub async fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, ApiError> {
        let url = self.url(path);
        let resp = self.state.http.get(&url).send().await?;
        Self::handle(resp).await
    }

    /// POST JSON（带 JWT）
    pub async fn post_json<Req: serde::Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &Req,
    ) -> Result<T, ApiError> {
        self.send_with_auth(Method::POST, path, Some(body)).await
    }

    /// GET（带 JWT）
    pub async fn get_authed<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        self.send_with_auth::<(), T>(Method::GET, path, None).await
    }

    /// 通用带认证请求（401 自动 refresh + 重试一次）
    pub(crate) async fn send_with_auth<Req: serde::Serialize, T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&Req>,
    ) -> Result<T, ApiError> {
        let url = self.url(path);
        let req = self.build_request(&method, &url, body)?;
        let resp = req.send().await?;

        if resp.status() == StatusCode::UNAUTHORIZED {
            // 401 → 尝试 refresh
            tracing::warn!(path = %path, "收到 401, 尝试 refresh token");
            self.try_refresh().await?;
            // 重试一次
            let req = self.build_request(&method, &url, body)?;
            let resp = req.send().await?;
            if resp.status().is_success() {
                self.mark_online();
                return Self::parse_json(resp).await;
            } else {
                self.mark_offline_or_err(resp.status());
                return Err(ApiError::from_response(resp).await);
            }
        }

        if resp.status().is_success() {
            self.mark_online();
            Self::parse_json(resp).await
        } else {
            self.mark_offline_or_err(resp.status());
            Err(ApiError::from_response(resp).await)
        }
    }

    fn build_request<Req: serde::Serialize>(
        &self,
        method: &Method,
        url: &str,
        body: Option<&Req>,
    ) -> Result<RequestBuilder, ApiError> {
        let mut req = self.state.http.request(method.clone(), url);
        if let Some(tok) = self.state.access_token() {
            req = req.bearer_auth(tok);
        }
        if let Some(b) = body {
            req = req.json(b);
        }
        Ok(req)
    }

    async fn parse_json<T: DeserializeOwned>(resp: reqwest::Response) -> Result<T, ApiError> {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        serde_json::from_str::<T>(&text).map_err(|e| {
            ApiError::Decode(format!("status={} body={} error={}", status, text, e))
        })
    }

    async fn handle<T: DeserializeOwned>(resp: reqwest::Response) -> Result<T, ApiError> {
        if resp.status().is_success() {
            Self::parse_json(resp).await
        } else {
            Err(ApiError::from_response(resp).await)
        }
    }

    /// 调 /v1/auth/refresh, 失败则清空 token
    async fn try_refresh(&self) -> Result<(), ApiError> {
        let refresh_token = {
            let guard = self.state.token.lock().expect("token mutex poisoned");
            guard.as_ref().map(|t| t.refresh_token.clone())
        };

        let Some(refresh_token) = refresh_token else {
            self.state.clear_token();
            return Err(ApiError::Unauthenticated);
        };

        #[derive(serde::Serialize)]
        struct RefreshReq<'a> {
            refresh_token: &'a str,
        }
        #[derive(serde::Deserialize)]
        struct RefreshResp {
            access_token: String,
            refresh_token: String,
            expires_in: i64,
        }

        let url = self.url("/v1/auth/refresh");
        let resp = self
            .state
            .http
            .post(&url)
            .json(&RefreshReq { refresh_token: &refresh_token })
            .send()
            .await?;

        if !resp.status().is_success() {
            // refresh 失败 → 视为登出
            self.state.clear_token();
            return Err(ApiError::from_response(resp).await);
        }

        let parsed: RefreshResp = Self::parse_json(resp).await?;
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(parsed.expires_in);
        self.state.set_token(TokenPair {
            access_token: parsed.access_token,
            refresh_token: parsed.refresh_token,
            expires_at,
        });
        Ok(())
    }

    fn mark_online(&self) {
        self.state.mark_online();
    }

    fn mark_offline_or_err(&self, status: reqwest::StatusCode) {
        // 5xx 或网络层错误 → 标记离线
        if status.is_server_error() {
            self.state.mark_offline();
        }
    }
}