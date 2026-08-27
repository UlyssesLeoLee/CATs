//! `auth-service` - 认证授权服务
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1
//! 引用: doc/02-基础设计/技术选型/CATs_Rust技术选型书_v1.0.md §9 (JWT + argon2)
//!
//! M1 业务实现:
//! - POST /v1/auth/login (per OpenAPI v1)
//! - POST /v1/auth/refresh (per OpenAPI v1)
//! - 8 逻辑库 auth_db (per Baseline §5.1)
//! - JWT HS256 + argon2id 密码 hash (per 安全要件 §3)

pub mod auth;
pub mod db;
pub mod handlers;
pub mod models;

pub use models::{ErrorBody, LoginRequest, LoginResponse, RefreshRequest, RefreshResponse};

/// 当前 crate 语义版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 当前 crate 名称
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// 返回 crate 版本字符串
pub fn version() -> &'static str {
    VERSION
}

/// 返回 crate 名称
pub fn name() -> &'static str {
    NAME
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_semver_like() {
        let v = version();
        assert!(
            v.starts_with("0.1."),
            "version should start with '0.1.', got {v}"
        );
    }

    #[test]
    fn name_is_crate_name() {
        assert_eq!(name(), "auth-service");
    }
}
