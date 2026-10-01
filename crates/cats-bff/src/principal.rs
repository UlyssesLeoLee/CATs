//! Principal 提取器 + RBAC 检查 helper
//!
//! 设计要点 (per ULYS-149 审核修复, 2026-09-23):
//! - BFF **验签** JWT (HS256, 与 auth-service 共享同一个 `JWT_SECRET`, per 技术选型书 §9 ADR-R-09)
//!   —— 此前版本只解析 payload、不校验签名，任何调用方可自行伪造 `sub`/`roles`，
//!   绕过认证与 RBAC (per ULYS-149 审核结论)。现改为 `jsonwebtoken::decode` 全量验证
//!   签名 + `exp` 过期时间，签名或时间任一不通过都会被拒绝，不再信任客户端可控输入。
//! - BFF 在 handler 入口用 `RbacChecker::check` 校验 (Role, Resource, Action)
//!
//! **补充修复 (per PR #12 review, 2026-09-23)**：初版只验签+校验 `exp`，未区分
//! `token_type`。`auth-service` 同时签发 `access`(1h) 和 `refresh`(24h) 两种 token，
//! 且自己的 `/v1/auth/refresh` handler 明确拒绝把 access token 当 refresh token 用
//! (`crates/auth-service/src/handlers.rs` 的 `token_type != "refresh"` 检查)——
//! 但反过来 BFF 这边此前没有对称检查，refresh token 泄漏后能直接当 access token
//! 打到所有受保护端点，可用窗口从 1h 变相延长到 24h。现在 `verify_jwt` 额外校验
//! `claims.token_type == "access"`，其它值一律按 `invalid_token_type` 拒绝。
//!
//! **已知留尾（本次修复范围之外，需要单独跟进）**：`auth-service` 签发的真实 JWT
//! (`crates/auth-service/src/models.rs::Claims`) 目前根本不包含 `roles` 声明——即便
//! 签名验证通过，`Principal.roles` 在生产环境下也永远是空的，所有走 `RbacChecker`
//! 的鉴权检查都会因为角色集为空而被拒绝。这不是本次修复引入的新问题，而是修复
//! "签名不校验"之后暴露出的下一层缺口：角色本身目前没有任何合法来源（无论是否验签）。
//! 落地角色来源（例如按 `sub` 查 user-service，或由 auth-service 在签发时把角色写入
//! JWT）需要另开工单跟进，本次不在 ULYS-149 的"JWT 验签"范围内顺手处理。
//!
//! 引用:
//! - 接口设计书 v2.0 §1.3 错误信封 + §3 JWT 转发
//! - 权限矩阵 v1.0 §3 角色 × 资源 × 操作
//! - `crates/auth-service/src/auth.rs::verify_jwt`（同一套验证逻辑的签发方实现，本文件与其对齐）

use crate::error::{BffError, BffResult};
use cats_rbac::{Action, Resource, RbacChecker, Role};
use jsonwebtoken::{decode, errors::ErrorKind, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
use std::future::{ready, Ready};
use actix_web::{FromRequest, HttpRequest};

/// JWT payload（已验签）
///
/// 字段对齐 `auth-service::models::Claims`（签发方）。注意真实 token **不含 `roles`**——
/// 这里保留 `roles` 字段只是为了不破坏 `Principal` 的既有接口形状，`#[serde(default)]`
/// 保证字段缺失时反序列化不报错，实际值目前恒为空数组（见上方模块文档"已知留尾"）。
///
/// `token_type` 与 `roles` 不同：真实 token 一定带这个字段（签发方 `issue_jwt` 强制写入，
/// 见 `auth-service::auth::issue_jwt`），所以这里**不加 `#[serde(default)]`**——缺失该字段
/// 的 token 应当直接反序列化失败、被当作 `invalid_token` 拒绝，而不是静默放行。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub username: String,
    pub exp: i64,
    pub token_type: String, // "access" | "refresh"; verify_jwt 只放行 "access"
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

        let claims = verify_jwt(token)?;

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

/// 验证 JWT 签名 + 过期时间 + token 类型，返回 Claims (per ULYS-149 审核修复 + PR #12 补充)
///
/// 与 `auth-service::auth::verify_jwt` 使用同一把共享密钥（`JWT_SECRET` 环境变量）、
/// 同一种算法（HS256，per 技术选型书 §9 ADR-R-09），确保 BFF 只信任 auth-service
/// 真正签发过的 token，拒绝任何签名不匹配或已过期的请求。签名+时间通过后，
/// 额外要求 `token_type == "access"`——refresh token 只应在 `/v1/auth/refresh`
/// 这一个 endpoint 里被使用，绝不应该被当作访问其它受保护端点的 Bearer token。
fn verify_jwt(token: &str) -> BffResult<Claims> {
    let secret = env::var("JWT_SECRET")
        .map_err(|_| BffError::ServerMisconfigured("JWT_SECRET env var not set".to_string()))?;

    let mut validation = Validation::new(Algorithm::HS256);
    // auth-service 签发时不设 `aud`/`iss`，这里保持默认 (仅强制校验 exp)，
    // 与 auth-service::auth::verify_jwt 的 `Validation::default()` 行为一致。
    validation.validate_exp = true;

    let claims = decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &validation)
        .map(|data| data.claims)
        .map_err(|err| match err.kind() {
            ErrorKind::ExpiredSignature => BffError::Unauthorized("token_expired"),
            _ => BffError::Unauthorized("invalid_token"),
        })?;

    if claims.token_type != "access" {
        return Err(BffError::Unauthorized("invalid_token_type"));
    }

    Ok(claims)
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
