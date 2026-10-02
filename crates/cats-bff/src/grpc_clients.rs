//! BFF → translation-core 的 gRPC 客户端封装
//!
//! **未接入状态（不参与编译）**
//!
//! 本文件由 `feat/mvp-final-*` 抢救进仓（commit `68fe10b`），`lib.rs` 未声明
//! `mod grpc_clients` 故不参与编译。它与 `routes.rs`、`upstream_passthrough.rs`
//! 是同一套未接入的平行设计。接入前需先做适配改造，详见
//! `upstream_passthrough.rs` 头部的说明。
//!
//! proto 定义见 `proto/cats/v1/translation_core.proto`：
//! ```proto
//! service TranslationCoreService {
//!   rpc MatchTM(MatchTMRequest) returns (MatchTMResponse);
//!   ...
//! }
//! ```
//!
//! **注意（与任务规范的偏离）**：
//! - 任务规范要求 `TmService/Lookup` 与 `TmService/Update`，但当前 `proto/cats/v1/translation_core.proto`
//!   实际定义的服务名是 `TranslationCoreService`，TM 检索 RPC 是 `MatchTM`（非 `Lookup`）。
//!   缺 `Update` / `Commit` RPC（BFF 阶段未实装，详见 `TODO.md` §1）。
//! - BFF 路径：`/api/v1/translate/lookup` → `MatchTM`（语义同 TM 检索）；
//!   `/api/v1/translate/commit` → 当前 stub 返回 501（待 proto v1.1 加 `UpdateTMMatch` RPC 后实装）。

use crate::config::Config;
use crate::error::BffError;
use cats_proto::cats::v1::{
    translation_core_service_client::TranslationCoreServiceClient,
    MatchTMRequest,
};
use serde::Serialize;
use tonic::transport::Channel;
use tracing::info;

/// translation-core gRPC 客户端（懒连接）
#[derive(Debug, Clone)]
pub struct TranslationClient {
    inner: TranslationCoreServiceClient<Channel>,
}

impl TranslationClient {
    /// 从 `TRANSLATION_CORE_GRPC` URL 建立 channel；channel 懒连接
    pub async fn connect(cfg: &Config) -> Result<Self, BffError> {
        info!(
            grpc_url = %cfg.translation_core_grpc,
            "connecting to translation-core gRPC"
        );
        let channel = Channel::from_shared(cfg.translation_core_grpc.clone())
            .map_err(|e| BffError::Upstream(format!("grpc url invalid: {e}")))?
            .connect()
            .await?;
        Ok(Self {
            inner: TranslationCoreServiceClient::new(channel),
        })
    }

    /// 从已有 channel 构造（测试用）
    pub fn from_channel(channel: Channel) -> Self {
        Self {
            inner: TranslationCoreServiceClient::new(channel),
        }
    }

    /// `MatchTM` — 调 translation-core 检索 TM
    ///
    /// 内部把 HTTP JSON 体映射成 proto `MatchTMRequest`，调 gRPC，再把响应投影成
    /// BFF 端点所需的 JSON 结构。
    pub async fn match_tm(&self, req: MatchTMRequest) -> Result<TmLookupResponse, BffError> {
        let mut client = self.inner.clone();
        let resp = client.match_tm(req).await?.into_inner();
        let matches = resp
            .matches
            .into_iter()
            .map(|m| TmMatchItemJson {
                tm_id: m.tm_id,
                source_text: m.source_text,
                target_text: m.target_text,
                similarity: m.similarity,
                is_exact: m.is_exact,
            })
            .collect();
        Ok(TmLookupResponse { matches })
    }
}

/// `/api/v1/translate/lookup` 响应 JSON 结构
#[derive(Debug, Serialize)]
pub struct TmLookupResponse {
    /// TM 匹配列表（按相似度降序）
    pub matches: Vec<TmMatchItemJson>,
}

/// 单条 TM 匹配（JSON 视图）
#[derive(Debug, Serialize)]
pub struct TmMatchItemJson {
    /// TM 条目 ID
    pub tm_id: String,
    pub source_text: String,
    pub target_text: String,
    pub similarity: f32,
    pub is_exact: bool,
}

/// `/api/v1/translate/commit` 响应占位（proto `UpdateTMMatch` RPC 待 v1.1 实装）
#[derive(Debug, Serialize)]
pub struct TmCommitAck {
    pub accepted: bool,
    pub note: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tm_commit_ack_serializes() {
        let ack = TmCommitAck {
            accepted: false,
            note: "proto UpdateTMMatch RPC pending v1.1",
        };
        let json = serde_json::to_string(&ack).unwrap();
        assert!(json.contains("\"accepted\":false"));
        assert!(json.contains("proto UpdateTMMatch RPC pending v1.1"));
    }
}