//! Provider 抽象层
//!
//! 4 mock provider (OpenAI / Anthropic / Gemini / DeepSeek) + 1 CompositeMockProvider (按 model 路由)
//!
//! 引用: doc/02-基础设计/决策/CATs_ADR-010_LLM部署与推理策略_v1.0.md §4 (4 provider 顺序回退)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::ProviderError;

/// Provider 名称 (强类型, 防拼写错)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderName {
    /// OpenAI / GPT 系列
    #[serde(rename = "openai")]
    OpenAi,
    /// Anthropic / Claude 系列
    #[serde(rename = "anthropic")]
    Anthropic,
    /// Google Gemini
    #[serde(rename = "gemini")]
    Gemini,
    /// DeepSeek
    #[serde(rename = "deepseek")]
    DeepSeek,
    /// Local vLLM (per ADR-010 §3.3 选项 C, MVP 未实现)
    #[serde(rename = "local")]
    Local,
}

impl ProviderName {
    /// 转字符串
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAi    => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini    => "gemini",
            Self::DeepSeek  => "deepseek",
            Self::Local     => "local",
        }
    }

    /// 从字符串解析
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "openai"    => Some(Self::OpenAi),
            "anthropic" => Some(Self::Anthropic),
            "gemini"    => Some(Self::Gemini),
            "deepseek"  => Some(Self::DeepSeek),
            "local"     => Some(Self::Local),
            _ => None,
        }
    }

    /// 路由顺序 (per ADR-010 §4 主备顺序)
    pub const FALLBACK_ORDER: &'static [ProviderName] = &[
        ProviderName::OpenAi,
        ProviderName::Anthropic,
        ProviderName::Gemini,
        ProviderName::DeepSeek,
    ];
}

/// Chat 请求 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// 模型名 ("gpt-4o-mini" / "claude-3-5-sonnet" / "gemini-1.5-flash" / "deepseek-chat")
    pub model: String,
    /// 对话消息列表
    pub messages: Vec<ChatMessage>,
    /// 温度 (0.0 ~ 2.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// 最大输出 token 数
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// 幂等键 (per 接口设计 §1.1)
    #[serde(default)]
    pub idempotency_key: String,
}

fn default_temperature() -> f32 { 0.7 }
fn default_max_tokens() -> u32 { 1024 }

/// 单条 chat 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// role: "system" | "user" | "assistant"
    pub role: String,
    pub content: String,
}

/// Chat 响应 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// 唯一响应 ID
    pub id: String,
    /// 实际命中的 provider 名
    pub provider: String,
    /// 实际命中的 model
    pub model: String,
    /// 响应内容
    pub content: String,
    /// 输入 token 数 (模拟真实调用)
    pub prompt_tokens: u32,
    /// 输出 token 数 (模拟真实调用)
    pub completion_tokens: u32,
    /// 总 token 数
    pub total_tokens: u32,
    /// 命中链路 "openai->anthropic" (按 fallback 顺序记录)
    pub fallback_chain: String,
}

/// Provider trait (per 任务要求 7.1)
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// provider 名
    fn name(&self) -> ProviderName;

    /// 支持的 model 列表 (e.g. "gpt-4o-mini")
    fn supported_models(&self) -> &[&str];

    /// 每千 token 单价 (USD, mock 硬编码)
    fn cost_per_1k_tokens(&self) -> f64;

    /// 同步 chat 调用
    async fn chat(&self, req: &ChatRequest) -> Result<ChatResponse, ProviderError>;
}

// ─────────────────────────────────────────────────────────────────────────
// 4 个 mock provider
// 每个加载对应 JSON fixture (编译期嵌入), 根据 prompt_substring 匹配响应
// ─────────────────────────────────────────────────────────────────────────

/// Mock provider 通用实现 (4 provider 共享逻辑)
pub struct MockProvider {
    name: ProviderName,
    default_model: String,
    cost_per_1k_tokens: f64,
    responses: Vec<MockResponseEntry>,
}

/// fixture 单条响应
#[derive(Debug, Clone, Deserialize)]
struct MockResponseEntry {
    prompt_substring: String,
    content: String,
    prompt_tokens: u32,
    completion_tokens: u32,
    /// 模拟 latency (ms), MVP 不实现真等待, 仅记录
    #[allow(dead_code)]
    latency_ms: u32,
}

/// fixture 根结构
#[derive(Debug, Deserialize)]
struct MockProviderFixture {
    default_model: String,
    cost_per_1k_tokens_usd: f64,
    responses: Vec<MockResponseEntry>,
}

impl MockProvider {
    /// 加载 fixture (编译期 include_str! 嵌入)
    fn from_fixture(name: ProviderName, fixture_json: &'static str) -> Self {
        let fixture: MockProviderFixture = serde_json::from_str(fixture_json)
            .expect("fixture should parse (validated at compile time)");
        Self {
            name,
            default_model: fixture.default_model,
            cost_per_1k_tokens: fixture.cost_per_1k_tokens_usd,
            responses: fixture.responses,
        }
    }

    /// 按 prompt 内容匹配响应 (首个 prompt_substring 包含在 prompt 里 wins; 否则 fallback 到 "default")
    fn pick_response(&self, prompt: &str) -> &MockResponseEntry {
        self.responses
            .iter()
            .find(|r| r.prompt_substring != "default" && prompt.contains(&r.prompt_substring))
            .or_else(|| self.responses.iter().find(|r| r.prompt_substring == "default"))
            .expect("fixture should contain a 'default' response")
    }
}

#[async_trait]
impl AiProvider for MockProvider {
    fn name(&self) -> ProviderName { self.name }

    fn supported_models(&self) -> &[&str] {
        // MVP: 每个 provider 仅 1 个 model
        match self.name {
            ProviderName::OpenAi    => &["gpt-4o-mini"],
            ProviderName::Anthropic => &["claude-3-5-sonnet"],
            ProviderName::Gemini    => &["gemini-1.5-flash"],
            ProviderName::DeepSeek  => &["deepseek-chat"],
            ProviderName::Local     => &["qwen2.5-7b-instruct-awq"],
        }
    }

    fn cost_per_1k_tokens(&self) -> f64 { self.cost_per_1k_tokens }

    async fn chat(&self, req: &ChatRequest) -> Result<ChatResponse, ProviderError> {
        // 拼接 prompt (mock 简化: 取最后一条 user 消息的 content)
        let prompt = req
            .messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.as_str())
            .unwrap_or("");

        let entry = self.pick_response(prompt);
        let model = if req.model.is_empty() {
            self.default_model.clone()
        } else {
            req.model.clone()
        };

        Ok(ChatResponse {
            id: uuid::Uuid::new_v4().to_string(),
            provider: self.name.as_str().to_string(),
            model,
            content: format!("[mock-{}] {}", self.name.as_str(), entry.content),
            prompt_tokens: entry.prompt_tokens,
            completion_tokens: entry.completion_tokens,
            total_tokens: entry.prompt_tokens + entry.completion_tokens,
            fallback_chain: self.name.as_str().to_string(),
        })
    }
}

// fixture 文件编译期嵌入 (per 落地任务: mock_data 在 crates/cats-mock/mock_data/ai_gw/)
// 路径: src/provider/mod.rs → 上 3 层 → crates/ → cats-mock/mock_data/ai_gw/
const OPENAI_FIXTURE: &str    = include_str!("../../../cats-mock/mock_data/ai_gw/openai_responses.json");
const ANTHROPIC_FIXTURE: &str = include_str!("../../../cats-mock/mock_data/ai_gw/anthropic_responses.json");
const GEMINI_FIXTURE: &str    = include_str!("../../../cats-mock/mock_data/ai_gw/gemini_responses.json");
const DEEPSEEK_FIXTURE: &str  = include_str!("../../../cats-mock/mock_data/ai_gw/deepseek_responses.json");

/// 构造 OpenAI mock provider
pub fn openai() -> Arc<dyn AiProvider> {
    Arc::new(MockProvider::from_fixture(ProviderName::OpenAi, OPENAI_FIXTURE))
}

/// 构造 Anthropic mock provider
pub fn anthropic() -> Arc<dyn AiProvider> {
    Arc::new(MockProvider::from_fixture(ProviderName::Anthropic, ANTHROPIC_FIXTURE))
}

/// 构造 Gemini mock provider
pub fn gemini() -> Arc<dyn AiProvider> {
    Arc::new(MockProvider::from_fixture(ProviderName::Gemini, GEMINI_FIXTURE))
}

/// 构造 DeepSeek mock provider
pub fn deepseek() -> Arc<dyn AiProvider> {
    Arc::new(MockProvider::from_fixture(ProviderName::DeepSeek, DEEPSEEK_FIXTURE))
}

// ─────────────────────────────────────────────────────────────────────────
// CompositeMockProvider: 4 mock provider 聚合, 按 model 字段路由
// ─────────────────────────────────────────────────────────────────────────

/// 聚合 mock provider (按 model 字段路由到对应 mock)
pub struct CompositeMockProvider {
    by_model: HashMap<String, Arc<dyn AiProvider>>,
    default: Arc<dyn AiProvider>,
}

impl CompositeMockProvider {
    /// 构造默认 4 mock (主用 OpenAI)
    pub fn default_4() -> Self {
        let openai    = openai();
        let anthropic = anthropic();
        let gemini    = gemini();
        let deepseek  = deepseek();
        let mut by_model = HashMap::new();
        for p in [&openai, &anthropic, &gemini, &deepseek] {
            for m in p.supported_models() {
                by_model.insert((*m).to_string(), p.clone());
            }
        }
        Self { by_model, default: openai }
    }

    /// 自定义 4 provider 注入 (测试用, e.g. 用 failing stub 替换某一 provider)
    pub fn with_providers(providers: Vec<Arc<dyn AiProvider>>) -> Self {
        let mut by_model = HashMap::new();
        for p in &providers {
            for m in p.supported_models() {
                by_model.insert((*m).to_string(), p.clone());
            }
        }
        // 第一个 provider 作为 default
        let default = providers.into_iter().next().expect("at least one provider");
        Self { by_model, default }
    }

    /// 按 model 查 provider
    pub fn find_by_model(&self, model: &str) -> Option<Arc<dyn AiProvider>> {
        self.by_model.get(model).cloned()
    }
}

#[async_trait]
impl AiProvider for CompositeMockProvider {
    fn name(&self) -> ProviderName { ProviderName::OpenAi } // 复合体名义默认 OpenAI

    fn supported_models(&self) -> &[&str] {
        // 复合体返回所有 model (拼接), 但 Rust 不支持拼接静态切片 — 简化: 返回空
        &[]
    }

    fn cost_per_1k_tokens(&self) -> f64 { self.default.cost_per_1k_tokens() }

    async fn chat(&self, req: &ChatRequest) -> Result<ChatResponse, ProviderError> {
        let provider = self
            .find_by_model(&req.model)
            .unwrap_or_else(|| self.default.clone());
        provider.chat(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_name_parse_and_serialize() {
        for n in [
            ProviderName::OpenAi,
            ProviderName::Anthropic,
            ProviderName::Gemini,
            ProviderName::DeepSeek,
            ProviderName::Local,
        ] {
            assert_eq!(ProviderName::parse(n.as_str()), Some(n));
            let s = serde_json::to_string(&n).unwrap();
            assert_eq!(s, format!("\"{}\"", n.as_str()));
        }
    }

    #[test]
    fn fallback_order_is_openai_arthropic_gemini_deepseek() {
        assert_eq!(
            ProviderName::FALLBACK_ORDER,
            &[
                ProviderName::OpenAi,
                ProviderName::Anthropic,
                ProviderName::Gemini,
                ProviderName::DeepSeek,
            ]
        );
    }

    #[tokio::test]
    async fn openai_mock_returns_echo() {
        let p = openai();
        let resp = p.chat(&ChatRequest {
            model: "gpt-4o-mini".into(),
            messages: vec![ChatMessage { role: "user".into(), content: "translate hello".into() }],
            temperature: 0.7,
            max_tokens: 1024,
            idempotency_key: "".into(),
        }).await.unwrap();
        assert_eq!(resp.provider, "openai");
        assert!(resp.content.starts_with("[mock-openai]"));
        assert!(resp.total_tokens > 0);
    }

    #[tokio::test]
    async fn anthropic_mock_picks_translate_branch() {
        let p = anthropic();
        let resp = p.chat(&ChatRequest {
            model: "claude-3-5-sonnet".into(),
            messages: vec![ChatMessage { role: "user".into(), content: "Please translate".into() }],
            temperature: 0.7,
            max_tokens: 1024,
            idempotency_key: "".into(),
        }).await.unwrap();
        assert_eq!(resp.provider, "anthropic");
        assert!(resp.content.contains("High-quality translation"));
    }

    #[tokio::test]
    async fn composite_routes_by_model_field() {
        let c = CompositeMockProvider::default_4();
        for (model, expected_provider) in [
            ("gpt-4o-mini",          "openai"),
            ("claude-3-5-sonnet",    "anthropic"),
            ("gemini-1.5-flash",     "gemini"),
            ("deepseek-chat",        "deepseek"),
        ] {
            let resp = c.chat(&ChatRequest {
                model: model.into(),
                messages: vec![ChatMessage { role: "user".into(), content: "hello".into() }],
                temperature: 0.5,
                max_tokens: 512,
                idempotency_key: "".into(),
            }).await.unwrap();
            assert_eq!(resp.provider, expected_provider, "model={model}");
        }
    }

    #[tokio::test]
    async fn composite_falls_back_to_default_for_unknown_model() {
        let c = CompositeMockProvider::default_4();
        let resp = c.chat(&ChatRequest {
            model: "unknown-model-xyz".into(),
            messages: vec![ChatMessage { role: "user".into(), content: "hi".into() }],
            temperature: 0.5,
            max_tokens: 512,
            idempotency_key: "".into(),
        }).await.unwrap();
        // unknown model falls back to default (openai)
        assert_eq!(resp.provider, "openai");
    }
}