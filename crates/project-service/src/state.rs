//! AppState — project-service 全局共享状态
//!
//! 引用: doc/03-模块设计/CATs_模块设计书_v2.0.md §4 (分层设计)
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §14 (RBAC 集成)

use cats_rbac::RbacChecker;
use sqlx::PgPool;
use std::sync::Arc;

/// 全局状态 — 在 main.rs 中用 `web::Data::new(...)` 注入
#[derive(Clone)]
pub struct AppState {
    /// PostgreSQL pool
    pub pool: PgPool,
    /// RBAC 权限矩阵 (per cats-rbac crate)
    pub checker: Arc<RbacChecker>,
}

impl AppState {
    /// 启动时构造
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            checker: Arc::new(RbacChecker::new()),
        }
    }
}