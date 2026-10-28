use std::path::PathBuf;

use tauri::State;

use catalog::{IndexStatusDto, SearchResultDto};

use crate::state::AppState;

#[tauri::command]
pub fn start_index(state: State<'_, AppState>, root: String) -> IndexStatusDto {
    state.start_index(&PathBuf::from(&root));
    state.index_status()
}

#[tauri::command]
pub fn index_status(state: State<'_, AppState>) -> IndexStatusDto {
    state.index_status()
}

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
