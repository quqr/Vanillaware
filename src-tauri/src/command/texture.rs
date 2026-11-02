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
    let mount = state.mount.clone();
    let key = path.clone();

    let infos = tokio::task::spawn_blocking(move || -> Result<Vec<TextureInfo>, String> {
        let stat = mount.stat(&key).map_err(err_string)?;
        let stamp = media::cache::Stamp::new(stat.size, stat.mtime_ms);
        let bytes = mount.read(&key).map_err(err_string)?;
        let infos = formats::ftx::probe_bytes(&bytes).map_err(err_string)?;
        cache
            .ensure_meta(&PathBuf::from(&key), stamp, &infos)
            .map_err(err_string)?;
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
    let mount = state.mount.clone();
    let decode_sem = state.decode_sem.clone();

    let permit = decode_sem
        .clone()
        .acquire_owned()
        .await
        .map_err(|e| format!("semaphore closed: {e}"))?;

    let bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        let _permit = permit;
        let container = mount
            .sibling_container(&path)
            .ok_or_else(|| "no texture container with the same name".to_string())?;
        let stat = mount.stat(&container).map_err(err_string)?;
        let stamp = media::cache::Stamp::new(stat.size, stat.mtime_ms);
        let raw = mount.read(&container).map_err(err_string)?;
        let index = tex_id as usize;
        cache
            .ensure_png_full(&PathBuf::from(&container), stamp, index, || {
                formats::ftx::decode_bytes_frame(&raw, index)
            })
            .map_err(err_string)
    })
    .await
    .map_err(|e| format!("join error: {e}"))??;

    Ok(Response::new(bytes))
}
