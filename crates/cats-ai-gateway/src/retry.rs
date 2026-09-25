//! 重试 + 降级策略
//!
//! 引用: doc/02-基础设计/决策/CATs_ADR-010_LLM部署与推理策略_v1.0.md §4
//!   - 云端故障 → 降级到本地 (若项目允许); 本地故障 → 返回明确错误, 不静默回云端
//!
//! MVP 简化:
//! - 指数退避: 100ms / 200ms / 400ms (3 次重试总 700ms 上限)
//! - 重试条件: ProviderError::is_retryable() == true (即 Transient 错误)
//! - fallback 顺序: 由 Router::fallback_chain 提供, retry 顺次遍历
//! - 全部失败 → ProviderError::AllFailed → HTTP 502 UPSTREAM_ERROR

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use crate::error::ProviderError;
use crate::provider::{AiProvider, ChatRequest, ChatResponse};
use crate::router::Router;

/// 重试 + 降级策略
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// 单 provider 最大重试次数 (per provider, 含首次)
    pub max_attempts_per_provider: u32,
    /// 基础退避时间 (ms), 指数倍增
    pub base_backoff_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        // per 任务要求 7.4: 指数退避, 最多 3 次
        Self {
            max_attempts_per_provider: 3,
            base_backoff_ms: 100,
        }
    }
}

impl RetryPolicy {
    /// 计算第 n 次重试的退避时间
    pub fn backoff(&self, attempt: u32) -> Duration {
        // attempt 0..max_attempts_per_provider-1
        // 100 * 2^attempt ms
        let ms = self.base_backoff_ms.saturating_mul(1u64 << attempt.min(10));
        Duration::from_millis(ms)
    }
}

/// 重试执行结果
pub enum RetryOutcome {
    /// 命中 provider + chat 成功
    Ok {
        response: ChatResponse,
        /// 命中链路 (per 任务要求 7.2 顺序回退记录)
        chain: Vec<String>,
    },
    /// 所有 provider 都失败
    Err(ProviderError),
}

/// 顺序遍历 fallback 链, 每个 provider 内部重试 N 次
pub async fn execute_with_retry(
    router: &Router,
    retry_policy: &RetryPolicy,
    req: &ChatRequest,
) -> RetryOutcome {
    let chain = router.fallback_chain(&req.model);
    let mut tried_chain: Vec<String> = Vec::new();

    for provider in chain.iter() {
        let provider_name = provider.name().as_str().to_string();
        tried_chain.push(provider_name.clone());

        let result = try_one_provider(provider, retry_policy, req).await;
        match result {
            Ok(resp) => {
                // 记录 fallback 链路
                let mut response = resp;
                response.fallback_chain = tried_chain.join("->");
                return RetryOutcome::Ok {
                    response,
                    chain: tried_chain,
                };
            }
            Err(e) => {
                // 非可重试 (Validation / Compliance / ProviderNotFound) 直接抛
                if !e.is_retryable() {
                    return RetryOutcome::Err(e);
                }
                // 否则继续下一个 provider
                tracing::warn!(
                    provider = %provider_name,
                    error = %e,
                    "provider failed after retries, falling back to next"
                );
            }
        }
    }

    RetryOutcome::Err(ProviderError::AllFailed(format!(
        "exhausted fallback chain: {}",
        tried_chain.join("->")
    )))
}

/// 单 provider 重试 (内部指数退避)
async fn try_one_provider(
    provider: &Arc<dyn AiProvider>,
    retry_policy: &RetryPolicy,
    req: &ChatRequest,
) -> Result<ChatResponse, ProviderError> {
    let mut last_err: Option<ProviderError> = None;

    for attempt in 0..retry_policy.max_attempts_per_provider {
        match provider.chat(req).await {
            Ok(resp) => return Ok(resp),
            Err(e) => {
                if !e.is_retryable() {
                    return Err(e);
                }
                last_err = Some(e);

                // 退避 (除最后一次外)
                if attempt + 1 < retry_policy.max_attempts_per_provider {
                    let backoff = retry_policy.backoff(attempt);
                    tracing::debug!(
                        provider = %provider.name().as_str(),
                        attempt = attempt + 1,
                        backoff_ms = backoff.as_millis() as u64,
                        "retrying after backoff"
                    );
                    sleep(backoff).await;
                }
            }
        }
    }

    // 重试耗尽, 返回最后一次错误
    Err(last_err.unwrap_or_else(|| ProviderError::Upstream {
        provider: provider.name().as_str().to_string(),
        message: "unknown error after retries".into(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{ChatMessage, ProviderName};
    use async_trait::async_trait;

    /// 第一次失败, 第二次成功的 mock (用于测试 retry+fallback)
    struct FlakyProvider {
        name: ProviderName,
        attempts: Arc<std::sync::Mutex<u32>>,
        succeed_after: u32,
    }

    #[async_trait]
    impl AiProvider for FlakyProvider {
        fn name(&self) -> ProviderName { self.name }
        fn supported_models(&self) -> &[&str] { &["flaky-model"] }
        fn cost_per_1k_tokens(&self) -> f64 { 0.0 }
        async fn chat(&self, _req: &ChatRequest) -> Result<ChatResponse, ProviderError> {
            let mut n = self.attempts.lock().unwrap();
            *n += 1;
            if *n < self.succeed_after {
                Err(ProviderError::Transient {
                    provider: self.name.as_str().to_string(),
                    message: "5xx".into(),
                })
            } else {
                Ok(ChatResponse {
                    id: "flaky-id".into(),
                    provider: self.name.as_str().to_string(),
                    model: "flaky-model".into(),
                    content: format!("[{}] ok", self.name.as_str()),
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                    fallback_chain: self.name.as_str().to_string(),
                })
            }
        }
    }

    /// 一直失败的 provider
    struct AlwaysFailProvider {
        name: ProviderName,
    }
    #[async_trait]
    impl AiProvider for AlwaysFailProvider {
        fn name(&self) -> ProviderName { self.name }
        fn supported_models(&self) -> &[&str] { &["fail-model"] }
        fn cost_per_1k_tokens(&self) -> f64 { 0.0 }
        async fn chat(&self, _req: &ChatRequest) -> Result<ChatResponse, ProviderError> {
            Err(ProviderError::Transient {
                provider: self.name.as_str().to_string(),
                message: "5xx".into(),
            })
        }
    }

    fn sample_req() -> ChatRequest {
        ChatRequest {
            model: "flaky-model".into(),
            messages: vec![ChatMessage { role: "user".into(), content: "hi".into() }],
            temperature: 0.7,
            max_tokens: 1024,
            idempotency_key: "".into(),
        }
    }

    #[tokio::test]
    async fn single_provider_succeeds_after_2_failures() {
        let attempts = Arc::new(std::sync::Mutex::new(0));
        let provider: Arc<dyn AiProvider> = Arc::new(FlakyProvider {
            name: ProviderName::OpenAi,
            attempts: attempts.clone(),
            succeed_after: 3, // 第 3 次成功 (attempt 0,1 fail, attempt 2 succeed)
        });
        let router = Router::new(vec![provider]);
        let outcome = execute_with_retry(&router, &RetryPolicy::default(), &sample_req()).await;
        match outcome {
            RetryOutcome::Ok { response, .. } => {
                assert_eq!(response.provider, "openai");
                assert_eq!(*attempts.lock().unwrap(), 3);
            }
            RetryOutcome::Err(e) => panic!("expected Ok, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn fallback_to_next_provider_after_all_failures() {
        // openai 一直 fail, anthropic 第一次成功
        let openai: Arc<dyn AiProvider> = Arc::new(AlwaysFailProvider { name: ProviderName::OpenAi });
        let anthropic: Arc<dyn AiProvider> = Arc::new(FlakyProvider {
            name: ProviderName::Anthropic,
            attempts: Arc::new(std::sync::Mutex::new(0)),
            succeed_after: 1, // 第 1 次就成功
        });
        let router = Router::new(vec![openai, anthropic]);
        let outcome = execute_with_retry(&router, &RetryPolicy::default(), &sample_req()).await;
        match outcome {
            RetryOutcome::Ok { response, chain } => {
                assert_eq!(response.provider, "anthropic");
                assert_eq!(chain, vec!["openai", "anthropic"]);
            }
            RetryOutcome::Err(e) => panic!("expected Ok, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn all_providers_fail_returns_all_failed() {
        let openai: Arc<dyn AiProvider> = Arc::new(AlwaysFailProvider { name: ProviderName::OpenAi });
        let anthropic: Arc<dyn AiProvider> = Arc::new(AlwaysFailProvider { name: ProviderName::Anthropic });
        let router = Router::new(vec![openai, anthropic]);
        let outcome = execute_with_retry(&router, &RetryPolicy::default(), &sample_req()).await;
        match outcome {
            RetryOutcome::Err(ProviderError::AllFailed(msg)) => {
                assert!(msg.contains("openai"));
                assert!(msg.contains("anthropic"));
            }
            other => panic!("expected AllFailed, got {:?}", other),
        }
    }
}