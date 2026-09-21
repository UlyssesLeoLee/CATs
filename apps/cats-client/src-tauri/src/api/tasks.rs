//! 任务管理调用封装 (per ULYS-154 切片 D)
//!
//! 端点 (per cats-bff §handlers.rs, 切片 A 落地):
//! - POST /v1/tasks — 派发翻译任务 (返回 task_id + status)
//!
//! MVP 客户端不实现 GET /v1/tasks/{id} 列表查询 (BFF 当前未代理 task-service
//! 的 GET 列表与 SSE, per 切片 A §honest scope). 切片 D 客户端只 dispatch +
//! 在本地 SQLite 记录, 等 BFF 升级到 M2 再接 SSE / 列表.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

use super::client::ApiClient;
use super::error::ApiError;

/// 派发任务请求 (per BFF handlers::dispatch_task + upstream/tasks.rs)
#[derive(Debug, Clone, Serialize)]
pub struct DispatchTaskRequest<'a> {
    pub project_id: &'a str,
    pub file_id: &'a str,
    pub media_type: &'a str, // TEXT | AUDIO | VIDEO | PDF | OFFICE | GAME
}

/// 派发任务响应
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DispatchTaskResponse {
    pub task_id: String,
    pub project_id: String,
    pub status: String, // queued | running | succeeded | failed
}

/// 调 POST /v1/tasks
///
/// 生成 UUID 作为 Idempotency-Key (per 接口设计书 §1.5).
pub async fn dispatch_translation_task(
    state: Arc<AppState>,
    req: DispatchTaskRequest<'_>,
) -> Result<DispatchTaskResponse, ApiError> {
    let client = ApiClient::new(state);
    let url = client.url("/v1/tasks");
    let idem_key = Uuid::new_v4().to_string();
    let resp = client
        .state
        .http
        .post(&url)
        .header("Idempotency-Key", idem_key)
        .json(&req)
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(ApiError::from_response(resp).await);
    }
    let r: DispatchTaskResponse = ApiClient::parse_json(resp).await?;
    Ok(r)
}
