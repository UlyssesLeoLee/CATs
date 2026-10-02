//! cats-ai-gateway 集成测试 (MVP 4 件套)
//!
//! - router: 4 provider 命中 + fallback 顺序
//! - quota: 超限拒绝
//! - compliance: Local 模式拒云端
//! - retry: provider 第一次失败、第二次成功
//!
//! 这组测试此前是照着一套**并不存在**的 API 写的（`QuotaConfig`、
//! `Router::with_fallback_order`、`QuotaTracker::try_consume`、
//! `ComplianceMode::allows`、`RetryPolicy::execute`、`ChatRequest::fail_closed`、
//! `ChatResponse::target_text`），导致 `cargo check --all-targets` 报 24 个错误。
//! 现在全部对齐 `src/` 下的真实签名。

use cats_ai_gateway::compliance::{ComplianceGate, ComplianceMode};
use cats_ai_gateway::error::ProviderError;
use cats_ai_gateway::provider::{AiProvider, ChatMessage, ChatRequest, ChatResponse, ProviderName};
use cats_ai_gateway::quota::QuotaTracker;
use cats_ai_gateway::retry::{execute_with_retry, RetryOutcome, RetryPolicy};
use cats_ai_gateway::router::Router;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// 构造一个最小可用的 ChatRequest。
/// ChatRequest 只 derive 了 Debug/Clone/Serialize/Deserialize，没有 Default，
/// 所以 5 个字段必须显式给全。
fn req(content: &str, model: &str) -> ChatRequest {
    ChatRequest {
        model: model.into(),
        messages: vec![ChatMessage {
            role: "user".into(),
            content: content.into(),
        }],
        temperature: 0.7,
        max_tokens: 1024,
        idempotency_key: String::new(),
    }
}

fn ok_response(provider: ProviderName, model: &str) -> ChatResponse {
    ChatResponse {
        id: "test-id".into(),
        provider: provider.as_str().to_string(),
        model: model.to_string(),
        content: format!("ok from {}", provider.as_str()),
        prompt_tokens: 1,
        completion_tokens: 1,
        total_tokens: 2,
        fallback_chain: String::new(),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Test 1: 4 provider 命中
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_routing_4_providers() {
    let router = Router::new(vec![
        Arc::new(MockProviderAlwaysOK::new(
            ProviderName::OpenAi,
            "gpt-4o-mini",
        )),
        Arc::new(MockProviderAlwaysOK::new(
            ProviderName::Anthropic,
            "claude-3-5-sonnet",
        )),
        Arc::new(MockProviderAlwaysOK::new(
            ProviderName::Gemini,
            "gemini-1.5-flash",
        )),
        Arc::new(MockProviderAlwaysOK::new(
            ProviderName::DeepSeek,
            "deepseek-chat",
        )),
    ]);

    for model in [
        "gpt-4o-mini",
        "claude-3-5-sonnet",
        "gemini-1.5-flash",
        "deepseek-chat",
    ] {
        let r = router.route_single(&req("hi", model)).await;
        assert!(r.is_ok(), "model {model} should route");
    }

    // 4 个 provider 都在 by_name 里可查
    for name in [
        ProviderName::OpenAi,
        ProviderName::Anthropic,
        ProviderName::Gemini,
        ProviderName::DeepSeek,
    ] {
        assert!(
            router.find_by_name(name).is_some(),
            "provider {} should be registered",
            name.as_str()
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Test 2: Router fallback 顺序 —— 第一个失败, 第二个顶上
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_router_fallback_order() {
    let router = Router::new(vec![
        Arc::new(MockProviderAlwaysFail::new(
            ProviderName::OpenAi,
            "gpt-4o-mini",
        )),
        Arc::new(MockProviderAlwaysOK::new(
            ProviderName::Anthropic,
            "claude-3-5-sonnet",
        )),
    ]);

    // model 命中的 openai 排在链首, anthropic 按 FALLBACK_ORDER 跟在其后
    let chain = router.fallback_chain("gpt-4o-mini");
    assert_eq!(chain.len(), 2, "both providers should be in the chain");
    assert_eq!(
        chain[0].name(),
        ProviderName::OpenAi,
        "model hit goes first"
    );
    assert_eq!(chain[1].name(), ProviderName::Anthropic);

    // openai 恒失败 => 落到 anthropic, 且 fallback_chain 记录完整链路
    let outcome =
        execute_with_retry(&router, &RetryPolicy::default(), &req("hi", "gpt-4o-mini")).await;
    match outcome {
        RetryOutcome::Ok { response, chain } => {
            assert_eq!(response.provider, "anthropic");
            assert_eq!(chain, vec!["openai", "anthropic"]);
            assert_eq!(response.fallback_chain, "openai->anthropic");
        }
        RetryOutcome::Err(e) => panic!("expected fallback to anthropic, got {e}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Test 3: Quota 超限拒绝
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_quota_over_limit() {
    let tracker = QuotaTracker::with_limit(100);

    // 第 1 次: 预检 80 token 通过, 然后扣减
    assert!(tracker.check("org-1", 80).await.is_ok());
    tracker.commit("org-1", 80).await;

    // 第 2 次: 再要 80 token, projected = 160 > 100 => 拒绝
    let r = tracker.check("org-1", 80).await;
    match r {
        Err(ProviderError::RateLimited { org_id, .. }) => {
            assert_eq!(org_id, "org-1");
        }
        Err(other) => panic!("expected RateLimited, got {other}"),
        Ok(()) => panic!("should reject quota overflow"),
    }

    // 另一个 org 不受影响
    assert!(tracker.check("org-2", 80).await.is_ok());
}

// ─────────────────────────────────────────────────────────────────────────
// Test 4: Compliance Local 模式拒云端
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_compliance_local_rejects_cloud() {
    let local = ComplianceGate::new(ComplianceMode::Local);
    assert!(
        local.check("openai").is_err(),
        "Local mode must reject cloud provider"
    );

    let cloud = ComplianceGate::new(ComplianceMode::Cloud);
    assert!(
        cloud.check("openai").is_ok(),
        "Cloud mode must allow cloud provider"
    );

    // Local 模式应放行本机 provider
    assert!(local.check("local").is_ok());
}

// ─────────────────────────────────────────────────────────────────────────
// Test 5: Retry 第一次失败、第二次成功
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_retry_first_fail_second_ok() {
    let provider = Arc::new(FlakyProvider {
        calls: Arc::new(AtomicU32::new(0)),
    });
    let router = Router::new(vec![provider.clone()]);
    let policy = RetryPolicy::default();

    let outcome = execute_with_retry(&router, &policy, &req("hi", "flaky-model")).await;
    match outcome {
        RetryOutcome::Ok { response, .. } => {
            assert_eq!(response.content, "ok");
            assert!(
                provider.calls.load(Ordering::SeqCst) >= 2,
                "expected at least 2 attempts (1 fail + 1 ok)"
            );
        }
        RetryOutcome::Err(e) => panic!("expected success after retry, got {e}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// mocks
// ─────────────────────────────────────────────────────────────────────────

struct FlakyProvider {
    calls: Arc<AtomicU32>,
}

#[async_trait::async_trait]
impl AiProvider for FlakyProvider {
    fn name(&self) -> ProviderName {
        ProviderName::OpenAi
    }

    fn supported_models(&self) -> &[&str] {
        &["flaky-model"]
    }

    fn cost_per_1k_tokens(&self) -> f64 {
        1.0
    }

    async fn chat(&self, req: &ChatRequest) -> Result<ChatResponse, ProviderError> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n == 0 {
            // 必须是 Transient：ProviderError::is_retryable() 只对 Transient 返回
            // true，用 Upstream 会被 try_one_provider 立即判为不可重试而直接返回，
            // 根本走不到「第二次成功」。
            Err(ProviderError::Transient {
                provider: "flaky".into(),
                message: "fail".into(),
            })
        } else {
            let mut r = ok_response(self.name(), &req.model);
            r.content = "ok".into();
            Ok(r)
        }
    }
}

struct MockProviderAlwaysOK {
    name: ProviderName,
    models: &'static [&'static str],
}

impl MockProviderAlwaysOK {
    fn new(name: ProviderName, model: &'static str) -> Self {
        // AiProvider::supported_models 返回 &[&str]，借用必须指向 'static 数据，
        // 直接写 &[model] 会返回对临时数组的引用（E0515），所以存成 'static 切片。
        let models: &'static [&'static str] = Box::leak(Box::new([model]));
        Self { name, models }
    }
}

#[async_trait::async_trait]
impl AiProvider for MockProviderAlwaysOK {
    fn name(&self) -> ProviderName {
        self.name
    }

    fn supported_models(&self) -> &[&str] {
        self.models
    }

    fn cost_per_1k_tokens(&self) -> f64 {
        1.0
    }

    async fn chat(&self, req: &ChatRequest) -> Result<ChatResponse, ProviderError> {
        Ok(ok_response(self.name, &req.model))
    }
}

struct MockProviderAlwaysFail {
    name: ProviderName,
    models: &'static [&'static str],
}

impl MockProviderAlwaysFail {
    fn new(name: ProviderName, model: &'static str) -> Self {
        // 同上：supported_models 的借用必须是 'static，不能返回临时数组的引用。
        let models: &'static [&'static str] = Box::leak(Box::new([model]));
        Self { name, models }
    }
}

#[async_trait::async_trait]
impl AiProvider for MockProviderAlwaysFail {
    fn name(&self) -> ProviderName {
        self.name
    }

    fn supported_models(&self) -> &[&str] {
        self.models
    }

    fn cost_per_1k_tokens(&self) -> f64 {
        1.0
    }

    async fn chat(&self, _req: &ChatRequest) -> Result<ChatResponse, ProviderError> {
        // 同上：必须 Transient 才会被重试耗尽后 fallback 到下一个 provider。
        Err(ProviderError::Transient {
            provider: self.name.as_str().to_string(),
            message: "always fail".into(),
        })
    }
}
