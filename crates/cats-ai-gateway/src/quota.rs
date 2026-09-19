//! 配额跟踪器 (MVP 内存记账, 每分钟滑动窗口)
//!
//! 引用: doc/02-基础设计/决策/CATs_ADR-010_LLM部署与推理策略_v1.0.md §4
//!   - 配额：云端 API 按租户配额计费 (per ADR-005 多租户)
//!
//! MVP 简化:
//! - 用 `tokio::sync::Mutex<HashMap<org_id, QuotaState>>` 内存记账
//! - 每分钟 token 数限制: 100_000 tokens/min/org (MVP 硬编码)
//! - 滑动窗口实现: 维护 (window_start_ts, tokens_used_in_window)
//! - 跨分钟时重置计数
//! - 超限 → RATE_LIMITED 429 + Retry-After (秒)
//!
//! 不在范围: 多租户 RBAC / Redis 持久化 / 配额预警 / 用量计费

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::Mutex;

use crate::error::ProviderError;

/// 默认配额: 100_000 tokens / 分钟 / org (MVP 硬编码, per 任务要求 7.3)
pub const DEFAULT_TOKENS_PER_MIN: u32 = 100_000;

/// 单 org 配额状态
#[derive(Debug, Clone)]
struct QuotaState {
    /// 当前窗口起点 (unix timestamp, 秒)
    window_start_ts: u64,
    /// 当前窗口已用 token 数
    tokens_used: u64,
}

/// 配额跟踪器
#[derive(Clone)]
pub struct QuotaTracker {
    inner: Arc<Mutex<HashMap<String, QuotaState>>>,
    /// 每分钟 token 数上限 (org 维度)
    tokens_per_min: u32,
}

impl QuotaTracker {
    /// 构造默认配额 (100_000 tokens/min/org)
    pub fn new() -> Self {
        Self::with_limit(DEFAULT_TOKENS_PER_MIN)
    }

    /// 自定义上限 (测试用)
    pub fn with_limit(tokens_per_min: u32) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            tokens_per_min,
        }
    }

    /// 获取当前 unix timestamp (秒)
    fn now_ts() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
    }

    /// 当前分钟窗口起点 (向下取整到分钟)
    fn window_start(now: u64) -> u64 {
        now - (now % 60)
    }

    /// 预检: 申请 `tokens` 是否在配额内, 不扣减
    pub async fn check(&self, org_id: &str, tokens: u32) -> Result<(), ProviderError> {
        let mut guard = self.inner.lock().await;
        let now = Self::now_ts();
        let win = Self::window_start(now);

        let state = guard.entry(org_id.to_string()).or_insert(QuotaState {
            window_start_ts: win,
            tokens_used: 0,
        });

        // 跨分钟: 重置
        if state.window_start_ts != win {
            state.window_start_ts = win;
            state.tokens_used = 0;
        }

        let projected = state.tokens_used + tokens as u64;
        if projected > self.tokens_per_min as u64 {
            // 距离窗口结束的秒数 (用于 Retry-After)
            let retry_after_secs = (state.window_start_ts + 60).saturating_sub(now).max(1);
            return Err(ProviderError::RateLimited {
                org_id: org_id.to_string(),
                message: format!(
                    "quota exceeded: used={}, requested={}, limit={}/min",
                    state.tokens_used, tokens, self.tokens_per_min
                ),
                retry_after_secs: retry_after_secs as u32,
            });
        }

        Ok(())
    }

    /// 扣减配额 (调用成功后)
    pub async fn commit(&self, org_id: &str, tokens: u32) {
        let mut guard = self.inner.lock().await;
        let now = Self::now_ts();
        let win = Self::window_start(now);

        let state = guard.entry(org_id.to_string()).or_insert(QuotaState {
            window_start_ts: win,
            tokens_used: 0,
        });

        if state.window_start_ts != win {
            state.window_start_ts = win;
            state.tokens_used = 0;
        }

        state.tokens_used += tokens as u64;
    }

    /// 查询当前用量 (per GET /v1/llm/usage?org_id=...)
    pub async fn usage(&self, org_id: &str) -> UsageSnapshot {
        let guard = self.inner.lock().await;
        let now = Self::now_ts();
        let win = Self::window_start(now);

        match guard.get(org_id) {
            Some(s) if s.window_start_ts == win => UsageSnapshot {
                org_id: org_id.to_string(),
                window_start_ts: s.window_start_ts,
                tokens_used: s.tokens_used,
                tokens_limit: self.tokens_per_min as u64,
            },
            _ => UsageSnapshot {
                org_id: org_id.to_string(),
                window_start_ts: win,
                tokens_used: 0,
                tokens_limit: self.tokens_per_min as u64,
            },
        }
    }

    /// 重置某 org 配额 (测试用)
    #[cfg(test)]
    pub async fn reset(&self, org_id: &str) {
        let mut guard = self.inner.lock().await;
        guard.remove(org_id);
    }
}

impl Default for QuotaTracker {
    fn default() -> Self { Self::new() }
}

/// 用量快照 (GET /v1/llm/usage 响应)
#[derive(Debug, Clone, serde::Serialize)]
pub struct UsageSnapshot {
    pub org_id: String,
    pub window_start_ts: u64,
    pub tokens_used: u64,
    pub tokens_limit: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn first_request_within_limit_succeeds() {
        let q = QuotaTracker::new();
        q.check("org-1", 1000).await.unwrap();
        q.commit("org-1", 1000).await;
        let snap = q.usage("org-1").await;
        assert_eq!(snap.tokens_used, 1000);
    }

    #[tokio::test]
    async fn exceeding_limit_returns_rate_limited_with_retry_after() {
        let q = QuotaTracker::with_limit(100); // 小限额便于测试
        // 90 token 占用
        q.commit("org-1", 90).await;
        // 再 20 token → 110 > 100
        let err = q.check("org-1", 20).await.unwrap_err();
        match err {
            ProviderError::RateLimited { org_id, retry_after_secs, .. } => {
                assert_eq!(org_id, "org-1");
                assert!(retry_after_secs >= 1 && retry_after_secs <= 60);
            }
            other => panic!("expected RateLimited, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn reset_clears_window() {
        let q = QuotaTracker::with_limit(100);
        q.commit("org-1", 90).await;
        q.reset("org-1").await;
        q.check("org-1", 100).await.unwrap();
    }

    #[tokio::test]
    async fn usage_returns_zero_for_unknown_org() {
        let q = QuotaTracker::new();
        let snap = q.usage("unknown-org").await;
        assert_eq!(snap.tokens_used, 0);
        assert_eq!(snap.tokens_limit, DEFAULT_TOKENS_PER_MIN as u64);
    }
}