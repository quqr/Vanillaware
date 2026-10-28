//! 缓存命令：统计与清空。

use tauri::State;

use super::err_string;
use crate::state::AppState;

#[tauri::command]
pub async fn get_cache_info(state: State<'_, AppState>) -> Result<media::CacheInfo, String> {
    let cache = state.cache.clone();
    tokio::task::spawn_blocking(move || cache.info())
        .await
        .map_err(|e| format!("join error: {e}"))
}

#[tauri::command]
pub async fn clear_thumb_cache(state: State<'_, AppState>) -> Result<media::CacheInfo, String> {
    let cache = state.cache.clone();
    tokio::task::spawn_blocking(move || cache.clear())
        .await
        .map_err(|e| format!("join error: {e}"))?
        .map_err(err_string)
}
