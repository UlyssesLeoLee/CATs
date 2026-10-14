//! 任务管理 + 本地术语库 Tauri 命令 (per ULYS-154 切片 D)
//!
//! 暴露给 Svelte 前端:
//! - `dispatch_translation_task`  — POST /v1/tasks + 写本地 task 记录
//! - `list_local_tasks`           — 从本地 SQLite 列出已 dispatch 的任务
//! - `update_local_task_status`   — 改本地任务状态 (手动模拟进度, 等 SSE 接入)
//! - `list_task_events`           — 列出一个任务的所有本地事件
//! - `add_local_glossary_entry`   — 新增一条术语 (本地)
//! - `list_local_glossary_entries`— 列出本地术语

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::api;
use crate::offline::{LocalGlossaryEntry, LocalTask, TaskEvent};
use crate::state::AppState;

// ---- dispatch_translation_task ----

#[derive(Debug, Deserialize)]
pub struct DispatchArgs {
    pub project_id: String,
    pub file_id: String,
    pub media_type: String, // TEXT (MVP 仅此)
    pub source_text: String, // 本地缓存用, 不进 BFF payload
}

#[derive(Debug, Serialize)]
pub struct DispatchResult {
    pub task_id: String,
    pub project_id: String,
    pub status: String,
}

#[tauri::command]
pub async fn dispatch_translation_task(
    state: State<'_, Arc<AppState>>,
    args: DispatchArgs,
) -> Result<DispatchResult, String> {
    let arc = state.inner().clone();
    let req = api::tasks::DispatchTaskRequest {
        project_id: &args.project_id,
        file_id: &args.file_id,
        media_type: &args.media_type,
    };
    let resp = api::tasks::dispatch_translation_task(arc.clone(), req)
        .await
        .map_err(|e| format!("dispatch_translation_task failed: {e}"))?;

    // 写本地任务 + 记 dispatched 事件
    {
        let guard = arc.offline_db.lock().expect("offline_db mutex poisoned");
        if let Some(db) = guard.as_ref() {
            let _ = db.upsert_local_task(
                &resp.task_id,
                &resp.project_id,
                &resp.status,
                &args.source_text,
                None,
                Some(&args.media_type),
            );
            let payload = serde_json::json!({
                "project_id": resp.project_id,
                "file_id": args.file_id,
                "media_type": args.media_type,
            })
            .to_string();
            let _ = db.record_task_event(&resp.task_id, "dispatched", Some(&payload));
        }
    }

    Ok(DispatchResult {
        task_id: resp.task_id,
        project_id: resp.project_id,
        status: resp.status,
    })
}

// ---- list_local_tasks ----

#[derive(Debug, Deserialize)]
pub struct ListTasksArgs {
    pub project_id: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ListTasksResponse {
    pub tasks: Vec<LocalTask>,
}

#[tauri::command]
pub async fn list_local_tasks(
    state: State<'_, Arc<AppState>>,
    args: ListTasksArgs,
) -> Result<ListTasksResponse, String> {
    let arc = state.inner().clone();
    let limit = args.limit.unwrap_or(50);
    let guard = arc.offline_db.lock().expect("offline_db mutex poisoned");
    let Some(db) = guard.as_ref() else {
        return Ok(ListTasksResponse { tasks: vec![] });
    };
    let tasks = db
        .list_local_tasks(args.project_id.as_deref(), limit)
        .map_err(|e| e.to_string())?;
    Ok(ListTasksResponse { tasks })
}

// ---- update_local_task_status ----

#[derive(Debug, Deserialize)]
pub struct UpdateTaskStatusArgs {
    pub task_id: String,
    pub new_status: String,
}

#[tauri::command]
pub async fn update_local_task_status(
    state: State<'_, Arc<AppState>>,
    args: UpdateTaskStatusArgs,
) -> Result<(), String> {
    let arc = state.inner().clone();
    let guard = arc.offline_db.lock().expect("offline_db mutex poisoned");
    let Some(db) = guard.as_ref() else {
        return Err("离线数据库未初始化".into());
    };
    db.update_local_task_status(&args.task_id, &args.new_status)
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ---- list_task_events ----

#[derive(Debug, Deserialize)]
pub struct ListEventsArgs {
    pub task_id: String,
}

#[derive(Debug, Serialize)]
pub struct ListEventsResponse {
    pub events: Vec<TaskEvent>,
}

#[tauri::command]
pub async fn list_task_events(
    state: State<'_, Arc<AppState>>,
    args: ListEventsArgs,
) -> Result<ListEventsResponse, String> {
    let arc = state.inner().clone();
    let guard = arc.offline_db.lock().expect("offline_db mutex poisoned");
    let Some(db) = guard.as_ref() else {
        return Ok(ListEventsResponse { events: vec![] });
    };
    let events = db
        .list_task_events(&args.task_id)
        .map_err(|e| e.to_string())?;
    Ok(ListEventsResponse { events })
}

// ---- add_local_glossary_entry ----

#[derive(Debug, Deserialize)]
pub struct AddGlossaryArgs {
    pub source_term: String,
    pub target_term: String,
    pub domain: Option<String>,
    pub notes: Option<String>,
    pub project_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GlossaryEntryView {
    pub entry: LocalGlossaryEntry,
}

#[tauri::command]
pub async fn add_local_glossary_entry(
    state: State<'_, Arc<AppState>>,
    args: AddGlossaryArgs,
) -> Result<GlossaryEntryView, String> {
    if args.source_term.trim().is_empty() || args.target_term.trim().is_empty() {
        return Err("源术语和目标术语不能为空".into());
    }
    let arc = state.inner().clone();
    let entry_id = uuid::Uuid::new_v4().to_string();
    let guard = arc.offline_db.lock().expect("offline_db mutex poisoned");
    let Some(db) = guard.as_ref() else {
        return Err("离线数据库未初始化".into());
    };
    db.add_glossary(
        &entry_id,
        &args.source_term,
        &args.target_term,
        args.domain.as_deref(),
        args.notes.as_deref(),
        args.project_id.as_deref(),
    )
    .map_err(|e| e.to_string())?;
    let entries = db
        .list_glossary(args.project_id.as_deref(), 1)
        .map_err(|e| e.to_string())?;
    let entry = entries
        .into_iter()
        .next()
        .ok_or_else(|| "写入成功但读取失败".to_string())?;
    Ok(GlossaryEntryView { entry })
}

// ---- list_local_glossary_entries ----

#[derive(Debug, Deserialize)]
pub struct ListGlossaryArgs {
    pub project_id: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ListGlossaryResponse {
    pub entries: Vec<LocalGlossaryEntry>,
}

#[tauri::command]
pub async fn list_local_glossary_entries(
    state: State<'_, Arc<AppState>>,
    args: ListGlossaryArgs,
) -> Result<ListGlossaryResponse, String> {
    let arc = state.inner().clone();
    let limit = args.limit.unwrap_or(100);
    let guard = arc.offline_db.lock().expect("offline_db mutex poisoned");
    let Some(db) = guard.as_ref() else {
        return Ok(ListGlossaryResponse { entries: vec![] });
    };
    let entries = db
        .list_glossary(args.project_id.as_deref(), limit)
        .map_err(|e| e.to_string())?;
    Ok(ListGlossaryResponse { entries })
}
