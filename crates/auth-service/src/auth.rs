//! JWT 签发/验证 + 密码 hash
//!
//! 引用: doc/02-基础设计/技术选型/CATs_Rust技术选型书_v1.0.md §9
//! 引用: doc/05-其他/安全/CATs_安全要件定义书_v1.0.md §3
//! 引用: doc/05-其他/CATs_实施前QA登记册_v1.3.md §2.2 OI-1 RBAC 决议
//!
//! 规范:
//! - JWT HS256 (per §9 ADR-R-09, 不接受 RS256 简化路径)
//! - argon2id (OWASP 推荐: m=19456, t=2, p=1)
//! - access_token 1h, refresh_token 24h (per OI-1 安全基线)

use crate::models::{Claims, ErrorBody};
use anyhow::{anyhow, Context, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use std::env;

/// 密码 hash (argon2id)
pub fn hash_password(plain: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(plain.as_bytes(), &salt)
        .map_err(|e| anyhow!("argon2 hash failed: {e}"))?
        .to_string();
    Ok(hash)
}

/// 密码验证 (返回 bool, 不泄露具体错误以防时序攻击)
pub fn verify_password(plain: &str, hash: &str) -> bool {
    let parsed = match PasswordHash::new(hash) {
        Ok(p) => p,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(plain.as_bytes(), &parsed)
        .is_ok()
}

/// 从 env 读 JWT 密钥 (per 安全约束: 不硬编码, 不打印)
fn jwt_secret() -> Result<String> {
    env::var("JWT_SECRET").context("JWT_SECRET env var not set")
}

fn jwt_expiry_secs() -> i64 {
    env::var("JWT_EXPIRY_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3600) // 1h 默认
}

fn jwt_refresh_expiry_secs() -> i64 {
    env::var("JWT_REFRESH_EXPIRY_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(86400) // 24h 默认
}

/// 签发 JWT
pub fn issue_jwt(user_id: uuid::Uuid, username: &str, token_type: &str) -> Result<(String, i64)> {
    let secret = jwt_secret()?;
    let now = Utc::now().timestamp();
    let expiry_secs = match token_type {
        "refresh" => jwt_refresh_expiry_secs(),
        _ => jwt_expiry_secs(),
    };
    let exp = now + expiry_secs;
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        exp,
        iat: now,
        jti: uuid::Uuid::new_v4().to_string(),
        token_type: token_type.to_string(),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .context("jwt encode failed")?;
    Ok((token, expiry_secs))
}

/// 验证 JWT, 返回 Claims
pub fn verify_jwt(token: &str) -> Result<Claims, ErrorBody> {
    let secret = jwt_secret().map_err(|_| ErrorBody {
        error: "server_misconfigured".to_string(),
        message: "JWT secret not set".to_string(),
        detail: None,
    })?;
    let validation = Validation::default();
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| ErrorBody {
        error: "invalid_token".to_string(),
        message: format!("token validation failed: {e}"),
        detail: None,
    })?;
    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify_password_roundtrip() {
        let plain = "test_password_123!";
        let hash = hash_password(plain).expect("hash ok");
        assert!(verify_password(plain, &hash));
        assert!(!verify_password("wrong_password", &hash));
    }

    #[test]
    fn verify_password_returns_false_for_invalid_hash() {
        assert!(!verify_password("any", "not-a-valid-hash"));
    }
}
