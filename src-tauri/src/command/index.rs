//! 索引命令：启动构建、查询状态、全局搜索。

use std::path::PathBuf;

use tauri::State;

use catalog::{IndexStatusDto, SearchResultDto};

use crate::state::AppState;

/// 换根重建索引。立即返回 `building` 状态，进度由 [`index_status`] 轮询。
#[tauri::command]
pub fn start_index(state: State<'_, AppState>, root: String) -> IndexStatusDto {
    state.start_index(&PathBuf::from(&root));
    state.index_status()
}

#[tauri::command]
pub fn index_status(state: State<'_, AppState>) -> IndexStatusDto {
    state.index_status()
}

/// 全局搜索。索引未建完时返回已收集的部分，状态由前端另行读取。
#[tauri::command]
pub async fn search_files(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<SearchResultDto, String> {
    let entries = state.index_entries();
    let limit = limit.map(|v| v as usize).unwrap_or(catalog::SEARCH_LIMIT);
    tokio::task::spawn_blocking(move || catalog::search_index(&entries, &query, limit))
        .await
        .map_err(|e| format!("join error: {e}"))
}
