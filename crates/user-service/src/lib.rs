//! `user-service` — 用户组织服务
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1
//! 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.1 (user_db 接口契约 v1.0.0)
//! 引用: doc/05-其他/管理/CATs_错误码表_v1.0.md §3-§4 (error enum 复用)
//! 引用: doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0+1.md §2 T-02
//!
//! M1 业务实现 (per Sprint 1 T-02 完成判据 ①②③):
//! - POST /v1/users (创建 UserProfile, CRUD stub)
//! - GET  /v1/users/{id} (查询)
//! - PUT  /v1/users/{id} (更新)
//! - GET  /healthz
//! - 7 逻辑库 user_db (per Baseline §5.1) + pgcrypto extension

pub mod db;
pub mod handlers;
pub mod models;

pub use models::{CreateUserRequest, ErrorBody, GetUserResponse, UpdateUserRequest, UserProfile};

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
    fn name_is_user_service() {
        assert_eq!(name(), "user-service");
    }
}
