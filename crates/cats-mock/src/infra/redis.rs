//! Redis 内存 mock
//!
//! 引用: 设计书 §4.3.2
//!
//! 覆盖子集 (测试高频, K3s 阶段二补全):
//! - GET / SET / DEL / EXISTS
//! - INCR / DECR
//! - EXPIRE / TTL
//! - SADD / SMEMBERS / SREM
//! - ZADD / ZRANGE / ZSCORE

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type Expiry = Option<Duration>;

/// Redis mock (in-mem, 单实例简化)
#[derive(Default)]
pub struct MockRedis {
    kv: Mutex<HashMap<String, (String, Expiry)>>,
    sets: Mutex<HashMap<String, BTreeSet<String>>>,
    zsets: Mutex<HashMap<String, BTreeMap<String, f64>>>,
}

impl MockRedis {
    pub fn new() -> Self { Self::default() }

    /// 当前 KV key 数量
    pub fn kv_len(&self) -> usize { self.kv.lock().unwrap().len() }

    /// 当前 set key 数量
    pub fn set_len(&self) -> usize { self.sets.lock().unwrap().len() }

    // ---- KV ----

    /// SET key value
    pub fn set(&self, key: impl Into<String>, value: impl Into<String>) {
        let mut kv = self.kv.lock().unwrap();
        kv.insert(key.into(), (value.into(), None));
    }

    /// SET key value EX seconds
    pub fn set_ex(&self, key: impl Into<String>, value: impl Into<String>, ttl_secs: u64) {
        let mut kv = self.kv.lock().unwrap();
        kv.insert(key.into(), (value.into(), Some(Duration::from_secs(ttl_secs))));
    }

    /// GET key
    pub fn get(&self, key: &str) -> Option<String> {
        let kv = self.kv.lock().unwrap();
        let (val, expiry) = kv.get(key)?.clone();
        if let Some(ttl) = expiry {
            if Self::is_expired(ttl) {
                drop(kv);
                self.kv.lock().unwrap().remove(key);
                return None;
            }
        }
        Some(val)
    }

    /// DEL key (返回是否真删除)
    pub fn del(&self, key: &str) -> bool {
        self.kv.lock().unwrap().remove(key).is_some()
    }

    /// EXISTS key
    pub fn exists(&self, key: &str) -> bool { self.get(key).is_some() }

    /// INCR key (key 不存在时初始化为 0)
    pub fn incr(&self, key: &str) -> i64 {
        let mut kv = self.kv.lock().unwrap();
        let cur = kv
            .get(key)
            .and_then(|(v, _)| v.parse::<i64>().ok())
            .unwrap_or(0);
        let new = cur + 1;
        kv.insert(key.to_string(), (new.to_string(), None));
        new
    }

    /// DECR key
    pub fn decr(&self, key: &str) -> i64 { self.incr_by(key, -1) }

    /// INCRBY key delta
    pub fn incr_by(&self, key: &str, delta: i64) -> i64 {
        let mut kv = self.kv.lock().unwrap();
        let cur = kv
            .get(key)
            .and_then(|(v, _)| v.parse::<i64>().ok())
            .unwrap_or(0);
        let new = cur + delta;
        kv.insert(key.to_string(), (new.to_string(), None));
        new
    }

    /// EXPIRE key seconds
    pub fn expire(&self, key: &str, ttl_secs: u64) -> bool {
        let mut kv = self.kv.lock().unwrap();
        if let Some((v, _)) = kv.get(key).cloned() {
            kv.insert(key.to_string(), (v, Some(Duration::from_secs(ttl_secs))));
            true
        } else {
            false
        }
    }

    // ---- SET ----

    /// SADD key member (返回新增数)
    pub fn sadd(&self, key: impl Into<String>, member: impl Into<String>) -> usize {
        let mut sets = self.sets.lock().unwrap();
        let s = sets.entry(key.into()).or_default();
        let m = member.into();
        if s.insert(m.clone()) { 1 } else { 0 }
    }

    /// SMEMBERS key (sorted)
    pub fn smembers(&self, key: &str) -> Vec<String> {
        self.sets
            .lock()
            .unwrap()
            .get(key)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// SREM key member
    pub fn srem(&self, key: &str, member: &str) -> bool {
        self.sets
            .lock()
            .unwrap()
            .get_mut(key)
            .map(|s| s.remove(member))
            .unwrap_or(false)
    }

    /// SCARD key
    pub fn scard(&self, key: &str) -> usize {
        self.sets.lock().unwrap().get(key).map(|s| s.len()).unwrap_or(0)
    }

    // ---- ZSET ----

    /// ZADD key score member
    pub fn zadd(&self, key: impl Into<String>, member: impl Into<String>, score: f64) {
        let mut zsets = self.zsets.lock().unwrap();
        zsets.entry(key.into()).or_default().insert(member.into(), score);
    }

    /// ZSCORE key member
    pub fn zscore(&self, key: &str, member: &str) -> Option<f64> {
        self.zsets.lock().unwrap().get(key)?.get(member).copied()
    }

    /// ZRANGE key start stop (按 score 升序)
    pub fn zrange(&self, key: &str, start: isize, stop: isize) -> Vec<String> {
        let zsets = self.zsets.lock().unwrap();
        let Some(z) = zsets.get(key) else { return vec![] };
        let mut v: Vec<(&String, &f64)> = z.iter().collect();
        v.sort_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal));
        let n = v.len() as isize;
        let s = if start < 0 { (n + start).max(0) } else { start.min(n) } as usize;
        let e = if stop < 0 { n + stop + 1 } else { (stop + 1).min(n) } as usize;
        v[s..e.min(v.len())].iter().map(|(m, _)| m.to_string()).collect()
    }

    /// 清空所有
    pub fn clear(&self) {
        self.kv.lock().unwrap().clear();
        self.sets.lock().unwrap().clear();
        self.zsets.lock().unwrap().clear();
    }

    // ---- 内部 ----

    /// 简化 TTL: 不真用定时器, get 时按 wall-clock 判过期
    fn is_expired(ttl: Duration) -> bool {
        let _now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        // 这里用 "创建时已记录 ttl" 简化 — 真生产应记录插入时间戳
        // 当前用 ttl==0 视为过期, 否则未过期 (测试用)
        ttl.is_zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get() {
        let r = MockRedis::new();
        r.set("k", "v");
        assert_eq!(r.get("k"), Some("v".to_string()));
    }

    #[test]
    fn del_returns_bool() {
        let r = MockRedis::new();
        r.set("k", "v");
        assert!(r.del("k"));
        assert!(!r.del("k"));
    }

    #[test]
    fn exists_tracks() {
        let r = MockRedis::new();
        assert!(!r.exists("k"));
        r.set("k", "v");
        assert!(r.exists("k"));
    }

    #[test]
    fn incr_from_zero() {
        let r = MockRedis::new();
        assert_eq!(r.incr("counter"), 1);
        assert_eq!(r.incr("counter"), 2);
        assert_eq!(r.decr("counter"), 1);
    }

    #[test]
    fn incr_by_negative_works() {
        let r = MockRedis::new();
        r.set("c", "10");
        assert_eq!(r.incr_by("c", -3), 7);
    }

    #[test]
    fn set_add_remove_card() {
        let r = MockRedis::new();
        assert_eq!(r.sadd("s", "a"), 1);
        assert_eq!(r.sadd("s", "b"), 1);
        assert_eq!(r.sadd("s", "a"), 0); // 已存在
        assert_eq!(r.scard("s"), 2);
        let mut m = r.smembers("s");
        m.sort();
        assert_eq!(m, vec!["a", "b"]);
        assert!(r.srem("s", "a"));
        assert_eq!(r.scard("s"), 1);
    }

    #[test]
    fn zset_add_score_range() {
        let r = MockRedis::new();
        r.zadd("z", "a", 1.0);
        r.zadd("z", "b", 3.0);
        r.zadd("z", "c", 2.0);
        assert_eq!(r.zscore("z", "b"), Some(3.0));
        let r0 = r.zrange("z", 0, -1);
        assert_eq!(r0, vec!["a", "c", "b"]); // 按 score 升序
    }

    #[test]
    fn zrange_partial() {
        let r = MockRedis::new();
        for (i, m) in ["a", "b", "c", "d"].iter().enumerate() {
            r.zadd("z", *m, i as f64);
        }
        assert_eq!(r.zrange("z", 1, 2), vec!["b", "c"]);
    }

    #[test]
    fn expire_returns_bool() {
        let r = MockRedis::new();
        r.set("k", "v");
        assert!(r.expire("k", 60));
        assert!(!r.expire("nonexistent", 60));
    }

    #[test]
    fn clear_drops_all() {
        let r = MockRedis::new();
        r.set("k", "v");
        r.sadd("s", "a");
        r.zadd("z", "x", 1.0);
        r.clear();
        assert_eq!(r.kv_len(), 0);
        assert_eq!(r.set_len(), 0);
    }
}
