//! 项目管理 Tauri 命令

use std::sync::Arc;

use tauri::State;

use crate::api;
use crate::state::AppState;

/// `list_projects` — 调 GET /v1/projects
#[tauri::command]
pub async fn list_projects(state: State<'_, Arc<AppState>>) -> Result<api::projects::ListProjectsResponse, String> {
    let arc = state.inner().clone();
    api::projects::list_projects(arc)
        .await
        .map_err(|e| format!("list_projects failed: {e}"))
}

/// `create_project` — 调 POST /v1/projects
#[tauri::command]
pub async fn create_project(
    state: State<'_, Arc<AppState>>,
    name: String,
    source_lang: String,
    target_lang: String,
) -> Result<api::projects::Project, String> {
    let arc = state.inner().clone();
    let req = api::projects::CreateProjectRequest {
        name: &name,
        source_lang: &source_lang,
        target_lang: &target_lang,
    };
    api::projects::create_project(arc, req)
        .await
        .map_err(|e| format!("create_project failed: {e}"))
}