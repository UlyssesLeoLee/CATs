//! Kafka 内存 mock
//!
//! 引用: 设计书 §4.3.1
//!
//! 接口对齐 `rdkafka` 的极简子集: `produce` / `consume` / `commit`
//! 真实 rdkafka 0.36 留给 K3s 阶段二 (per OI-3)

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Kafka 消息
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KafkaMessage {
    /// topic
    pub topic: String,
    /// partition (固定 0, 单分区简化)
    pub partition: i32,
    /// offset (per topic 递增)
    pub offset: i64,
    /// key (允许 None)
    pub key: Option<String>,
    /// value (原始 bytes)
    pub value: Vec<u8>,
    /// 时间戳 (ms since epoch)
    pub timestamp_ms: i64,
}

impl KafkaMessage {
    /// value 转 String (UTF-8 假设)
    pub fn value_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.value).ok()
    }
}

/// Kafka mock (in-mem, 单 broker 单 partition 简化)
#[derive(Default)]
pub struct MockKafka {
    /// topic → FIFO 队列
    topics: Mutex<HashMap<String, VecDeque<KafkaMessage>>>,
    /// 已 produce 的总条数
    produced_count: Mutex<u64>,
    /// 已 consume 的总条数
    consumed_count: Mutex<u64>,
}

impl MockKafka {
    /// 新建
    pub fn new() -> Self { Self::default() }

    /// produce 一条
    pub fn produce(&self, topic: impl Into<String>, key: Option<&str>, value: &[u8]) -> KafkaMessage {
        let topic = topic.into();
        let mut topics = self.topics.lock().unwrap();
        let queue = topics.entry(topic.clone()).or_default();
        let offset = queue.len() as i64;
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let msg = KafkaMessage {
            topic: topic.clone(),
            partition: 0,
            offset,
            key: key.map(|s| s.to_string()),
            value: value.to_vec(),
            timestamp_ms,
        };
        queue.push_back(msg.clone());
        *self.produced_count.lock().unwrap() += 1;
        msg
    }

    /// 消费一条 (FIFO, 非阻塞; None = topic 空)
    pub fn consume(&self, topic: &str) -> Option<KafkaMessage> {
        let mut topics = self.topics.lock().unwrap();
        let queue = topics.get_mut(topic)?;
        let msg = queue.pop_front()?;
        *self.consumed_count.lock().unwrap() += 1;
        Some(msg)
    }

    /// peek 一条 (不消费)
    pub fn peek(&self, topic: &str) -> Option<KafkaMessage> {
        let topics = self.topics.lock().unwrap();
        topics.get(topic)?.front().cloned()
    }

    /// 某 topic 当前队列长度
    pub fn queue_len(&self, topic: &str) -> usize {
        self.topics
            .lock()
            .unwrap()
            .get(topic)
            .map(|q| q.len())
            .unwrap_or(0)
    }

    /// 全部 topic 列表
    pub fn topic_list(&self) -> Vec<String> {
        self.topics.lock().unwrap().keys().cloned().collect()
    }

    /// 已 produce 总数
    pub fn produced(&self) -> u64 { *self.produced_count.lock().unwrap() }

    /// 已 consume 总数
    pub fn consumed(&self) -> u64 { *self.consumed_count.lock().unwrap() }

    /// 清空所有 topic
    pub fn clear(&self) {
        self.topics.lock().unwrap().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produce_and_consume_round_trip() {
        let k = MockKafka::new();
        let msg = k.produce("audit.event", Some("user-1"), b"{\"event_id\":\"...\"}");
        assert_eq!(msg.topic, "audit.event");
        assert_eq!(msg.offset, 0);
        assert_eq!(msg.value_str(), Some("{\"event_id\":\"...\"}"));

        let consumed = k.consume("audit.event").expect("one msg");
        assert_eq!(consumed.offset, 0);
        assert_eq!(consumed.value_str(), Some("{\"event_id\":\"...\"}"));
    }

    #[test]
    fn offset_increments_per_topic() {
        let k = MockKafka::new();
        k.produce("a", None, b"1");
        k.produce("a", None, b"2");
        k.produce("a", None, b"3");
        k.produce("b", None, b"x");
        assert_eq!(k.queue_len("a"), 3);
        assert_eq!(k.queue_len("b"), 1);

        let m1 = k.consume("a").unwrap();
        let m2 = k.consume("a").unwrap();
        let m3 = k.consume("a").unwrap();
        assert_eq!(m1.offset, 0);
        assert_eq!(m2.offset, 1);
        assert_eq!(m3.offset, 2);
    }

    #[test]
    fn consume_empty_returns_none() {
        let k = MockKafka::new();
        assert!(k.consume("never-produced").is_none());
    }

    #[test]
    fn peek_does_not_consume() {
        let k = MockKafka::new();
        k.produce("t", None, b"hello");
        let p1 = k.peek("t").unwrap();
        let p2 = k.peek("t").unwrap();
        assert_eq!(p1.value, p2.value);
        assert_eq!(k.queue_len("t"), 1);
    }

    #[test]
    fn topic_list_includes_all() {
        let k = MockKafka::new();
        k.produce("a", None, b"1");
        k.produce("b", None, b"2");
        let mut ts = k.topic_list();
        ts.sort();
        assert_eq!(ts, vec!["a", "b"]);
    }

    #[test]
    fn counters_track() {
        let k = MockKafka::new();
        k.produce("a", None, b"1");
        k.produce("a", None, b"2");
        k.consume("a");
        assert_eq!(k.produced(), 2);
        assert_eq!(k.consumed(), 1);
    }

    #[test]
    fn clear_drops_all() {
        let k = MockKafka::new();
        k.produce("a", None, b"1");
        k.clear();
        assert_eq!(k.queue_len("a"), 0);
    }
}
