use std::path::PathBuf;

use tauri::State;
use tauri::ipc::Response;

use catalog::{DirListingDto, FileMetaDto, FormatDto, GameProfileDto};

use super::{decode_frame_png, err_string};
use crate::state::AppState;

#[tauri::command]
pub async fn list_dir(path: String) -> Result<DirListingDto, String> {
    let dir = PathBuf::from(&path);
    tokio::task::spawn_blocking(move || catalog::list_dir(&dir))
        .await
        .map_err(|e| format!("join error: {e}"))?
        .map_err(err_string)
}

#[tauri::command]
pub async fn file_meta(path: String) -> Result<FileMetaDto, String> {
    let target = PathBuf::from(&path);
    tokio::task::spawn_blocking(move || catalog::file_meta(&target))
        .await
        .map_err(|e| format!("join error: {e}"))?
        .map_err(err_string)
}

#[tauri::command]
pub fn list_formats() -> Vec<FormatDto> {
    catalog::FORMATS.iter().map(FormatDto::from).collect()
}

#[tauri::command]
pub async fn detect_profile(root: String) -> Result<GameProfileDto, String> {
    tokio::task::spawn_blocking(move || {
        catalog::detect_game(std::path::Path::new(&root))
    })
    .await
    .map_err(|e| format!("join error: {e}"))
}

#[tauri::command]
pub async fn file_thumb(state: State<'_, AppState>, path: String) -> Result<Response, String> {
    let format = catalog::format::lookup(&catalog::fslist::extension(std::path::Path::new(&path)))
        .ok_or_else(|| "extension is not in the format registry".to_string())?;

    if !format.has_thumb {
        return Err(format!("{} has no thumbnail", format.label));
    }

    decode_frame_png(&state, path, 0, false).await
}

#[tauri::command]
pub async fn read_file_head(path: String, max_bytes: u32) -> Result<Response, String> {
    let target = PathBuf::from(&path);
    let max = max_bytes.clamp(1, 64 * 1024) as usize;
    let bytes = tokio::task::spawn_blocking(move || catalog::read_head(&target, max))
        .await
        .map_err(|e| format!("join error: {e}"))?
        .map_err(err_string)?;
    Ok(Response::new(bytes))
}
