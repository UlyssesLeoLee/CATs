//! auth-service 集成测试 (per 微服务架构书 §4.1)
//!
//! 引用: doc/05-其他/CATs_实施前QA登记册_v1.3.md §2.3 OI-1
//! 引用: api/openapi/cats-openapi-v1.yaml
//!
//! 测试范围 (per 任务清单):
//! - (a) login 成功返回 200 + JWT
//! - (b) login 错误密码返回 401
//! - (c) refresh 成功返回新 access_token
//! - (d) refresh 错 token 返回 401
//! - (e) auth (JWT 签发/验证/argon2 哈希) 模块单元测试
//!
//! 集成测试不连真实 PG: 单元覆盖 auth + models 模块的纯函数路径。
//! 端到端测试 (含 actix HTTP) 留 M1 实战 (per OI-3 兼容性验证后)。

use auth_service::auth::{hash_password, issue_jwt, verify_jwt, verify_password};
use auth_service::models::Claims;
use std::env;
use uuid::Uuid;

// === 单元测试: argon2 密码 hash ===

#[test]
fn hash_password_then_verify_password_succeeds() {
    let plain = "test_password_123!@#";
    let hash = hash_password(plain).expect("hash should succeed");
    assert!(verify_password(plain, &hash));
    assert!(!verify_password("wrong_password", &hash));
}

#[test]
fn verify_password_rejects_invalid_hash() {
    assert!(!verify_password("any", "not-a-valid-hash"));
}

#[test]
fn hash_password_produces_different_hashes_for_same_input() {
    // argon2 加盐, 同样明文产生不同 hash
    let plain = "same_password";
    let h1 = hash_password(plain).unwrap();
    let h2 = hash_password(plain).unwrap();
    assert_ne!(h1, h2);
    // 但都应验证通过
    assert!(verify_password(plain, &h1));
    assert!(verify_password(plain, &h2));
}

// === 单元测试: JWT 签发/验证 ===

fn setup_jwt_secret() {
    // 测试用稳定 secret
    env::set_var(
        "JWT_SECRET",
        "test_secret_at_least_32_bytes_long_for_hs256_xx",
    );
    env::set_var("JWT_EXPIRY_SECS", "3600");
    env::set_var("JWT_REFRESH_EXPIRY_SECS", "86400");
}

#[test]
fn issue_jwt_produces_valid_token() {
    setup_jwt_secret();
    let user_id = Uuid::new_v4();
    let username = "alice";
    let (token, expiry) = issue_jwt(user_id, username, "access").expect("issue ok");
    assert!(!token.is_empty());
    assert_eq!(expiry, 3600);
}

#[test]
fn verify_jwt_roundtrips_claims() {
    setup_jwt_secret();
    let user_id = Uuid::new_v4();
    let username = "bob";
    let (token, _) = issue_jwt(user_id, username, "access").expect("issue ok");
    let claims = verify_jwt(&token).expect("verify ok");
    assert_eq!(claims.sub, user_id.to_string());
    assert_eq!(claims.username, username);
    assert_eq!(claims.token_type, "access");
}

#[test]
fn issue_jwt_refresh_has_longer_expiry() {
    setup_jwt_secret();
    let user_id = Uuid::new_v4();
    let (_access_token, access_exp) = issue_jwt(user_id, "u", "access").expect("access ok");
    let (_refresh_token, refresh_exp) = issue_jwt(user_id, "u", "refresh").expect("refresh ok");
    assert!(refresh_exp > access_exp);
    assert_eq!(access_exp, 3600);
    assert_eq!(refresh_exp, 86400);
}

#[test]
fn verify_jwt_rejects_tampered_token() {
    setup_jwt_secret();
    let (token, _) = issue_jwt(Uuid::new_v4(), "alice", "access").expect("issue ok");
    // 篡改 token 末位
    let mut tampered = token;
    let last = tampered.pop().unwrap();
    tampered.push(if last == 'A' { 'B' } else { 'A' });
    assert!(verify_jwt(&tampered).is_err());
}

#[test]
fn verify_jwt_rejects_empty_token() {
    setup_jwt_secret();
    let result = verify_jwt("");
    assert!(result.is_err());
}

#[test]
fn claims_serialize_deserialize_roundtrip() {
    let claims = Claims {
        sub: Uuid::new_v4().to_string(),
        username: "test".to_string(),
        exp: 1234567890,
        iat: 1234567800,
        jti: Uuid::new_v4().to_string(),
        token_type: "access".to_string(),
    };
    let json = serde_json::to_string(&claims).expect("serialize");
    let back: Claims = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.sub, claims.sub);
    assert_eq!(back.username, claims.username);
    assert_eq!(back.exp, claims.exp);
    assert_eq!(back.token_type, claims.token_type);
}

// === 端到端测试占位 (M1 实战时填实) ===
//
// (a) POST /v1/auth/login 成功 → 200 + JWT: 需要 actix TestRequest + 真实 PG 或 sqlite 替身
// (b) POST /v1/auth/login 错误密码 → 401: 同上
// (c) POST /v1/auth/refresh 成功 → 200: 同上
// (d) POST /v1/auth/refresh 错 token → 401: 同上
//
// 当前未实施原因: CI 无 PG, sqlx::query_as! 编译期需 DATABASE_URL (per 微服务架构书 §5.2)
// 留 M1-Sprint 0 末 (QA-041 benchmark 跑完后) 用 [sqlx::test] attribute 实装

#[test]
fn end_to_end_login_placeholder() {
    // 占位: M1 实战后用 actix_test::init_service + sqlx::test 实装
    // 本测试仅作为占位, 防止 test runner 报 "no tests"
    let _ = std::marker::PhantomData::<()>;
}
