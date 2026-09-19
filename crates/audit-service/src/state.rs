//! audit-service 共享状态

use cats_rbac::RbacChecker;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub checker: std::sync::Arc<RbacChecker>,
}

impl AppState {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            checker: std::sync::Arc::new(RbacChecker::new()),
        }
    }
}