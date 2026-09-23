//! AI 网关路由器
//!
//! 引用: doc/02-基础设计/决策/CATs_ADR-010_LLM部署与推理策略_v1.0.md §4
//!   - 路由策略: 项目级配置 `compliance_mode` 决定走本地还是云端
//!   - 主备顺序: OpenAI → Anthropic → Gemini → DeepSeek
//!   - 模型按 `model` 字段命中, fallback 按 ProviderName::FALLBACK_ORDER 顺序
//!
//! MVP 行为:
//! - find_by_model(req.model): 命中 → 调该 provider; 未命中 → 走 FALLBACK_ORDER 顺序
//! - 顺序回退由 retry 模块组合完成, router 本身只暴露"按 model 拿 provider"
//! - 找不到 → ProviderNotFound 错误

use std::collections::HashMap;
use std::sync::Arc;

use crate::error::ProviderError;
use crate::provider::{AiProvider, ChatRequest, ChatResponse, ProviderName};

/// 路由器
pub struct Router {
    /// 按 model 索引 provider (model 名 → provider)
    by_model: HashMap<String, Arc<dyn AiProvider>>,
    /// 按 provider 名索引 (用于 fallback 顺序遍历)
    by_name: HashMap<ProviderName, Arc<dyn AiProvider>>,
}

impl Router {
    /// 构造路由器 (从 CompositeMockProvider 取索引, 也可以直接传 provider 列表)
    pub fn new(providers: Vec<Arc<dyn AiProvider>>) -> Self {
        let mut by_model = HashMap::new();
        let mut by_name = HashMap::new();
        for p in &providers {
            by_name.insert(p.name(), p.clone());
            for m in p.supported_models() {
                by_model.insert((*m).to_string(), p.clone());
            }
        }
        Self { by_model, by_name }
    }

    /// 按 model 字段找 provider (命中第一个)
    pub fn find_by_model(&self, model: &str) -> Option<Arc<dyn AiProvider>> {
        self.by_model.get(model).cloned()
    }

    /// 按 provider 名找
    pub fn find_by_name(&self, name: ProviderName) -> Option<Arc<dyn AiProvider>> {
        self.by_name.get(&name).cloned()
    }

    /// 按 fallback 顺序返回 provider 列表 (per ADR-010 §4 顺序回退)
    ///
    /// 输入是 req.model, 输出是"如果这个 model 不命中, 应依次尝试的 provider 列表"
    /// MVP 策略: 第一个是 model 命中的, 后续是 ProviderName::FALLBACK_ORDER 顺序去重
    pub fn fallback_chain(&self, model: &str) -> Vec<Arc<dyn AiProvider>> {
        let mut chain = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // 1. model 命中的 provider 优先
        if let Some(p) = self.find_by_model(model) {
            if seen.insert(p.name()) {
                chain.push(p);
            }
        }

        // 2. 按 FALLBACK_ORDER 追加其余 provider
        for name in ProviderName::FALLBACK_ORDER {
            if let Some(p) = self.by_name.get(name) {
                if seen.insert(p.name()) {
                    chain.push(p.clone());
                }
            }
        }

        chain
    }

    /// 单 provider 直调 (不触发 fallback), 给 router 单点测试 / 单 provider 单元使用
    pub async fn route_single(&self, req: &ChatRequest) -> Result<ChatResponse, ProviderError> {
        let provider = self
            .find_by_model(&req.model)
            .ok_or_else(|| ProviderError::ProviderNotFound(req.model.clone()))?;
        provider.chat(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{anthropic, deepseek, gemini, openai, CompositeMockProvider};

    fn test_router() -> Router {
        let c = CompositeMockProvider::default_4();
        let _ = c; // suppress unused
        Router::new(vec![openai(), anthropic(), gemini(), deepseek()])
    }

    #[test]
    fn find_by_model_returns_correct_provider() {
        let r = test_router();
        let p = r.find_by_model("gpt-4o-mini").unwrap();
        assert_eq!(p.name(), ProviderName::OpenAi);
        let p = r.find_by_model("claude-3-5-sonnet").unwrap();
        assert_eq!(p.name(), ProviderName::Anthropic);
    }

    #[test]
    fn find_by_unknown_model_returns_none() {
        let r = test_router();
        assert!(r.find_by_model("unknown-model-xyz").is_none());
    }

    #[test]
    fn fallback_chain_starts_with_hit_then_follows_order() {
        let r = test_router();
        let chain = r.fallback_chain("claude-3-5-sonnet");
        let names: Vec<_> = chain.iter().map(|p| p.name()).collect();
        assert_eq!(names, vec![
            ProviderName::Anthropic,
            ProviderName::OpenAi,    // FALLBACK_ORDER 后续
            ProviderName::Gemini,
            ProviderName::DeepSeek,
        ]);
    }

    #[test]
    fn fallback_chain_unknown_model_falls_back_to_order() {
        let r = test_router();
        let chain = r.fallback_chain("unknown-xyz");
        let names: Vec<_> = chain.iter().map(|p| p.name()).collect();
        assert_eq!(names, vec![
            ProviderName::OpenAi,
            ProviderName::Anthropic,
            ProviderName::Gemini,
            ProviderName::DeepSeek,
        ]);
    }

    #[tokio::test]
    async fn route_single_unknown_model_returns_provider_not_found() {
        let r = test_router();
        let err = r.route_single(&ChatRequest {
            model: "unknown-xyz".into(),
            messages: vec![],
            temperature: 0.7,
            max_tokens: 1024,
            idempotency_key: "".into(),
        }).await.unwrap_err();
        match err {
            ProviderError::ProviderNotFound(m) => assert_eq!(m, "unknown-xyz"),
            other => panic!("expected ProviderNotFound, got {:?}", other),
        }
    }
}