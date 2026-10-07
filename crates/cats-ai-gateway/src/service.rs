//! AI 网关业务编排 (核心 service)
//!
//! 组合 router / quota / retry / compliance, 提供统一的 `chat()` 入口

use std::sync::Arc;

use crate::compliance::{ComplianceGate, ComplianceMode};
use crate::error::ProviderError;
use crate::provider::{ChatRequest, ChatResponse};
use crate::quota::QuotaTracker;
use crate::retry::{execute_with_retry, RetryOutcome, RetryPolicy};
use crate::router::Router;

/// AI 网关业务编排 (核心 service)
pub struct AiGatewayService {
    router: Router,
    quota: QuotaTracker,
    /// 重试策略。**当前恒为 `RetryPolicy::default()`**（max_attempts=3 / base_backoff=100ms）：
    /// 它没有任何 env / Config 入口，唯一能设置它的 `with_retry_policy` 已在 2026-10-06
    /// 作为零引用代码删除。要做成可配需要补 `Config` 字段 + env + Helm values + compose，
    /// 属新增功能，不在死代码清理范围内。这里写明事实，以免下一个读代码的人
    /// 误以为它已经可配。
    retry_policy: RetryPolicy,
    compliance: ComplianceGate,
}

impl AiGatewayService {
    /// 构造 (默认 Cloud 模式, 默认 RetryPolicy)
    pub fn new(router: Router, quota: QuotaTracker) -> Self {
        Self {
            router,
            quota,
            retry_policy: RetryPolicy::default(),
            compliance: ComplianceGate::new(ComplianceMode::Cloud),
        }
    }

    /// 设置合规模式
    pub fn with_compliance(mut self, mode: ComplianceMode) -> Self {
        self.compliance.set_mode(mode);
        self
    }

    // 2026-10-05 死代码清理：删除 `with_retry_policy`。
    // 全仓零调用；生产路径 (`main.rs`) 只走 `AiGatewayService::new(...).with_compliance(...)`。
    // 删除后 `retry_policy` 字段恒为 `RetryPolicy::default()`（见字段上的注释）。

    /// 引用 router
    pub fn router(&self) -> &Router {
        &self.router
    }

    /// 引用 quota
    pub fn quota(&self) -> &QuotaTracker {
        &self.quota
    }

    /// 引用 compliance
    pub fn compliance(&self) -> &ComplianceGate {
        &self.compliance
    }

    /// 业务级 chat 调用:
    /// 1. 合规预检 (按 model 字段命中的 provider, 必须在合规模式内)
    /// 2. 配额预检 (estimated tokens = prompt_tokens 启发式估算)
    /// 3. retry + fallback
    /// 4. 成功后扣减配额
    ///
    /// `org_id` 用于配额记账
    pub async fn chat(
        &self,
        org_id: &str,
        req: &ChatRequest,
    ) -> Result<ChatResponse, ProviderError> {
        // 1. 合规预检
        let chain = self.router.fallback_chain(&req.model);
        if let Some(first) = chain.first() {
            // 仅检查链首 provider; 链中后续 provider 由 fallback 触发的合规策略另行处理
            // MVP 简化: 如果链首 provider 被合规 block, 直接拒绝 (不静默 fallback)
            self.compliance.check(first.name().as_str())?;
        } else {
            return Err(ProviderError::ProviderNotFound(req.model.clone()));
        }

        // 2. 配额预检 (启发式: 按消息字符数 / 4 估算 token, MVP 简化)
        let estimated_tokens = estimate_tokens(req);
        self.quota.check(org_id, estimated_tokens).await?;

        // 3. retry + fallback
        let outcome =
            execute_with_retry(&self.router, &retry_policy_via(&self.retry_policy), req).await;

        match outcome {
            RetryOutcome::Ok { response, .. } => {
                // 4. 扣减配额
                self.quota.commit(org_id, response.total_tokens).await;
                Ok(response)
            }
            RetryOutcome::Err(e) => Err(e),
        }
    }
}

/// RetryPolicy 引用包装 (避免在 AiGatewayService 上持有 RetryPolicy 引用导致的 lifetime 麻烦)
fn retry_policy_via(p: &RetryPolicy) -> RetryPolicy {
    p.clone()
}

/// 启发式 token 估算 (per 任务要求 7.3 配额)
fn estimate_tokens(req: &ChatRequest) -> u32 {
    let total_chars: usize = req.messages.iter().map(|m| m.content.len()).sum();
    // 4 字符 ≈ 1 token (经验值, MVP)
    let prompt = (total_chars / 4).max(1) as u32;
    // 上限 max_tokens 输出估算
    let completion = (req.max_tokens / 4).max(1);
    prompt + completion
}

/// 工厂: 用默认 4 mock provider 构造完整 service
pub fn default_service() -> AiGatewayService {
    use crate::provider::{anthropic, deepseek, gemini, openai};
    let router = Router::new(vec![openai(), anthropic(), gemini(), deepseek()]);
    AiGatewayService::new(router, QuotaTracker::new())
}

/// 工厂: 测试用 — 注入自定义 provider 列表 + 配额 + 合规模式
pub fn service_for_test(
    providers: Vec<Arc<dyn crate::provider::AiProvider>>,
    quota: QuotaTracker,
    mode: ComplianceMode,
) -> AiGatewayService {
    let router = Router::new(providers);
    AiGatewayService::new(router, quota).with_compliance(mode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{ChatMessage, ProviderName};

    #[tokio::test]
    async fn full_flow_success_charges_quota() {
        let svc = default_service();
        let req = ChatRequest {
            model: "gpt-4o-mini".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "translate hello".into(),
            }],
            temperature: 0.7,
            max_tokens: 1024,
            idempotency_key: "".into(),
        };
        let resp = svc.chat("org-1", &req).await.unwrap();
        assert_eq!(resp.provider, "openai");
        assert!(resp.total_tokens > 0);

        // 配额已扣减
        let usage = svc.quota().usage("org-1").await;
        assert!(usage.tokens_used > 0);
    }

    #[tokio::test]
    async fn local_mode_blocks_cloud_provider() {
        let svc = default_service().with_compliance(ComplianceMode::Local);
        let req = ChatRequest {
            model: "gpt-4o-mini".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "hi".into(),
            }],
            temperature: 0.7,
            max_tokens: 1024,
            idempotency_key: "".into(),
        };
        let err = svc.chat("org-1", &req).await.unwrap_err();
        assert!(matches!(err, ProviderError::ComplianceBlocked { .. }));
    }

    #[tokio::test]
    async fn quota_exceeded_short_circuits_before_provider_call() {
        // 用极小配额 (1 token), 即便 prompt 估算也超限
        // 走 `service_for_test` 注入单 provider + 配额 (Cloud = `new()` 的默认值)
        let small_quota = QuotaTracker::with_limit(1);
        let svc = service_for_test(
            vec![crate::provider::openai()],
            small_quota,
            ComplianceMode::Cloud,
        );
        let req = ChatRequest {
            model: "gpt-4o-mini".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "x".repeat(1000),
            }],
            temperature: 0.7,
            max_tokens: 1024,
            idempotency_key: "".into(),
        };
        let err = svc.chat("org-1", &req).await.unwrap_err();
        assert!(matches!(err, ProviderError::RateLimited { .. }));
    }

    #[tokio::test]
    async fn unknown_model_falls_back_to_default_provider() {
        let svc = default_service();
        let req = ChatRequest {
            model: "unknown-xyz".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "hi".into(),
            }],
            temperature: 0.7,
            max_tokens: 1024,
            idempotency_key: "".into(),
        };
        // fallback chain 会返回 default (openai), 所以这里不会 ProviderNotFound
        // 但 mock openai 会返回 — 应当 Ok
        //
        // 之前的写法是 `svc.chat(...).await.unwrap_err()` 然后断言
        // `ProviderNotFound || contains("openai")` —— 自相矛盾：注释和断言都说
        // 可能返回 Ok，unwrap_err() 却会在这里直接 panic。改为如实断言实际行为。
        match svc.chat("org-1", &req).await {
            Ok(resp) => {
                assert_eq!(
                    resp.provider, "openai",
                    "unknown model should fall back to the default provider"
                );
            }
            Err(ProviderError::ProviderNotFound(m)) => {
                panic!("unexpected ProviderNotFound for {m}: router falls back instead")
            }
            Err(other) => panic!("unexpected error: {other}"),
        }
        let _ = ProviderName::OpenAi; // suppress unused
    }
}
