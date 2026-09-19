//! 离线模式 Tauri 命令

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::api;
use crate::offline::{OfflineAction, OfflineStatus};
use crate::state::AppState;

/// 离线队列动作（前端传入）
#[derive(Debug, Deserialize)]
pub struct EnqueueRequest {
    pub action_type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct EnqueueResponse {
    pub seq: i64,
}

/// 同步结果
#[derive(Debug, Serialize)]
pub struct SyncResult {
    pub total: i64,
    pub succeeded: i64,
    pub failed: i64,
}

/// `get_offline_status` — 当前离线状态 + 队列长度
#[tauri::command]
pub async fn get_offline_status(
    state: State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
) -> Result<OfflineStatus, String> {
    let arc = state.inner().clone();
    let online = arc.is_online();

    let db_path = app
        .path()
        .app_local_data_dir()
        .map(|p| p.join("offline.db").to_string_lossy().to_string())
        .unwrap_or_else(|_| "<unknown>".to_string());

    let guard = arc.offline_db.lock().expect("offline_db mutex poisoned");
    let Some(db) = guard.as_ref() else {
        return Ok(OfflineStatus {
            online,
            queue_length: 0,
            last_sync_at: None,
            db_path,
        });
    };
    db.status(online, &db_path).map_err(|e| e.to_string())
}

/// `enqueue_offline_action` — 写一条动作到本地队列
#[tauri::command]
pub async fn enqueue_offline_action(
    state: State<'_, Arc<AppState>>,
    request: EnqueueRequest,
) -> Result<EnqueueResponse, String> {
    let arc = state.inner().clone();
    let payload_json = serde_json::to_string(&request.payload).map_err(|e| e.to_string())?;
    let guard = arc.offline_db.lock().expect("offline_db mutex poisoned");
    let Some(db) = guard.as_ref() else {
        return Err("离线数据库未初始化".into());
    };
    let seq = db
        .enqueue(&OfflineAction {
            action_type: request.action_type,
            payload_json,
        })
        .map_err(|e| e.to_string())?;
    Ok(EnqueueResponse { seq })
}

/// `sync_offline_queue` — 批量同步队列到 BFF
///
/// MVP 简化:
/// - 仅处理 `create_project` 动作
/// - 其他 action_type 暂不实现（per apps/cats-client/TODO.md）
#[tauri::command]
pub async fn sync_offline_queue(
    state: State<'_, Arc<AppState>>,
) -> Result<SyncResult, String> {
    let arc = state.inner().clone();

    let pending = {
        let guard = arc.offline_db.lock().expect("offline_db mutex poisoned");
        let Some(db) = guard.as_ref() else {
            return Err("离线数据库未初始化".into());
        };
        db.fetch_pending(100).map_err(|e| e.to_string())?
    };

    let mut succeeded = 0i64;
    let mut failed = 0i64;

    for action in pending {
        let result: anyhow::Result<()> = match action.action_type.as_str() {
            "create_project" => {
                let v: serde_json::Value = serde_json::from_str(&action.payload_json)?;
                let req = api::projects::CreateProjectRequest {
                    name: v.get("name").and_then(|x| x.as_str()).unwrap_or(""),
                    source_lang: v.get("source_lang").and_then(|x| x.as_str()).unwrap_or(""),
                    target_lang: v.get("target_lang").and_then(|x| x.as_str()).unwrap_or(""),
                };
                api::projects::create_project(arc.clone(), req)
                    .await
                    .map(|_| ())
                    .map_err(Into::into)
            }
            _ => {
                tracing::warn!(action_type = %action.action_type, "未实现的离线 action 类型，跳过");
                Ok(())
            }
        };

        let guard = arc.offline_db.lock().expect("offline_db mutex poisoned");
        let db = guard.as_ref().expect("offline_db missing");
        match result {
            Ok(()) => {
                db.mark_done(action.seq).map_err(|e| e.to_string())?;
                succeeded += 1;
            }
            Err(e) => {
                db.mark_failed(action.seq, &e.to_string())
                    .map_err(|e| e.to_string())?;
                failed += 1;
            }
        }
    }

    Ok(SyncResult {
        total: succeeded + failed,
        succeeded,
        failed,
    })
}