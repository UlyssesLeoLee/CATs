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
//!
//! 已清理: `set_ex()` — 全仓零调用; `set()` + `expire()` 已完全覆盖其行为

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 过期时刻（绝对时间点），`None` = 不过期。
///
/// 2026-10-05 修正：原先存的是**相对时长** `Duration`，而 `is_expired` 只判断
/// `ttl.is_zero()` —— 于是 `expire(key, 60)` 存进去的 60s 永远不等于 0，
/// **任何 key 都不会过期**。改为在 `expire()` 时就记下绝对截止时刻。
type Expiry = Option<Instant>;

/// Redis mock (in-mem, 单实例简化)
#[derive(Default)]
pub struct MockRedis {
    kv: Mutex<HashMap<String, (String, Expiry)>>,
    sets: Mutex<HashMap<String, BTreeSet<String>>>,
    zsets: Mutex<HashMap<String, BTreeMap<String, f64>>>,
}

impl MockRedis {
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前 KV key 数量
    pub fn kv_len(&self) -> usize {
        self.kv.lock().unwrap().len()
    }

    /// 当前 set key 数量
    pub fn set_len(&self) -> usize {
        self.sets.lock().unwrap().len()
    }

    // ---- KV ----

    /// SET key value
    pub fn set(&self, key: impl Into<String>, value: impl Into<String>) {
        let mut kv = self.kv.lock().unwrap();
        kv.insert(key.into(), (value.into(), None));
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
    pub fn exists(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

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
    pub fn decr(&self, key: &str) -> i64 {
        self.incr_by(key, -1)
    }

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
    ///
    /// 记下**绝对截止时刻**（而非相对时长），否则无法判断"过了多久"。
    pub fn expire(&self, key: &str, ttl_secs: u64) -> bool {
        let mut kv = self.kv.lock().unwrap();
        if let Some((v, _)) = kv.get(key).cloned() {
            let deadline = Instant::now() + Duration::from_secs(ttl_secs);
            kv.insert(key.to_string(), (v, Some(deadline)));
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
        if s.insert(m.clone()) {
            1
        } else {
            0
        }
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
        self.sets
            .lock()
            .unwrap()
            .get(key)
            .map(|s| s.len())
            .unwrap_or(0)
    }

    // ---- ZSET ----

    /// ZADD key score member
    pub fn zadd(&self, key: impl Into<String>, member: impl Into<String>, score: f64) {
        let mut zsets = self.zsets.lock().unwrap();
        zsets
            .entry(key.into())
            .or_default()
            .insert(member.into(), score);
    }

    /// ZSCORE key member
    pub fn zscore(&self, key: &str, member: &str) -> Option<f64> {
        self.zsets.lock().unwrap().get(key)?.get(member).copied()
    }

    /// ZRANGE key start stop (按 score 升序)
    pub fn zrange(&self, key: &str, start: isize, stop: isize) -> Vec<String> {
        let zsets = self.zsets.lock().unwrap();
        let Some(z) = zsets.get(key) else {
            return vec![];
        };
        let mut v: Vec<(&String, &f64)> = z.iter().collect();
        v.sort_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal));
        let n = v.len() as isize;
        let s = if start < 0 {
            (n + start).max(0)
        } else {
            start.min(n)
        } as usize;
        let e = if stop < 0 {
            n + stop + 1
        } else {
            (stop + 1).min(n)
        } as usize;
        v[s..e.min(v.len())]
            .iter()
            .map(|(m, _)| m.to_string())
            .collect()
    }

    /// 清空所有
    pub fn clear(&self) {
        self.kv.lock().unwrap().clear();
        self.sets.lock().unwrap().clear();
        self.zsets.lock().unwrap().clear();
    }

    // ---- 内部 ----

    /// 判断是否已到过期时刻
    ///
    /// 2026-10-05 修正：原先形参是相对时长 `Duration`，函数体算了个 `_now` 又丢掉，
    /// 然后 `return ttl.is_zero()` —— 结果**非零 TTL 永远判为未过期**。
    /// 现在形参是 `expire()` 时记下的绝对截止时刻。
    fn is_expired(deadline: Instant) -> bool {
        Instant::now() >= deadline
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

    // 2026-10-05 新增。`expire_returns_bool` 只断言 `expire()` 的返回值，
    // 完全没有验证 key 是否真的会过期 —— 而在修复前它**永远不会**过期
    // （除 TTL=0 外），那个测试却是绿的。这里补上真正查行为的用例。
    //
    // 注意：只测 TTL=0 抓不住旧 bug —— 旧实现 `Duration::from_secs(0).is_zero()`
    // 为 true，TTL=0 照样会过期。旧 bug 是**非零 TTL 永远不过期**，
    // 所以下面第一个用例才是判别性的那个。
    #[test]
    fn is_expired_uses_absolute_deadline() {
        // 判定函数是私有的，但测试模块能直接调：过去的时刻必须判为过期。
        // 旧实现形参是相对时长、对"1 秒前"返回 is_zero()==false ⇒ 此断言会红。
        assert!(
            MockRedis::is_expired(Instant::now() - Duration::from_secs(1)),
            "已经过去的时刻必须判为过期"
        );
        assert!(
            !MockRedis::is_expired(Instant::now() + Duration::from_secs(60)),
            "尚未到来的时刻必须判为未过期"
        );
    }

    #[test]
    fn expired_key_is_gone_after_ttl_elapses() {
        let r = MockRedis::new();
        r.set("k", "v");
        assert!(r.expire("k", 1));
        assert_eq!(r.get("k").as_deref(), Some("v"), "TTL 内应当仍可读");
        std::thread::sleep(Duration::from_millis(1_100));
        assert!(r.get("k").is_none(), "TTL 走完后 key 必须读不到");
        assert!(!r.exists("k"));
        assert_eq!(r.kv_len(), 0, "已过期的 key 必须被逐出, 而不只是读不到");
    }

    #[test]
    fn ttl_zero_key_is_expired_and_evicted() {
        let r = MockRedis::new();
        r.set("k", "v");
        assert!(r.expire("k", 0));
        assert!(r.get("k").is_none(), "TTL 0 的 key 必须读作已过期");
        assert_eq!(r.kv_len(), 0, "已过期的 key 必须被逐出, 而不只是读不到");
    }

    #[test]
    fn unexpired_key_still_readable() {
        let r = MockRedis::new();
        r.set("k", "v");
        assert!(r.expire("k", 3600));
        assert_eq!(
            r.get("k").as_deref(),
            Some("v"),
            "TTL 1 小时的 key 不能被立刻判为过期"
        );
        assert!(r.exists("k"));
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
