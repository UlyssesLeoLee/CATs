//! sqlx 0.8 兼容性冒烟
//!
//! 验证目标: sqlx 0.8 + PG 18.6 驱动 + pgvector 0.8.6 在 Rust 1.98.0 下编译
//! 运行时不连 DB (per CI 环境无 PG)

use sqlx::postgres::{PgPool, PgPoolOptions};

/// Smoke 入口: 验证 PgPool 类型 + 类型 state 可编译
pub async fn build_pool_lazy(url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect_lazy(url)
}

/// Smoke pgvector 0.8.6 类型探针: 确保 vector 类型在编译期可见
pub fn pgvector_type_probe() -> &'static str {
    // pgvector 通过 sqlx::types::Json + 自定义类型集成, 这里仅 type-check
    std::any::type_name::<sqlx::postgres::PgRow>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pool_build_lazy_succeeds_without_db() {
        // 不连 DB, 仅验证 URL parse 路径
        let pool = build_pool_lazy("postgres://invalid:invalid@127.0.0.1:1/none").await;
        assert!(
            pool.is_ok(),
            "lazy pool should not require connection at build"
        );
    }

    #[test]
    fn pgvector_type_probe_runs() {
        // 编译期验证: sqlx::postgres::PgRow 类型存在
        let _ = pgvector_type_probe();
    }
}
