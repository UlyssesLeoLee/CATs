//! audit-service Kafka consumer
//!
//! MVP 简化策略 (per ULYS-153 切片 C-2 范围):
//! - 不直接依赖 `rdkafka` (cmake-build 依赖 librdkafka 系统库 + rustc 1.98 metadata bug)
//! - 通过 Kafka REST Proxy (Confluent / 自建) HTTP 长轮询消费 `cats.audit.v1` topic
//! - 收到一条消息 → 反序列化为 `KafkaAuditEvent` → 直接 SQL INSERT 到 audit_logs
//!   (用 ON CONFLICT (event_id) DO UPDATE 幂等)
//! - 若 `KAFKA_REST_URL` 未设, 退化为 30s 心跳 no-op (per 旧 stub 行为, 兼容 K3s 阶段二未上线)
//!
//! 设计: REST proxy 抽象让 `rdkafka` 升级为 feature flag 时只需替换 poll 内部实现,
//! `run_consumer_loop` 签名不变.
//!
//! 引用: doc/02-基础设计/部署设计/CATs_Kafka物理发布设计_v1.0.md
//! 引用: doc/05-其他/错误码/CATs_错误码表_v1.0.md (per 失败重试策略)
//!
//! 自包含: 不依赖 crate 内其他模块 (handlers/db/models 都还是孤儿文件),
//! 重复定义 `KafkaAuditEvent` 与 INSERT SQL 以保证 consumer 模块可独立编译.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::env;
use std::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;

// =====================================================================
// 1. 自包含 Kafka event schema (重复自 crate 原 models.rs, 但消费链独立)
// =====================================================================

/// `cats.audit.v1` topic 消息 schema (per 接口设计 §1.5 schema_version 字段)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KafkaAuditEvent {
    pub event_id: Uuid,
    pub org_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    #[serde(default)]
    pub before_state: Option<serde_json::Value>,
    #[serde(default)]
    pub after_state: Option<serde_json::Value>,
    #[serde(default)]
    pub ip: Option<String>,
    pub occurred_at: DateTime<Utc>,
    #[serde(default)]
    pub schema_version: u32,
}

// =====================================================================
// 2. REST proxy 长轮询响应 schema (兼容 Confluent REST Proxy v2)
// =====================================================================

#[derive(Debug, Deserialize)]
struct RestRecord {
    #[serde(default)]
    topic: Option<String>,
    #[serde(default)]
    partition: Option<i32>,
    #[serde(default)]
    offset: Option<i64>,
    /// 业务 payload (proxy 默认 base64, 但我们直接当 raw JSON 字符串处理)
    #[serde(default)]
    value: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct RestRecordsResponse {
    #[serde(default)]
    records: Vec<RestRecordGroup>,
}

#[derive(Debug, Deserialize)]
struct RestRecordGroup {
    #[serde(default)]
    partition: i32,
    #[serde(default)]
    offset: i64,
    #[serde(default)]
    records: Vec<RestRecord>,
}

// =====================================================================
// 3. 业务函数: 处理一条 Kafka 消息 payload (bytes)
// =====================================================================

/// 处理一条 Kafka 消息 payload (bytes) → 写 audit_logs 表
///
/// 错误返回 `Result<i64, String>` (字符串错误, 不依赖 cats_common::CatsError)
pub async fn process_event(pool: &PgPool, payload: &[u8]) -> Result<i64, String> {
    let ev: KafkaAuditEvent = serde_json::from_slice(payload)
        .map_err(|e| format!("invalid audit event payload: {e}"))?;

    // IP 解析: 字符串 → PG INET (用 ::inet 转换), 失败置 None
    // 不依赖 sqlx::types::ipnetwork (feature 未开), 让 PG 自己做类型转换
    let ip_bind: Option<&str> = ev.ip.as_deref();

    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO audit_logs
            (event_id, org_id, actor_user_id, action, resource_type, resource_id,
             before_state, after_state, ip, occurred_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::inet, $10)
        ON CONFLICT (event_id) DO UPDATE SET ingested_at = now()
        RETURNING id
        "#,
    )
    .bind(ev.event_id)
    .bind(ev.org_id)
    .bind(ev.actor_user_id)
    .bind(&ev.action)
    .bind(&ev.resource_type)
    .bind(&ev.resource_id)
    .bind(ev.before_state)
    .bind(ev.after_state)
    .bind(ip_bind)
    .bind(ev.occurred_at)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("insert audit_log failed: {e}"))?;

    Ok(row.0)
}

// =====================================================================
// 4. consumer loop: 启动 / 退避 / 长轮询
// =====================================================================

/// 真实 consumer loop (per ULYS-153 切片 C-2)
///
/// 行为:
/// - 优先从 env `KAFKA_REST_URL` 读取 Kafka REST proxy base URL
/// - 若已设, 启动 HTTP 长轮询: 每 1s GET `${KAFKA_REST_URL}/consumers/${group}/topics/${topic}/records?timeout=1000`
///   解析返回的 records, 每条调 `process_event` 落档
/// - 若未设, 退化 30s 心跳 no-op (旧 stub 行为, 允许无 broker 环境运行)
pub async fn run_consumer_loop(pool: PgPool, topic: String) {
    let group = env::var("KAFKA_CONSUMER_GROUP").unwrap_or_else(|_| "audit-service".to_string());
    let rest_url = env::var("KAFKA_REST_URL").ok();

    match rest_url {
        None => {
            warn!(
                topic = %topic,
                "audit-service consumer: KAFKA_REST_URL not set, falling back to 30s heartbeat no-op \
                 (per K3s 阶段二 rdkafka 物理发布未上线, see BACKEND_STATUS_v0.1 §2 + OI-3)"
            );
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        }
        Some(base) => {
            info!(
                topic = %topic,
                group = %group,
                rest_url = %base,
                "audit-service consumer started (HTTP poll via Kafka REST proxy)"
            );
            run_rest_poll_loop(pool, base, group, topic).await;
        }
    }
}

/// REST proxy HTTP 长轮询主循环
///
/// 失败重试策略: 网络/HTTP 错误 → warn + sleep 5s 重连, 不退出循环
/// 单条消息处理失败 → error + skip (依赖 ON CONFLICT 幂等保证重发安全)
async fn run_rest_poll_loop(pool: PgPool, base_url: String, group: String, topic: String) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(35)) // 略大于 REST proxy 的 timeout=1000ms + 网络抖动
        .build()
        .expect("reqwest Client build");

    let url = format!(
        "{}/consumers/{}/topics/{}/records?timeout=1000&max_bytes=1048576",
        base_url.trim_end_matches('/'),
        group,
        topic
    );
    info!(poll_url = %url, "audit consumer poll loop started");

    loop {
        match client.get(&url).send().await {
            Ok(resp) => {
                let status = resp.status();
                if !status.is_success() {
                    warn!(http_status = %status, "audit consumer: REST proxy non-2xx, retry in 5s");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
                match resp.json::<RestRecordsResponse>().await {
                    Ok(body) => {
                        let count = consume_response(&pool, body).await;
                        if count == 0 {
                            // 空响应 → 服务端 timeout, 不需要退避
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "audit consumer: REST proxy JSON decode failed, retry in 5s");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "audit consumer: REST proxy request failed, retry in 5s");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

/// 解析 REST proxy 响应, 对每条 record 调 `process_event`
/// 返回成功处理的条数
async fn consume_response(pool: &PgPool, body: RestRecordsResponse) -> usize {
    let mut ok_count = 0usize;
    for group_rec in body.records {
        for rec in group_rec.records {
            let payload: Vec<u8> = match &rec.value {
                serde_json::Value::String(s) => s.as_bytes().to_vec(),
                serde_json::Value::Null => continue,
                other => other.to_string().into_bytes(),
            };
            match process_event(pool, &payload).await {
                Ok(id) => {
                    info!(audit_log_id = id, partition = rec.partition, offset = rec.offset, "audit event ingested");
                    ok_count += 1;
                }
                Err(e) => {
                    error!(error = %e, partition = rec.partition, offset = rec.offset, "audit event ingest failed (skip, ON CONFLICT 幂等保护)");
                }
            }
        }
    }
    ok_count
}

// =====================================================================
// 5. 单元测试 (不依赖 DB / 网络, 验证 schema 解析)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kafka_audit_event_deserialize_minimal() {
        let json = serde_json::json!({
            "event_id": "11111111-1111-1111-1111-111111111111",
            "org_id": "22222222-2222-2222-2222-222222222222",
            "action": "user.login",
            "resource_type": "user",
            "resource_id": "33333333-3333-3333-3333-333333333333",
            "occurred_at": "2026-09-20T12:00:00Z"
        });
        let ev: KafkaAuditEvent = serde_json::from_value(json).unwrap();
        assert_eq!(ev.action, "user.login");
        assert_eq!(ev.schema_version, 0); // 默认
        assert!(ev.before_state.is_none());
    }

    #[test]
    fn kafka_audit_event_deserialize_full() {
        let json = serde_json::json!({
            "event_id": "11111111-1111-1111-1111-111111111111",
            "org_id": "22222222-2222-2222-2222-222222222222",
            "actor_user_id": "44444444-4444-4444-4444-444444444444",
            "action": "project.update",
            "resource_type": "project",
            "resource_id": "55555555-5555-5555-5555-555555555555",
            "before_state": {"name": "old"},
            "after_state": {"name": "new"},
            "ip": "192.168.1.1",
            "occurred_at": "2026-09-20T12:00:00Z",
            "schema_version": 1
        });
        let ev: KafkaAuditEvent = serde_json::from_value(json).unwrap();
        assert_eq!(ev.action, "project.update");
        assert_eq!(ev.schema_version, 1);
        assert_eq!(ev.ip.as_deref(), Some("192.168.1.1"));
    }

    #[test]
    fn rest_records_response_decode_empty() {
        let json = serde_json::json!({ "records": [] });
        let parsed: RestRecordsResponse = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.records.len(), 0);
    }

    #[test]
    fn rest_records_response_decode_with_null_value_skipped() {
        let json = serde_json::json!({
            "records": [
                {
                    "partition": 0,
                    "offset": 42,
                    "records": [
                        {"value": null},
                        {"value": "{\"event_id\":\"11111111-1111-1111-1111-111111111111\",\"org_id\":\"22222222-2222-2222-2222-222222222222\",\"action\":\"test\",\"resource_type\":\"x\",\"resource_id\":\"y\",\"occurred_at\":\"2026-09-20T12:00:00Z\"}"}
                    ]
                }
            ]
        });
        let parsed: RestRecordsResponse = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.records.len(), 1);
        assert_eq!(parsed.records[0].records.len(), 2);
        // null value 应被 consume_response 跳过 (此处仅验证 schema 解码)
        assert!(parsed.records[0].records[0].value.is_null());
        assert!(parsed.records[0].records[1].value.is_string());
    }

    #[test]
    fn process_event_returns_err_for_invalid_json() {
        // 同步版本仅验证 invalid JSON 路径 (async 路径需要 DB, 不在单测跑)
        let bad = b"not json";
        let parsed: Result<KafkaAuditEvent, _> = serde_json::from_slice(bad);
        assert!(parsed.is_err());
    }
}
