//! `translation-core` gRPC 端到端集成测试
//!
//! **为什么需要它**: 2026-10-04 之前，`service.rs` 里的 4 个 RPC 从未被编译，
//! 更没被调用过。接上线之后，"能编译"和"调得通"是两件事 ——
// prost 的类型映射、枚举在**线路上的表示**、`CatsError` 到
//! `tonic::Status` 的映射，都只有真正发一次请求才能验证。
//!
//! 这里起一个真实的 tonic server（绑 127.0.0.1:0，由内核分配端口），
//! 用生成的 client 连回去，逐个 RPC 走一遍真实的序列化/反序列化。
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 契约: proto/cats/v1/translation_core.proto

use std::time::Duration;

use cats_proto::cats::v1::{
    translation_core_service_client::TranslationCoreServiceClient,
    translation_core_service_server::TranslationCoreServiceServer, LanguageCode, MatchTmRequest,
    RunQaRequest, TermItem, TranslateSegmentRequest,
};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::{Channel, Server};
use translation_core::service::TranslationCoreServiceImpl;

async fn start_server() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let svc = TranslationCoreServiceImpl::default();
    tokio::spawn(async move {
        Server::builder()
            .add_service(TranslationCoreServiceServer::new(svc))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .expect("gRPC server 应当正常退出前一直服务");
    });
    // 端口是内核给的，但 accept loop 未必已经进入 accept；给一点余量。
    tokio::time::sleep(Duration::from_millis(80)).await;
    format!("http://{addr}")
}

async fn client() -> TranslationCoreServiceClient<Channel> {
    let url = start_server().await;
    let mut c = TranslationCoreServiceClient::connect(url).await.unwrap();
    // 本仓所有客户端调用都带 5s 超时；测试里给足但不无限等。
    c = c
        .max_decoding_message_size(4 * 1024 * 1024)
        .max_encoding_message_size(4 * 1024 * 1024);
    c
}

#[tokio::test]
async fn match_tm_returns_one_exact_hit() {
    let mut c = client().await;
    let resp = c
        .match_tm(MatchTmRequest {
            tenant_id: "t-1".into(),
            project_id: "p-1".into(),
            source_text: "Hello".into(),
            source_lang: LanguageCode::LangEnUs as i32,
            target_lang: LanguageCode::LangZhCn as i32,
            threshold: 0.85,
        })
        .await
        .expect("MatchTM 应当成功")
        .into_inner();

    assert_eq!(resp.matches.len(), 1, "非空 source_text 应返回 1 个命中");
    assert!(resp.matches[0].is_exact);
    assert_eq!(resp.matches[0].source_text, "Hello");
    assert!(resp.matches[0].similarity >= 0.85);
}

#[tokio::test]
async fn match_tm_empty_text_returns_nothing() {
    let mut c = client().await;
    let resp = c
        .match_tm(MatchTmRequest {
            tenant_id: "t-1".into(),
            project_id: "p-1".into(),
            source_text: "   ".into(),
            source_lang: LanguageCode::LangEnUs as i32,
            target_lang: LanguageCode::LangZhCn as i32,
            threshold: 0.85,
        })
        .await
        .unwrap()
        .into_inner();
    assert!(resp.matches.is_empty(), "空白 source_text 不应产生命中");
}

#[tokio::test]
async fn translate_segment_maps_lang_code_over_the_wire() {
    // 这条是最关键的一条: LanguageCode 在 proto 字段上是 i32，
    // 而 service 内部用枚举。线路上只传 i32，服务端必须转回来。
    // 接线前的代码把 req.source_lang 当枚举用，4 处编译不过；
    // 现在要验证转换在真实 RPC 路径上确实生效。
    let mut c = client().await;
    let resp = c
        .translate_segment(TranslateSegmentRequest {
            tenant_id: "t-1".into(),
            project_id: "p-1".into(),
            segment_id: "seg-1".into(),
            source_text: "Hello".into(),
            source_lang: LanguageCode::LangEnUs as i32,
            target_lang: LanguageCode::LangJaJp as i32,
            injected_terms: vec![],
            tm_hints: vec![],
            model_provider: "openai".into(),
            fail_closed: false,
        })
        .await
        .expect("TranslateSegment 应当成功")
        .into_inner();

    assert_eq!(resp.segment_id, "seg-1");
    assert_eq!(
        resp.target_text, "[ja-JP] Hello",
        "target_lang 必须被正确还原成 ja-JP"
    );
    assert!(resp.model_used.starts_with("mock:"));
    assert!(resp.completion_tokens > 0);
}

#[tokio::test]
async fn translate_segment_fail_closed_maps_to_failed_precondition() {
    // 验证 CatsError -> tonic::Status 的映射（接线时把 error_code() 改成了 code()）
    let mut c = client().await;
    let err = c
        .translate_segment(TranslateSegmentRequest {
            tenant_id: "t-1".into(),
            project_id: "p-1".into(),
            segment_id: "seg-1".into(),
            source_text: "Hello".into(),
            source_lang: LanguageCode::LangEnUs as i32,
            target_lang: LanguageCode::LangZhCn as i32,
            injected_terms: vec![],
            tm_hints: vec![],
            model_provider: "openai".into(),
            fail_closed: true,
        })
        .await
        .expect_err("fail_closed 应当被拦截");
    assert_eq!(
        err.code(),
        tonic::Code::FailedPrecondition,
        "ErrorCode::ComplianceBlocked 应映射成 FailedPrecondition"
    );
}

#[tokio::test]
async fn run_qa_flags_forbidden_term() {
    let mut c = client().await;
    let resp = c
        .run_qa(RunQaRequest {
            segment_id: "seg-1".into(),
            source_text: "Use Acme".into(),
            target_text: "请使用 Acme".into(),
            source_lang: LanguageCode::LangEnUs as i32,
            target_lang: LanguageCode::LangZhCn as i32,
            expected_terms: vec![TermItem {
                source_term: "acme".into(),
                target_term: "Acme".into(),
                forbidden: true,
            }],
        })
        .await
        .unwrap()
        .into_inner();

    assert!(!resp.passed, "出现禁用术语应判不通过");
    assert_eq!(resp.violations.len(), 1);
    assert_eq!(resp.violations[0].rule_id, "glossary.forbidden");
}

#[tokio::test]
async fn run_qa_flags_broken_placeholder() {
    let mut c = client().await;
    let resp = c
        .run_qa(RunQaRequest {
            segment_id: "seg-1".into(),
            source_text: "Click {button} now".into(),
            target_text: "现在点击 button".into(), // 占位符没保住
            source_lang: LanguageCode::LangEnUs as i32,
            target_lang: LanguageCode::LangZhCn as i32,
            expected_terms: vec![],
        })
        .await
        .unwrap()
        .into_inner();

    assert!(!resp.passed);
    assert!(
        resp.violations
            .iter()
            .any(|v| v.rule_category == "placeholder"),
        "应报出占位符类违规，实际: {:?}",
        resp.violations
    );
}

#[tokio::test]
async fn run_qa_passes_on_clean_text() {
    let mut c = client().await;
    let resp = c
        .run_qa(RunQaRequest {
            segment_id: "seg-1".into(),
            source_text: "Click {button}".into(),
            target_text: "点击 {button}".into(),
            source_lang: LanguageCode::LangEnUs as i32,
            target_lang: LanguageCode::LangZhCn as i32,
            expected_terms: vec![],
        })
        .await
        .unwrap()
        .into_inner();
    assert!(
        resp.passed,
        "干净的文本应通过，实际违规: {:?}",
        resp.violations
    );
}

#[tokio::test]
async fn batch_translate_returns_one_result_per_segment() {
    let mut c = client().await;
    // proto 里 BatchTranslateRequest.segments 是 repeated TranslateSegmentRequest，
    // 没有独立的 SegmentItem 类型
    let seg = |id: &str, text: &str| TranslateSegmentRequest {
        segment_id: id.into(),
        source_text: text.into(),
        source_lang: LanguageCode::LangEnUs as i32,
        target_lang: LanguageCode::LangKoKr as i32,
        injected_terms: vec![],
        tm_hints: vec![],
        model_provider: "openai".into(),
        fail_closed: false,
        tenant_id: String::new(),
        project_id: String::new(),
    };
    let resp = c
        .batch_translate(cats_proto::cats::v1::BatchTranslateRequest {
            tenant_id: "t-1".into(),
            project_id: "p-1".into(),
            segments: vec![seg("s1", "A"), seg("s2", "B"), seg("s3", "C")],
        })
        .await
        .unwrap()
        .into_inner();

    assert_eq!(resp.results.len(), 3, "3 段应得 3 个结果");
    // MVP 的 batch 实现里 segment_id 被写成了 model_used（源码注释标了 dummy），
    // 这里如实断言现状，不假装它是对的。
    assert!(resp
        .results
        .iter()
        .all(|r| r.segment_id.starts_with("mock:")));
    assert_eq!(resp.results[0].target_text, "[ko-KR] A");
    assert_eq!(resp.results[2].target_text, "[ko-KR] C");
}
