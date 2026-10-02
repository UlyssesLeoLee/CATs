//! BFF → 下游 service 的透传 / 调用层
//!
//! 包含：
//! - [`UpstreamClient`] — 共享 reqwest client + 三个下游基址
//! - `forward_auth_login` / `forward_auth_refresh` — auth-service REST 透传
//! - `forward_project_list` — project-service REST 透传（带 X-Cats-* header）
//!
//! 翻译相关 gRPC 调用见 [`crate::grpc_clients`]。
//!
//! ---
//!
//! **未接入状态（不参与编译）**
//!
//! 本文件由 `feat/mvp-final-*` 抢救进仓（commit `68fe10b`），`lib.rs` 未声明
//! `mod` 故不参与编译。它与 `src/upstream/` 目录版是两条平行设计：
//!
//! - 本文件：`UpstreamClient` 共享 reqwest client + 裸 `serde_json::Value` 透传
//! - `src/upstream/`：`AuthClient` / `ProjectsClient` / `TasksClient` 三个强类型
//!   客户端，带 RBAC 校验，是 `main.rs` 实际注册路由所用的实现
//!
//! 直接接入不可行——它依赖的 4 个 API 在当前 `Config` 上均不存在：
//! `Config::for_test`、`auth_service_base`、`project_service_base`、
//! `upstream_timeout_ms`（当前是 `auth_service_url` / `project_service_url` /
//! `upstream_timeout_secs`）。接入前需先做适配改造。
//!
//! 文件名带 `_passthrough` 后缀而非直接叫 `upstream.rs`：同名会与
//! `src/upstream/mod.rs` 争抢同一模块路径，触发 E0761
//! `file for module 'upstream' found at both ...`，并连带让 `handlers.rs` 的
//! `crate::upstream::auth` 解析失败，级联出 14 个 never-type-fallback 错误。

use std::time::Duration;

use crate::config::Config;
use crate::error::BffError;
use reqwest::{header, Client, StatusCode as HttpStatus};
use serde_json::Value;

/// 透传层共享状态
#[derive(Debug, Clone)]
pub struct UpstreamClient {
    /// reqwest client（复用连接池）
    http: Client,
    /// auth-service 基址
    pub auth_base: String,
    /// project-service 基址
    pub project_base: String,
}

impl UpstreamClient {
    /// 从 [`Config`] 构造客户端；超时由 config 控制
    pub fn new(cfg: &Config) -> Result<Self, BffError> {
        let http = Client::builder()
            .timeout(Duration::from_millis(cfg.upstream_timeout_ms))
            .connect_timeout(Duration::from_millis(cfg.upstream_timeout_ms.min(2000)))
            .pool_idle_timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| BffError::Upstream(format!("reqwest build: {e}")))?;
        Ok(Self {
            http,
            auth_base: cfg.auth_service_base.trim_end_matches('/').to_string(),
            project_base: cfg.project_service_base.trim_end_matches('/').to_string(),
        })
    }

    /// `POST /v1/auth/login` — 透传；body 直接 forward
    pub async fn forward_auth_login(&self, body: &Value) -> Result<(HttpStatus, Value), BffError> {
        let url = format!("{}/v1/auth/login", self.auth_base);
        let resp = self
            .http
            .post(&url)
            .header(header::CONTENT_TYPE, "application/json")
            .json(body)
            .send()
            .await?;
        let status = resp.status();
        let json: Value = resp.json().await.map_err(|e| {
            BffError::Upstream(format!("auth login response decode: {e}"))
        })?;
        Ok((status, json))
    }

    /// `POST /v1/auth/refresh` — 透传
    pub async fn forward_auth_refresh(&self, body: &Value) -> Result<(HttpStatus, Value), BffError> {
        let url = format!("{}/v1/auth/refresh", self.auth_base);
        let resp = self
            .http
            .post(&url)
            .header(header::CONTENT_TYPE, "application/json")
            .json(body)
            .send()
            .await?;
        let status = resp.status();
        let json: Value = resp.json().await.map_err(|e| {
            BffError::Upstream(format!("auth refresh response decode: {e}"))
        })?;
        Ok((status, json))
    }

    /// `GET /v1/projects` — 透传；从 JWT claims 注入 `X-Cats-User-Id` / `X-Cats-Org-Id`
    ///
    /// 真实环境下这两个 header 由 Envoy Gateway 从 JWT claims 解析后注入；
    /// BFF 阶段保持透传（不在 BFF 内做验签），由调用方（worker#5 cats-edge 或客户端）传入。
    pub async fn forward_project_list(
        &self,
        user_id: Option<&str>,
        org_id: Option<&str>,
        auth_header: Option<&str>,
    ) -> Result<(HttpStatus, Value), BffError> {
        let url = format!("{}/v1/projects", self.project_base);
        let mut req = self.http.get(&url);
        if let Some(uid) = user_id {
            req = req.header("X-Cats-User-Id", uid);
        }
        if let Some(oid) = org_id {
            req = req.header("X-Cats-Org-Id", oid);
        }
        if let Some(auth) = auth_header {
            req = req.header(header::AUTHORIZATION, auth);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let json: Value = resp.json().await.map_err(|e| {
            BffError::Upstream(format!("project list response decode: {e}"))
        })?;
        Ok((status, json))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upstream_client_strips_trailing_slash() {
        let cfg = Config::for_test(
            "http://127.0.0.1:9001/".into(),
            "http://127.0.0.1:9003//".into(),
            "http://127.0.0.1:9090".into(),
        );
        let c = UpstreamClient::new(&cfg).unwrap();
        assert_eq!(c.auth_base, "http://127.0.0.1:9001");
        assert_eq!(c.project_base, "http://127.0.0.1:9003");
    }
}