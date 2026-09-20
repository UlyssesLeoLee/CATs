//! Principal 提取器 + RBAC 检查 helper
//!
//! 设计要点:
//! - BFF 不验签 JWT (只做路由), 验签由 auth-service 完成 (per 微服务架构 §4.1)
//! - BFF 在 handler 入口用 `RbacChecker::check` 校验 (Role, Resource, Action)
//! - 当前实现: 从 Authorization 头拿 token, 解析 JWT payload (无验签) 拿到 user_id + roles
//!   这是 M1 阶段简化版; M2 阶段接入 JWKS / auth-service introspect endpoint
//!
//! 引用:
//! - 接口设计书 v2.0 §1.3 错误信封 + §3 JWT 转发
//! - 权限矩阵 v1.0 §3 角色 × 资源 × 操作

use crate::error::{BffError, BffResult};
use cats_rbac::{Action, Resource, RbacChecker, Role};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::future::{ready, Ready};
use actix_web::{FromRequest, HttpRequest};

/// JWT payload (不验签版, 简化处理)
///
/// 真实生产应该走 JWKS 或 auth-service introspect endpoint (per 接口设计书 v2.0 §3.4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub username: String,
    pub exp: i64,
    #[serde(default)]
    pub roles: Vec<String>,
}

impl Claims {
    pub fn roles(&self) -> Vec<Role> {
        self.roles
            .iter()
            .filter_map(|r| match r.as_str() {
                "Sponsor" => Some(Role::Sponsor),
                "ArchitectLead" => Some(Role::ArchitectLead),
                "RustLead" => Some(Role::RustLead),
                "DatabaseLead" => Some(Role::DatabaseLead),
                "QualityLead" => Some(Role::QualityLead),
                "ProjectLead" => Some(Role::ProjectLead),
                "SRELead" => Some(Role::SRELead),
                "User" => Some(Role::User),
                "Guest" => Some(Role::Guest),
                _ => None,
            })
            .collect()
    }
}

/// BFF 请求上下文 (从 Authorization 头抽取)
#[derive(Debug, Clone)]
pub struct Principal {
    pub user_id: String,
    pub username: String,
    pub access_token: String,
    pub roles: Vec<Role>,
}

impl Principal {
    pub fn require_auth(req: &HttpRequest) -> BffResult<Self> {
        let auth_header = req
            .headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or(BffError::Unauthorized("missing_authorization"))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(BffError::Unauthorized("invalid_authorization_scheme"))?;

        let claims = decode_jwt_payload(token).ok_or(BffError::Unauthorized("invalid_token"))?;

        let roles = claims.roles();
        Ok(Self {
            user_id: claims.sub,
            username: claims.username,
            access_token: token.to_string(),
            roles,
        })
    }

    /// RBAC 检查: (resource, action)
    pub async fn check(
        &self,
        checker: &RbacChecker,
        resource: Resource,
        action: Action,
    ) -> BffResult<()> {
        checker
            .check_roles(&self.roles, resource, action)
            .await
            .map_err(|err| BffError::Forbidden(err.to_string()))
    }
}

/// 简化 JWT 解析 — 只解 payload, 不验签
/// (per BACKEND_STATUS_v0.1 §3 M0 简化版, M2 接 JWKS)
fn decode_jwt_payload(token: &str) -> Option<Claims> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload_b64 = parts[1];
    use base64::Engine;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// 共享 RBAC checker (per main.rs 注入到 AppState)
pub fn shared_checker() -> Arc<RbacChecker> {
    Arc::new(RbacChecker::new())
}

// ---- FromRequest impl (供 handler 用 `p: Principal` 参数) ----

impl FromRequest for Principal {
    type Error = BffError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        ready(Self::require_auth(req))
    }
}
