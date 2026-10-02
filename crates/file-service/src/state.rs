//! AppState (file-service)

use cats_rbac::RbacChecker;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub checker: Arc<RbacChecker>,
}

impl AppState {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            checker: Arc::new(RbacChecker::new()),
        }
    }
}