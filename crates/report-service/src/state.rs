//! AppState (report-service)

use crate::db::Pools;
use cats_rbac::RbacChecker;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pools: Pools,
    pub checker: Arc<RbacChecker>,
}

impl AppState {
    pub fn new(pools: Pools) -> Self {
        Self {
            pools,
            checker: Arc::new(RbacChecker::new()),
        }
    }
}