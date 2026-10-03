//! API 错误信封（per 接口设计书 §1.3）
//!
//! Schema (v2.0+2 patch):
//! ```json
//! {
//!   "error": {
//!     "code": "VALIDATION_ERROR",
//!     "message": "字段 target_lang 不是合法的 BCP-47 语言代码",
//!     "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
//!     "details": { "field": "target_lang" }
//!   }
//! }
//! ```

use serde::{Deserialize, Serialize};

/// 顶层错误信封
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiErrorEnvelope {
    pub error: ApiErrorBody,
}

/// 错误体内层
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub trace_id: Option<String>,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
}

/// 业务层 API 错误（统一错误类型，所有命令返回 `Result<T, ApiError>`）
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("网络/传输错误: {0}")]
    Transport(#[from] reqwest::Error),

    #[error("BFF 返回错误: code={code} message={message} trace_id={trace_id:?}")]
    Server {
        code: String,
        message: String,
        trace_id: Option<String>,
        /// 原始 HTTP 状态码（保留以便上层做重试 / UI 分支）
        status: u16,
    },

    #[error("响应解析失败: {0}")]
    Decode(String),

    #[error("未认证（token 缺失或 refresh 失败）")]
    Unauthenticated,

    #[error("客户端内部错误: {0}")]
    Internal(#[from] anyhow::Error),
}

impl ApiError {
    /// 从 reqwest::Response 解析错误信封
    pub async fn from_response(resp: reqwest::Response) -> Self {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();

        // 尝试解析信封
        if let Ok(env) = serde_json::from_str::<ApiErrorEnvelope>(&text) {
            let b = env.error;
            return Self::Server {
                code: b.code,
                message: b.message,
                trace_id: b.trace_id,
                status,
            };
        }

        // 解析失败 → 退化到 Internal(原始 body)
        Self::Decode(format!("status={status} body={text}"))
    }
}