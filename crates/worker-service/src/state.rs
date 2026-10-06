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
            translation_core_url: std::env::var("TRANSLATION_CORE_URL").unwrap_or_else(|_| {
                "http://translation-core.cats-core.svc.cluster.local:50051".to_string()
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    /// lazy pool: 只解析 URL, 不实际建连 (per file-service / task-service 同模式)
    fn lazy_pool() -> PgPool {
        PgPoolOptions::new()
            .connect_lazy("postgres://u:p@127.0.0.1:1/none")
            .expect("connect_lazy should not dial")
    }

    /// 进程级 env 是全局的: 所有改动 TRANSLATION_CORE_URL 的用例共用这把锁
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[tokio::test]
    async fn new_builds_state_with_lazy_pool() {
        let state = AppState::new(lazy_pool());
        assert!(!state.pool.is_closed());
    }

    #[tokio::test]
    async fn new_defaults_translation_core_url_to_incluster_endpoint() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let prev = std::env::var("TRANSLATION_CORE_URL").ok();
        std::env::remove_var("TRANSLATION_CORE_URL");

        let state = AppState::new(lazy_pool());
        assert_eq!(
            state.translation_core_url,
            "http://translation-core.cats-core.svc.cluster.local:50051"
        );

        if let Some(v) = prev {
            std::env::set_var("TRANSLATION_CORE_URL", v);
        }
    }

    #[tokio::test]
    async fn new_honours_translation_core_url_override() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let prev = std::env::var("TRANSLATION_CORE_URL").ok();
        std::env::set_var("TRANSLATION_CORE_URL", "http://127.0.0.1:50099");
        let state = AppState::new(lazy_pool());
        assert_eq!(state.translation_core_url, "http://127.0.0.1:50099");
        match prev {
            Some(v) => std::env::set_var("TRANSLATION_CORE_URL", v),
            None => std::env::remove_var("TRANSLATION_CORE_URL"),
        }
    }
}
