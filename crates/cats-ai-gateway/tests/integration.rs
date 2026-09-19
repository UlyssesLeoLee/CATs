//! cats-ai-gateway 集成测试 (MVP 4 件套)
//!
//! - routing.rs: 4 provider 命中 + fallback 顺序
//! - quota.rs: 超限拒绝
//! - compliance.rs: Local 模式拒云端
//! - retry.rs: provider 第一次失败、第二次成功

use cats_ai_gateway::compliance::ComplianceMode;
use cats_ai_gateway::provider::{AiProvider, ChatMessage, ChatRequest};
use cats_ai_gateway::quota::{QuotaConfig, QuotaTracker};
use cats_ai_gateway::retry::RetryPolicy;
use cats_ai_gateway::router::Router;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

fn req(content: &str, model: &str) -> ChatRequest {
    ChatRequest {
        model: model.into(),
        messages: vec![ChatMessage {
            role: "user".into(),
            content: content.into(),
        }],
        fail_closed: false,
    }
}

/// Test 1: 4 provider 命中
#[tokio::test]
async fn test_routing_4_providers() {
    let openai = Arc::new(MockProviderAlwaysOK::new("openai")) as Arc<dyn AiProvider>;
    let anthropic = Arc::new(MockProviderAlwaysOK::new("anthropic")) as Arc<dyn AiProvider>;
    let gemini = Arc::new(MockProviderAlwaysOK::new("gemini")) as Arc<dyn AiProvider>;
    let deepseek = Arc::new(MockProviderAlwaysOK::new("deepseek")) as Arc<dyn AiProvider>;

    let router = Router::new(vec![
        ("openai".into(), openai.clone()),
        ("anthropic".into(), anthropic.clone()),
        ("gemini".into(), gemini.clone()),
        ("deepseek".into(), deepseek.clone()),
    ]);

    for model in &["openai", "anthropic", "gemini", "deepseek"] {
        let r = router.route(&req("hi", model)).await;
        assert!(r.is_ok(), "model {model} should route");
    }
}

/// Test 2: Router fallback 顺序 - 第一个失败, 第二个顶上
#[tokio::test]
async fn test_router_fallback_order() {
    let openai = Arc::new(MockProviderAlwaysFail::new("openai")) as Arc<dyn AiProvider>;
    let anthropic = Arc::new(MockProviderAlwaysOK::new("anthropic")) as Arc<dyn AiProvider>;

    let router = Router::with_fallback_order(vec![
        ("openai".into(), openai.clone()),
        ("anthropic".into(), anthropic.clone()),
    ]);
    let r = router.route(&req("hi", "openai")).await;
    assert!(r.is_ok(), "should fallback to anthropic");
}

/// Test 3: Quota 超限拒绝
#[tokio::test]
async fn test_quota_over_limit() {
    let tracker = QuotaTracker::new(QuotaConfig {
        max_tokens_per_minute: 100,
    });

    // 第 1 次: 80 token 通过
    assert!(tracker.try_consume("org-1", 80).is_ok());
    // 第 2 次: 80 token 超过 100 上限
    let r = tracker.try_consume("org-1", 80);
    assert!(r.is_err(), "should reject quota overflow");
}

/// Test 4: Compliance Local 模式拒云端
#[tokio::test]
async fn test_compliance_local_rejects_cloud() {
    let cm = ComplianceMode::Local;
    assert!(cm.allows("openai").is_err(), "Local mode rejects cloud provider");
    let cm = ComplianceMode::Cloud;
    assert!(cm.allows("openai").is_ok());
}

/// Test 5: Retry 第一次失败、第二次成功
#[tokio::test]
async fn test_retry_first_fail_second_ok() {
    struct FlakyProvider {
        calls: Arc<AtomicU32>,
    }
    #[async_trait::async_trait]
    impl AiProvider for FlakyProvider {
        fn name(&self) -> &str { "flaky" }
        fn cost_per_1k_tokens(&self) -> f64 { 1.0 }
        async fn chat(&self, _req: ChatRequest) -> Result<cats_ai_gateway::provider::ChatResponse, cats_ai_gateway::error::ProviderError> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            if n == 0 {
                Err(cats_ai_gateway::error::ProviderError::Upstream("fail".into()))
            } else {
                Ok(cats_ai_gateway::provider::ChatResponse {
                    target_text: "ok".into(),
                    prompt_tokens: 1,
                    completion_tokens: 1,
                })
            }
        }
    }

    let provider: Arc<dyn AiProvider> = Arc::new(FlakyProvider { calls: Arc::new(AtomicU32::new(0)) });
    let policy = RetryPolicy::default();

    let result = policy.execute(&provider, req("hi", "flaky")).await;
    assert!(result.is_ok(), "should succeed after 1 retry");
}

// --- mock impls ---

struct MockProviderAlwaysOK {
    name: String,
}
impl MockProviderAlwaysOK {
    fn new(name: &str) -> Self { Self { name: name.into() } }
}
#[async_trait::async_trait]
impl AiProvider for MockProviderAlwaysOK {
    fn name(&self) -> &str { &self.name }
    fn cost_per_1k_tokens(&self) -> f64 { 1.0 }
    async fn chat(&self, _req: ChatRequest) -> Result<cats_ai_gateway::provider::ChatResponse, cats_ai_gateway::error::ProviderError> {
        Ok(cats_ai_gateway::provider::ChatResponse {
            target_text: format!("ok from {}", self.name),
            prompt_tokens: 1,
            completion_tokens: 1,
        })
    }
}

struct MockProviderAlwaysFail {
    name: String,
}
impl MockProviderAlwaysFail {
    fn new(name: &str) -> Self { Self { name: name.into() } }
}
#[async_trait::async_trait]
impl AiProvider for MockProviderAlwaysFail {
    fn name(&self) -> &str { &self.name }
    fn cost_per_1k_tokens(&self) -> f64 { 1.0 }
    async fn chat(&self, _req: ChatRequest) -> Result<cats_ai_gateway::provider::ChatResponse, cats_ai_gateway::error::ProviderError> {
        Err(cats_ai_gateway::error::ProviderError::Upstream("always fail".into()))
    }
}