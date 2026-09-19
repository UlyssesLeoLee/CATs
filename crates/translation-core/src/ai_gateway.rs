//! translation-core AI 网关 trait
//!
//! MVP 阶段: 实现 `MockAiGateway` (无外部依赖, 返回 deterministic mock 答案).
//! Sprint 3: 替换为 `HttpAiGateway` 调 `cats-ai-gateway` REST endpoint.

use async_trait::async_trait;
use cats_common::{CatsError, ErrorCode};
use cats_proto::cats::v1::LanguageCode;

/// 一段翻译请求 (translation-core 内部表示)
#[derive(Debug, Clone)]
pub struct TranslateInput {
    pub tenant_id: String,
    pub project_id: String,
    pub segment_id: String,
    pub source_text: String,
    pub source_lang: LanguageCode,
    pub target_lang: LanguageCode,
    pub injected_terms: Vec<(String, String)>,
    pub model_provider: String,
    pub fail_closed: bool,
}

/// 一段翻译响应
#[derive(Debug, Clone)]
pub struct TranslateOutput {
    pub target_text: String,
    pub model_used: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub confidence: f32,
}

#[async_trait]
pub trait AiGateway: Send + Sync {
    async fn translate(&self, input: TranslateInput) -> Result<TranslateOutput, CatsError>;
}

/// Mock AI 网关 - Sprint 2 MVP
///
/// 行为:
/// - fail_closed=true 时, 任何源文 = 拒绝 (`COMPLIANCE_BLOCKED`)
/// - 否则返回 `[LANG:<target>] <source_text>` 占位译文, 1 token
/// - model_used = "mock"
pub struct MockAiGateway;

#[async_trait]
impl AiGateway for MockAiGateway {
    async fn translate(&self, input: TranslateInput) -> Result<TranslateOutput, CatsError> {
        if input.fail_closed {
            // 模拟合规拦截
            return Err(CatsError::business(
                ErrorCode::ComplianceBlocked,
                "fail-closed enforced (mock)",
            ));
        }
        let target_text = format!("[{}] {}", lang_name(&input.target_lang), input.source_text);
        Ok(TranslateOutput {
            target_text,
            model_used: format!("mock:{}", input.model_provider),
            prompt_tokens: input.source_text.len() as i32,
            completion_tokens: target_text.len() as i32,
            confidence: 0.85,
        })
    }
}

fn lang_name(code: &LanguageCode) -> &'static str {
    match code {
        LanguageCode::LangZhCn => "zh-CN",
        LanguageCode::LangEnUs => "en-US",
        LanguageCode::LangJaJp => "ja-JP",
        LanguageCode::LangKoKr => "ko-KR",
        _ => "und",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_translate_basic() {
        let gw = MockAiGateway;
        let out = gw
            .translate(TranslateInput {
                tenant_id: "t1".into(),
                project_id: "p1".into(),
                segment_id: "s1".into(),
                source_text: "Hello".into(),
                source_lang: LanguageCode::LangEnUs,
                target_lang: LanguageCode::LangZhCn,
                injected_terms: vec![],
                model_provider: "openai".into(),
                fail_closed: false,
            })
            .await
            .unwrap();
        assert!(out.target_text.contains("Hello"));
        assert_eq!(out.model_used, "mock:openai");
    }

    #[tokio::test]
    async fn mock_translate_fail_closed() {
        let gw = MockAiGateway;
        let res = gw
            .translate(TranslateInput {
                tenant_id: "t1".into(),
                project_id: "p1".into(),
                segment_id: "s1".into(),
                source_text: "secret".into(),
                source_lang: LanguageCode::LangEnUs,
                target_lang: LanguageCode::LangZhCn,
                injected_terms: vec![],
                model_provider: "openai".into(),
                fail_closed: true,
            })
            .await;
        assert!(res.is_err());
    }
}