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
    // issue_jwt 自 ULYS-149 起第 4 个参数是 roles（per BFF principal.roles 硬需求）
    let (token, expiry) = issue_jwt(user_id, username, "access", vec![]).expect("issue ok");
    assert!(!token.is_empty());
    assert_eq!(expiry, 3600);
}

#[test]
fn verify_jwt_roundtrips_claims() {
    setup_jwt_secret();
    let user_id = Uuid::new_v4();
    let username = "bob";
    let roles = vec!["tenant_admin".to_string(), "user".to_string()];
    let (token, _) = issue_jwt(user_id, username, "access", roles.clone()).expect("issue ok");
    let claims = verify_jwt(&token).expect("verify ok");
    assert_eq!(claims.sub, user_id.to_string());
    assert_eq!(claims.username, username);
    assert_eq!(claims.token_type, "access");
    // roles 必须穿过 JWT 往返（BFF 鉴权依赖该字段，ULYS-149）
    assert_eq!(claims.roles, roles);
}

/// 无 roles 时应为空 vec，而不是反序列化失败（Claims.roles 带 serde default）
#[test]
fn issue_jwt_without_roles_yields_empty_roles() {
    setup_jwt_secret();
    let (token, _) = issue_jwt(Uuid::new_v4(), "carol", "access", vec![]).expect("issue ok");
    let claims = verify_jwt(&token).expect("verify ok");
    assert!(claims.roles.is_empty());
}

#[test]
fn issue_jwt_refresh_has_longer_expiry() {
    setup_jwt_secret();
    let user_id = Uuid::new_v4();
    let (_access_token, access_exp) = issue_jwt(user_id, "u", "access", vec![]).expect("access ok");
    let (_refresh_token, refresh_exp) =
        issue_jwt(user_id, "u", "refresh", vec![]).expect("refresh ok");
    assert!(refresh_exp > access_exp);
    assert_eq!(access_exp, 3600);
    assert_eq!(refresh_exp, 86400);
}

#[test]
fn verify_jwt_rejects_tampered_token() {
    setup_jwt_secret();
    let (token, _) = issue_jwt(Uuid::new_v4(), "alice", "access", vec![]).expect("issue ok");
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
        roles: vec!["user".to_string()],
        roles_csv: "user".to_string(),
    };
    let json = serde_json::to_string(&claims).expect("serialize");
    let back: Claims = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.sub, claims.sub);
    assert_eq!(back.username, claims.username);
    assert_eq!(back.exp, claims.exp);
    assert_eq!(back.token_type, claims.token_type);
    assert_eq!(back.roles, claims.roles);
}

/// `roles` 数组与 `roles_csv` 逗号串必须同源派生。
///
/// 2026-10-07 新增。边缘（envoy `jwt_authn`）注入 `X-Cats-Roles` 用的是
/// `roles_csv` —— 因为 `claim_to_headers` 对 array 会 Base64 编码、对 string
/// 才原样拷贝。这条用例把"两份表示必须一致"钉住，防止将来只改其中一处。
#[test]
fn issued_token_carries_roles_and_matching_roles_csv() {
    setup_jwt_secret();
    let roles = vec!["User".to_string(), "QualityLead".to_string()];
    let (token, _) =
        issue_jwt(Uuid::new_v4(), "someone", "access", roles.clone()).expect("issue_jwt");

    let claims = verify_jwt(&token).expect("verify_jwt");
    assert_eq!(claims.roles, roles, "数组表示应原样保留");
    assert_eq!(
        claims.roles_csv, "User,QualityLead",
        "逗号串必须是同一份 roles 的 join(',')，顺序不得变"
    );

    // 反向：数组每一项都必须能在逗号串里按序找到（防止任何一边被悄悄裁剪）
    let parts: Vec<&str> = claims.roles_csv.split(',').collect();
    assert_eq!(
        parts,
        claims.roles.iter().map(|s| s.as_str()).collect::<Vec<_>>()
    );
}

/// 旧 token（JSON 里没有 roles 字段）必须仍能解析，靠 Claims.roles 的 serde(default)
#[test]
fn claims_deserialize_without_roles_field_defaults_to_empty() {
    let json = r#"{
        "sub": "00000000-0000-0000-0000-000000000001",
        "username": "legacy",
        "exp": 1234567890,
        "iat": 1234567800,
        "jti": "legacy-jti",
        "token_type": "access"
    }"#;
    let back: Claims = serde_json::from_str(json).expect("legacy token must still parse");
    assert_eq!(back.username, "legacy");
    assert!(back.roles.is_empty());
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
