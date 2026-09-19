//! 翻译 / TM lookup Tauri 命令

use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use crate::api;
use crate::api::translate::{LookupResponse, TmMatch};
use crate::state::AppState;

/// 翻译 lookup 响应（前端可见）
#[derive(Debug, Serialize)]
pub struct TranslateLookupResponse {
    pub matches: Vec<TmMatch>,
    /// 是否命中本地缓存（命中 = true 时表示离线 fallback）
    pub from_cache: bool,
}

/// `fetch_translation_lookup` — 查 TM 候选
///
/// 在线 → 调 BFF; 离线 → 读本地 tm_cache
#[tauri::command]
pub async fn fetch_translation_lookup(
    state: State<'_, Arc<AppState>>,
    source: String,
    project_id: String,
    source_lang: String,
    target_lang: String,
    threshold: Option<f32>,
) -> Result<TranslateLookupResponse, String> {
    let arc = state.inner().clone();

    // 离线模式 → 直接读本地缓存
    if !arc.is_online() {
        let guard = arc.offline_db.lock().expect("offline_db mutex poisoned");
        let Some(db) = guard.as_ref() else {
            return Err("离线数据库未初始化".into());
        };
        let cached = db
            .read_tm_cache(&project_id, 20)
            .map_err(|e| format!("read_tm_cache failed: {e}"))?;
        let matches: Vec<TmMatch> = cached
            .into_iter()
            .map(|c| TmMatch {
                tm_id: c.tm_id,
                source_text: c.source_text,
                target_text: c.target_text,
                similarity: c.similarity,
                is_exact: c.is_exact,
            })
            .collect();
        return Ok(TranslateLookupResponse {
            matches,
            from_cache: true,
        });
    }

    let q = api::translate::LookupQuery {
        source: &source,
        project_id: &project_id,
        source_lang: &source_lang,
        target_lang: &target_lang,
        threshold,
    };
    let resp: LookupResponse = api::translate::lookup_tm(arc.clone(), q)
        .await
        .map_err(|e| format!("lookup_tm failed: {e}"))?;

    // 异步写缓存（best-effort，不阻塞响应）
    {
        let arc = arc.clone();
        let matches = resp.matches.clone();
        let project_id = project_id.clone();
        tauri::async_runtime::spawn(async move {
            let guard = arc.offline_db.lock().expect("offline_db mutex poisoned");
            if let Some(db) = guard.as_ref() {
                for m in &matches {
                    if let Err(e) = db.cache_tm(
                        &m.tm_id,
                        &project_id,
                        &m.source_text,
                        &m.target_text,
                        m.similarity,
                        m.is_exact,
                    ) {
                        tracing::warn!(error = %e, "TM 缓存写入失败");
                    }
                }
            }
        });
    }

    Ok(TranslateLookupResponse {
        matches: resp.matches,
        from_cache: false,
    })
}