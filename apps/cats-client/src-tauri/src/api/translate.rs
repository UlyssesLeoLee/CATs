//! 翻译 / TM lookup 调用封装
//!
//! 端点:
//! - GET /v1/translate/lookup?source=...&project_id=...&source_lang=...&target_lang=...
//!
//! 内部经 BFF gRPC 调 translation-core.MatchTM (per proto/cats/v1/translation_core.proto)
//!
//! MVP 客户端只读 lookup，不实现 TM 写入（per apps/cats-client/TODO.md）。

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::state::AppState;

use super::client::ApiClient;
use super::error::ApiError;

/// TM 候选条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmMatch {
    pub tm_id: String,
    pub source_text: String,
    pub target_text: String,
    pub similarity: f32,
    pub is_exact: bool,
}

/// 查询参数（query string）
#[derive(Debug, Serialize)]
pub struct LookupQuery<'a> {
    pub source: &'a str,
    pub project_id: &'a str,
    pub source_lang: &'a str,
    pub target_lang: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f32>,
}

/// 响应（matches 数组）
#[derive(Debug, Deserialize)]
pub struct LookupResponse {
    pub matches: Vec<TmMatch>,
}

/// 调 /v1/translate/lookup
pub async fn lookup_tm(
    state: Arc<AppState>,
    q: LookupQuery<'_>,
) -> Result<LookupResponse, ApiError> {
    let client = ApiClient::new(state);
    let url = format!(
        "/v1/translate/lookup?source={}&project_id={}&source_lang={}&target_lang={}{}",
        urlencoding(&q.source),
        urlencoding(q.project_id),
        urlencoding(q.source_lang),
        urlencoding(q.target_lang),
        q.threshold
            .map(|t| format!("&threshold={t}"))
            .unwrap_or_default(),
    );
    client.get_authed(&url).await
}

/// 最小 URL 编码（MVP 阶段只对 query string 做百分号编码，
/// 不引入 urlencoding crate）。
fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char);
            }
            other => {
                out.push_str(&format!("%{other:02X}"));
            }
        }
    }
    out
}