use std::path::PathBuf;

use tauri::State;

use super::{err_string, real_path};
use crate::model::archive::{ArchiveCacheDto, ArchiveSummaryDto};
use crate::state::AppState;

#[tauri::command]
pub async fn archive_summary(
    state: State<'_, AppState>,
    path: String,
) -> Result<ArchiveSummaryDto, String> {
    let mount = state.mount.clone();
    tokio::task::spawn_blocking(move || mount.summary(&path))
        .await
        .map_err(|e| format!("join error: {e}"))?
        .map(ArchiveSummaryDto::from)
        .map_err(err_string)
}

#[tauri::command]
pub async fn archive_export(
    state: State<'_, AppState>,
    path: String,
    out_path: String,
) -> Result<u64, String> {
    let target = PathBuf::from(&out_path);
    let mount = state.mount.clone();
    tokio::task::spawn_blocking(move || mount.export(&path, &target))
        .await
        .map_err(|e| format!("join error: {e}"))?
        .map_err(err_string)
}

#[tauri::command]
pub fn archive_cache_info(state: State<'_, AppState>) -> ArchiveCacheDto {
    let (used, budget, root) = state.mount.cache_report();
    ArchiveCacheDto {
        used_bytes: used as u64,
        budget_bytes: budget as u64,
        root: root.map(|path| path.to_string_lossy().into_owned()),
    }
}

#[tauri::command]
pub async fn archive_cache_clear(state: State<'_, AppState>) -> Result<ArchiveCacheDto, String> {
    state.mount.clear();
    Ok(archive_cache_info(state))
}

#[tauri::command]
pub async fn archive_materialize(
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    let real = real_path(&state, &path)?;
    Ok(real.to_string_lossy().into_owned())
}
