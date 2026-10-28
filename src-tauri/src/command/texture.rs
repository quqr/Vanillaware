use std::path::PathBuf;

use tauri::State;
use tauri::ipc::Response;

use media::TextureInfo;

use super::{decode_frame_png, err_string, frame_metas};
use crate::model::FrameListDto;
use crate::state::AppState;

#[tauri::command]
pub async fn list_frames(state: State<'_, AppState>, path: String) -> Result<FrameListDto, String> {
    let cache = state.cache.clone();
    let source = PathBuf::from(&path);

    let infos = tokio::task::spawn_blocking(move || -> Result<Vec<TextureInfo>, String> {
        let infos = formats::ftx::probe_file(&source).map_err(err_string)?;
        cache.ensure_meta(&source, &infos).map_err(err_string)?;
        Ok(infos)
    })
    .await
    .map_err(|e| format!("join error: {e}"))??;

    Ok(FrameListDto {
        path,
        count: infos.len() as u32,
        frames: frame_metas(&infos),
    })
}

#[tauri::command]
pub async fn get_thumb_png(
    state: State<'_, AppState>,
    path: String,
    index: u32,
) -> Result<Response, String> {
    decode_frame_png(&state, path, index, false).await
}

#[tauri::command]
pub async fn get_frame_png(
    state: State<'_, AppState>,
    path: String,
    index: u32,
) -> Result<Response, String> {
    decode_frame_png(&state, path, index, true).await
}

#[tauri::command]
pub async fn get_atlas_png(
    state: State<'_, AppState>,
    path: String,
    tex_id: i64,
) -> Result<Response, String> {
    let key = format!("atlas#{}#{}", path.to_lowercase(), tex_id);
    let lock = state.inflight_lock(&key);
    let _guard = lock.lock().await;

    let cache = state.cache.clone();
    let decode_sem = state.decode_sem.clone();
    let source = PathBuf::from(&path);

    let permit = decode_sem
        .clone()
        .acquire_owned()
        .await
        .map_err(|e| format!("semaphore closed: {e}"))?;

    let bytes = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let container = formats::atlas::sibling_container(&source)?
            .ok_or_else(|| formats::Error::BadData("no texture container with the same name".to_string()))?;
        let index = tex_id as usize;
        cache.ensure_png_full(&container, index, || {
            formats::ftx::decode_file_frame(&container, index)
        })
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
    .map_err(err_string)?;

    Ok(Response::new(bytes))
}
