//! 基础设施 mock: Kafka / Redis 内存替身
//!
//! 引用: 设计书 §4.3
//!
//! 提供:
//! - [`MockKafka`]: in-mem topic 队列 (生产/消费 offset 都记)
//! - [`MockRedis`]: in-mem KV + 简单 set/zset 操作
//!
//! 适用: worker-service / audit-service / notification-service 集成测试

pub mod kafka;
pub mod redis;

pub use kafka::*;
pub use redis::*;
