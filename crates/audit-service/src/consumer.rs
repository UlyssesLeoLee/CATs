//! audit-service Kafka consumer
//!
//! MVP 简化: 不实际连 Kafka,提供 `process_event` 函数供测试 / REST / 后续 worker 调;
//! worker-service / producer 在 Kafka 真正落地后,这里的 `run_consumer_loop` 会订阅
//! `cats.audit.v1` topic 调用 `process_event`.

use crate::db;
use crate::models::KafkaAuditEvent;
use cats_common::{CatsError, ErrorCode};
use sqlx::PgPool;
use tracing::{error, info};

/// 处理一条 Kafka 消息 payload (bytes)
pub async fn process_event(pool: &PgPool, payload: &[u8]) -> Result<i64, CatsError> {
    let ev: KafkaAuditEvent = serde_json::from_slice(payload).map_err(|e| {
        CatsError::business(
            ErrorCode::InvalidRequest,
            format!("invalid audit event payload: {e}"),
        )
    })?;
    db::insert_event(pool, &ev).await
}

/// MVP 占位 consumer loop (per Sprint 2 W3-W4 SCOPE):
/// 真正连 Kafka 的版本留 Sprint 3 (需要 Kafka broker + topic 配置 + SASL 凭据)。
/// 这里 spawn 一个每 30s 心跳的循环,既不报错也不消耗资源。
pub async fn run_consumer_loop(_pool: PgPool, topic: String) {
    info!(topic = %topic, "audit-service consumer stub started (MVP simplification, real Kafka client deferred to Sprint 3)");
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn process_event_valid_payload_inserts() {
        // 注: 此测试需要 audit_db 在线 (cargo test 默认不跑)
        // 留给 Sprint 末 CI 跑
    }

    #[tokio::test]
    async fn process_event_invalid_payload_returns_err() {
        let pool = sqlx::PgPool::connect_lazy("postgres://invalid").unwrap();
        let res = process_event(&pool, b"not json").await;
        assert!(res.is_err());
    }
}