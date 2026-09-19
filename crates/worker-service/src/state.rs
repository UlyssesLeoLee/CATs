//! worker-service 共享状态

use cats_rbac::RbacChecker;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub checker: Arc<RbacChecker>,
    /// translation-core gRPC endpoint (默认 `http://translation-core.cats-core.svc.cluster.local:50051`)
    pub translation_core_url: String,
}

impl AppState {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            checker: Arc::new(RbacChecker::new()),
            translation_core_url: std::env::var("TRANSLATION_CORE_URL")
                .unwrap_or_else(|_| "http://translation-core.cats-core.svc.cluster.local:50051".to_string()),
        }
    }
}