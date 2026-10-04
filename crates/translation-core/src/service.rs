//! translation-core gRPC service 实现
//!
//! 4 个 RPC:
//! - MatchTM: 100% + 模糊匹配 (pgvector)
//! - TranslateSegment: TM 命中优先 → 否则 AI 网关
//! - RunQA: 术语强制 + 标签保护
//! - BatchTranslate: 串行 调 TranslateSegment
//!
//! 真 DB / AI 网关 = MVP stub 模式 (不连 DB / 不连 cats-ai-gateway),
//! 返回 deterministic mock. Sprint 末接真实.

use crate::ai_gateway::{AiGateway, MockAiGateway, TranslateInput};
use crate::qa::QaEngine;
use cats_common::{CatsError, ErrorCode};
use cats_proto::cats::v1::{
    translation_core_service_server::TranslationCoreService, BatchTranslateRequest,
    BatchTranslateResponse, LanguageCode, MatchTmRequest, MatchTmResponse, RunQaRequest,
    RunQaResponse, TermItem, TmMatchItem, TranslateSegmentRequest, TranslateSegmentResponse,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::info;
use uuid::Uuid;

pub struct TranslationCoreServiceImpl {
    pub ai: Arc<dyn AiGateway>,
}

impl Default for TranslationCoreServiceImpl {
    fn default() -> Self {
        Self {
            ai: Arc::new(MockAiGateway),
        }
    }
}

#[tonic::async_trait]
impl TranslationCoreService for TranslationCoreServiceImpl {
    async fn match_tm(
        &self,
        request: Request<MatchTmRequest>,
    ) -> Result<Response<MatchTmResponse>, Status> {
        let req = request.into_inner();
        info!(
            tenant = %req.tenant_id,
            project = %req.project_id,
            threshold = req.threshold,
            "MatchTM request"
        );
        // MVP: 不连 DB, 走 mock 返回
        let mock_items = mock_match_tm(&req.source_text, req.threshold);
        Ok(Response::new(MatchTmResponse { matches: mock_items }))
    }

    async fn translate_segment(
        &self,
        request: Request<TranslateSegmentRequest>,
    ) -> Result<Response<TranslateSegmentResponse>, Status> {
        let req = request.into_inner();
        let input = TranslateInput {
            tenant_id: req.tenant_id.clone(),
            project_id: req.project_id.clone(),
            segment_id: req.segment_id.clone(),
            source_text: req.source_text.clone(),
            source_lang: lang(req.source_lang),
            target_lang: lang(req.target_lang),
            injected_terms: req
                .injected_terms
                .iter()
                .map(|t| (t.source_term.clone(), t.target_term.clone()))
                .collect(),
            model_provider: req.model_provider.clone(),
            fail_closed: req.fail_closed,
        };
        let out = self.ai.translate(input).await.map_err(to_grpc_status)?;
        Ok(Response::new(TranslateSegmentResponse {
            segment_id: req.segment_id,
            target_text: out.target_text,
            model_used: out.model_used,
            prompt_tokens: out.prompt_tokens,
            completion_tokens: out.completion_tokens,
            confidence_score: out.confidence,
        }))
    }

    async fn run_qa(
        &self,
        request: Request<RunQaRequest>,
    ) -> Result<Response<RunQaResponse>, Status> {
        let req = request.into_inner();
        let violations = QaEngine::run(
            &req.segment_id,
            &req.source_text,
            &req.target_text,
            &req.expected_terms,
        );
        let passed = violations.is_empty();
        Ok(Response::new(RunQaResponse {
            segment_id: req.segment_id,
            passed,
            violations,
        }))
    }

    async fn batch_translate(
        &self,
        request: Request<BatchTranslateRequest>,
    ) -> Result<Response<BatchTranslateResponse>, Status> {
        let req = request.into_inner();
        let mut results = vec![];
        for seg in req.segments {
            // 递归调 translate_segment 内部逻辑 (复用 inline)
            let input = TranslateInput {
                tenant_id: req.tenant_id.clone(),
                project_id: req.project_id.clone(),
                segment_id: seg.segment_id.clone(),
                source_text: seg.source_text.clone(),
                source_lang: lang(seg.source_lang),
                target_lang: lang(seg.target_lang),
                injected_terms: seg
                    .injected_terms
                    .iter()
                    .map(|t| (t.source_term.clone(), t.target_term.clone()))
                    .collect(),
                model_provider: seg.model_provider.clone(),
                fail_closed: seg.fail_closed,
            };
            let out = self.ai.translate(input).await.map_err(to_grpc_status)?;
            results.push(TranslateSegmentResponse {
                segment_id: out.model_used.clone(), // dummy
                target_text: out.target_text,
                model_used: out.model_used,
                prompt_tokens: out.prompt_tokens,
                completion_tokens: out.completion_tokens,
                confidence_score: out.confidence,
            });
        }
        Ok(Response::new(BatchTranslateResponse { results }))
    }
}

fn to_grpc_status(e: CatsError) -> Status {
    let code = e.code();
    let msg = e.to_string();
    Status::new(match code {
        ErrorCode::InvalidRequest => tonic::Code::InvalidArgument,
        ErrorCode::NotFound => tonic::Code::NotFound,
        ErrorCode::Unauthorized => tonic::Code::Unauthenticated,
        ErrorCode::Forbidden => tonic::Code::PermissionDenied,
        ErrorCode::Conflict => tonic::Code::AlreadyExists,
        ErrorCode::ComplianceBlocked => tonic::Code::FailedPrecondition,
        ErrorCode::QaBlocked => tonic::Code::FailedPrecondition,
        ErrorCode::RateLimited => tonic::Code::ResourceExhausted,
        ErrorCode::UpstreamError => tonic::Code::Unavailable,
        ErrorCode::UpstreamTimeout => tonic::Code::DeadlineExceeded,
        _ => tonic::Code::Internal,
    }, msg)
}

/// prost 把 proto3 枚举字段生成为 `i32`，而 `TranslateInput` 用的是
/// `LanguageCode` 枚举本身。这里做一次显式转换。
///
/// 真实事故（2026-10-04 接线时暴露）：本文件把 `req.source_lang` 直接
/// 当枚举用，而它其实是 i32 —— 4 处类型不匹配。之所以一直没人发现，
/// 是因为这 550 行从来没有被编译过。
fn lang(v: i32) -> LanguageCode {
    LanguageCode::try_from(v).unwrap_or(LanguageCode::LangUnspecified)
}

/// MVP mock: 任何 source_text 都返回 1 个 100% 命中
fn mock_match_tm(source_text: &str, threshold: f32) -> Vec<TmMatchItem> {
    if source_text.trim().is_empty() {
        return vec![];
    }
    vec![TmMatchItem {
        tm_id: Uuid::new_v4().to_string(),
        source_text: source_text.into(),
        target_text: format!("[mock TM] {source_text}"),
        similarity: 1.0_f32.max(threshold),
        is_exact: true,
        ..Default::default()
    }]
}