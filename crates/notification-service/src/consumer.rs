//! Kafka 消费者 stub (notification-service)
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.2 (Kafka 事件)
//! 引用: doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md §2 (Kafka topic `cats.notifications.v1`)
//!
//! MVP 简化 (per brief §1 notification-service): 不实装 rdkafka 客户端, 提供一个
//! 周期 poll 接口, 接收 `KafkaEvent` JSON bytes, 调 `db::insert_from_event` + `tracing::info!` 日志。
//! 真实 Kafka 集成 (rdkafka / kafka crate) 留 Sprint 2 + K3s 阶段二。

use crate::db;
use crate::models::KafkaEvent;
use cats_common::CatsError;
use sqlx::PgPool;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

/// 周期 poll 模拟 Kafka consumer (MVP)
pub async fn run_consumer_loop(pool: PgPool, topic: String) {
    info!(topic = %topic, "starting Kafka consumer loop (MVP poll stub)");
    loop {
        match poll_once(&pool, &topic).await {
            Ok(0) => {
                // 没消息, sleep 5s
                sleep(Duration::from_secs(5)).await;
            }
            Ok(n) => {
                info!(topic = %topic, processed = n, "poll_once processed");
            }
            Err(e) => {
                warn!(error = %e, topic = %topic, "poll_once failed");
                sleep(Duration::from_secs(10)).await;
            }
        }
    }
}

/// 模拟 poll 一次: 真实环境用 `KafkaConsumer::recv()`, MVP 返回 0 (无消息)。
/// 注入测试用 `process_event`。
pub async fn poll_once(_pool: &PgPool, _topic: &str) -> Result<usize, CatsError> {
    Ok(0)
}

/// 处理单个 Kafka event (供未来 Kafka 客户端回调)
pub async fn process_event(pool: &PgPool, event_bytes: &[u8]) -> Result<uuid::Uuid, CatsError> {
    let ev: KafkaEvent = serde_json::from_slice(event_bytes)?;
    let row = db::insert_from_event(pool, &ev).await?;
    info!(
        event_id = %ev.event_id,
        notification_id = %row.id,
        event_type = %ev.event_type,
        user_id = %ev.user_id,
        "notification dispatched (log-only, no SMTP/WebSocket)"
    );
    Ok(row.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kafka_event_round_trip() {
        let ev = KafkaEvent {
            event_id: uuid::Uuid::new_v4(),
            org_id: uuid::Uuid::new_v4(),
            user_id: uuid::Uuid::new_v4(),
            event_type: "task.completed".to_string(),
            payload: serde_json::json!({"task_id": "abc"}),
        };
        let bytes = serde_json::to_vec(&ev).unwrap();
        let back: KafkaEvent = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back.event_type, "task.completed");
    }
}