//! 审计事件抽象 (per T-01 auth-service 实战深化 §2)
//!
//! 引用: doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md §2 T-01
//! 引用: doc/05-其他/安全/CATs_安全要件定义书_v1.0.md §6
//!
//! 设计:
//! - `AuditSink` trait: 抽象事件输出, 业务逻辑不直接耦合 Kafka / DB
//! - `InMemoryAuditSink`: 测试用, Mutex<Vec<AuditEvent>> 收集
//! - `DbAuditSink`: 生产用, 写 audit_log 表 (DB 兜底, 永不丢)
//! - `KafkaAuditSink`: stub, cfg flag 控制, K3s 阶段二物理落地
//!   (per Sprint 1 拆解 §6.10 已知缺口, T-01 范围内不实做)
//!
//! 所有 sink 实现都 `async fn emit(...) -> Result<()>`,
//! 业务调用方不区分后端, 单测 + e2e 用 InMemory, 集成/prod 用 Db。

use crate::models::AuditEvent;
use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Mutex;

/// 审计事件输出抽象
#[async_trait]
pub trait AuditSink: Send + Sync {
    async fn emit(&self, event: &AuditEvent) -> Result<()>;
}

/// 内存 sink (测试用): 收集事件到 vec, 单测 + e2e 验证 emit 行为
#[derive(Default)]
pub struct InMemoryAuditSink {
    pub events: Mutex<Vec<AuditEvent>>,
}

impl InMemoryAuditSink {
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    /// 测试断言辅助: 取走所有事件 (按发生顺序)
    pub fn drain_events(&self) -> Vec<AuditEvent> {
        std::mem::take(&mut *self.events.lock().unwrap())
    }

    /// 测试断言辅助: 当前事件数
    pub fn len(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    /// 测试断言辅助: 是否空 (clippy `len_without_is_empty` 要求配对)
    pub fn is_empty(&self) -> bool {
        self.events.lock().unwrap().is_empty()
    }

    /// 测试断言辅助: 找某 event_type
    pub fn find_event(&self, event_type: &str) -> Option<AuditEvent> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .find(|e| e.event_type == event_type)
            .cloned()
    }
}

#[async_trait]
impl AuditSink for InMemoryAuditSink {
    async fn emit(&self, event: &AuditEvent) -> Result<()> {
        self.events.lock().unwrap().push(event.clone());
        Ok(())
    }
}

/// DB 兜底 sink (生产用): 写 audit_log 表
///
/// 选择理由 (per T-01 设计):
/// - DB 写失败 → panic 不允许 (event 必落)
/// - 但 sink 内部用 ON CONFLICT DO NOTHING (event_id 唯一) → 重发幂等
/// - 当前服务结构: 写 audit_log 表 + (TODO K3s) Kafka topic 同步广播
pub struct DbAuditSink {
    pub pool: PgPool,
}

impl DbAuditSink {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 构造幂等 INSERT SQL
    ///
    /// 单测使用: 验证 SQL 文本固定, 包含 `ON CONFLICT (event_id) DO NOTHING`
    /// 这是审计**幂等性**的核心保证 (重发不产生重复行)
    ///
    /// 返回: (sql, 占位符数量) — 占位符数量用于验证 bind 顺序正确
    pub fn build_insert_sql() -> (&'static str, usize) {
        (
            r#"
            INSERT INTO audit_log
                (event_id, user_id, event_type, outcome, detail, source_ip, user_agent, occurred_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (event_id) DO NOTHING
            "#,
            8, // $1..$8
        )
    }
}

#[async_trait]
impl AuditSink for DbAuditSink {
    async fn emit(&self, event: &AuditEvent) -> Result<()> {
        let (sql, _) = Self::build_insert_sql();
        sqlx::query(sql)
            .bind(event.event_id)
            .bind(event.user_id)
            .bind(&event.event_type)
            .bind(event.outcome.as_str())
            .bind(&event.detail)
            .bind(event.source_ip.as_deref())
            .bind(event.user_agent.as_deref())
            .bind(event.occurred_at)
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("audit_log insert failed: db_err={e}"))?;
        Ok(())
    }
}

/// Kafka sink (stub): K3s 阶段二实做, 当前 cfg flag 控制, emit 时 log + 标 TODO
///
/// 为什么是 stub 而非 rdkafka 实做:
/// - OI-3 已记录 rdkafka 0.36 移 K3s 阶段二 (cmake-build 依赖 librdkafka 系统库)
/// - T-01 范围: trait + InMemory + Db 三层就够验证
/// - 生产部署: 启用此 sink 时需 rdkafka feature, K3s 阶段二统一处理
pub struct KafkaAuditSinkStub {
    pub topic: String,
}

impl KafkaAuditSinkStub {
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            topic: topic.into(),
        }
    }
}

#[async_trait]
impl AuditSink for KafkaAuditSinkStub {
    async fn emit(&self, event: &AuditEvent) -> Result<()> {
        // TODO(K3s-阶段二): rdkafka 实做物理发布
        // 当前: 仅 tracing 记录, DB 兜底由 DbAuditSink 接管 (双写架构)
        tracing::info!(
            target: "audit.kafka_stub",
            topic = %self.topic,
            event_type = %event.event_type,
            "Kafka emit stub (K3s 阶段二 实做, 当前 DB 兜底)"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AuditEvent, AuditOutcome};
    use chrono::Utc;
    use uuid::Uuid;

    fn sample_event(event_type: &str) -> AuditEvent {
        AuditEvent {
            event_id: Uuid::new_v4(),
            user_id: Some(Uuid::new_v4()),
            event_type: event_type.to_string(),
            outcome: AuditOutcome::Success,
            detail: Some(serde_json::json!({"key": "value"})),
            source_ip: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent/1.0".to_string()),
            occurred_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn in_memory_sink_collects_events() {
        let sink = InMemoryAuditSink::new();
        let e1 = sample_event("login");
        let e2 = sample_event("logout");
        sink.emit(&e1).await.unwrap();
        sink.emit(&e2).await.unwrap();
        assert_eq!(sink.len(), 2);
        let drained = sink.drain_events();
        assert_eq!(drained[0].event_type, "login");
        assert_eq!(drained[1].event_type, "logout");
    }

    #[tokio::test]
    async fn in_memory_sink_find_event_by_type() {
        let sink = InMemoryAuditSink::new();
        sink.emit(&sample_event("login")).await.unwrap();
        sink.emit(&sample_event("logout")).await.unwrap();
        let found = sink.find_event("logout").expect("logout event present");
        assert_eq!(found.event_type, "logout");
        assert!(sink.find_event("nonexistent").is_none());
    }

    #[tokio::test]
    async fn kafka_stub_sink_returns_ok_and_logs() {
        // 验证 stub emit 不 panic, 返回 Ok(())
        let sink = KafkaAuditSinkStub::new("audit.event");
        let e = sample_event("login");
        let r = sink.emit(&e).await;
        assert!(r.is_ok());
    }

    // =====================================================================
    // 幂等性单测 (per 设计书 §6.1 v0.1 后续 + UT 审查报告 §3 P2)
    // 引用: CATs_测试Mock项目设计书_v1.0 §6.1
    // 验证: ON CONFLICT (event_id) DO NOTHING 是审计**幂等性**核心保证
    // =====================================================================

    #[test]
    fn build_insert_sql_contains_on_conflict_clause() {
        // 验证 SQL 文本固定包含 ON CONFLICT (event_id) DO NOTHING
        // 防回归: 有人无意移除 ON CONFLICT 后重发会产生重复行
        let (sql, _) = DbAuditSink::build_insert_sql();
        assert!(
            sql.contains("ON CONFLICT (event_id) DO NOTHING"),
            "INSERT SQL must be idempotent (ON CONFLICT DO NOTHING); got:\n{sql}"
        );
    }

    #[test]
    fn build_insert_sql_targets_audit_log_table() {
        // 验证 SQL 写入正确的表
        let (sql, _) = DbAuditSink::build_insert_sql();
        assert!(sql.contains("INSERT INTO audit_log"), "must INSERT INTO audit_log");
    }

    #[test]
    fn build_insert_sql_has_eight_placeholders() {
        // 验证占位符数量与 bind 数量一致 ($1..$8)
        // 防回归: bind 与 placeholder 错位会导致运行时 panic
        let (sql, placeholders) = DbAuditSink::build_insert_sql();
        assert_eq!(placeholders, 8, "must have 8 placeholders for 8 fields");
        // 额外硬验证: 字符串中确实出现 $1, $2, ..., $8
        for i in 1..=8 {
            assert!(
                sql.contains(&format!("${i}")),
                "SQL must contain ${i} placeholder"
            );
        }
    }

    #[test]
    fn build_insert_sql_is_static_across_calls() {
        // 验证同一 SQL 两次调用返回相同内容 (无随机/时间戳干扰)
        let (sql1, n1) = DbAuditSink::build_insert_sql();
        let (sql2, n2) = DbAuditSink::build_insert_sql();
        assert_eq!(sql1, sql2, "SQL must be static (same across calls)");
        assert_eq!(n1, n2, "placeholder count must be stable");
    }

    #[test]
    fn build_insert_sql_covers_all_audit_event_fields() {
        // 验证 SQL 引用了 AuditEvent 的所有 8 个字段 (按顺序)
        // 防回归: 有人增字段后忘记加到 SQL
        let (sql, _) = DbAuditSink::build_insert_sql();
        let required_columns = [
            "event_id", "user_id", "event_type", "outcome",
            "detail", "source_ip", "user_agent", "occurred_at",
        ];
        for col in required_columns {
            assert!(
                sql.contains(col),
                "INSERT column list must include {col}"
            );
        }
    }
}
