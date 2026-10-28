pub mod animation;
pub mod audio;
pub mod cache;
pub mod content;
pub mod document;
pub mod index;
pub mod library;
pub mod texture;
pub mod video;

use std::path::PathBuf;

use tauri::State;
use tauri::ipc::Response;

use media::TextureInfo;

use crate::model::FrameMetaDto;
use crate::state::AppState;

pub(crate) fn err_string(e: impl std::fmt::Display) -> String {
    e.to_string()
}

pub(crate) fn frame_metas(infos: &[TextureInfo]) -> Vec<FrameMetaDto> {
    infos
        .iter()
        .enumerate()
        .map(|(index, info)| FrameMetaDto {
            index: index as u32,
            width: info.width,
            height: info.height,
            format: info.format,
            format_name: info.format_name(),
            channels: info.channels,
        })
        .collect()
}

pub(crate) async fn decode_frame_png(
    state: &State<'_, AppState>,
    path: String,
    index: u32,
    full: bool,
) -> Result<Response, String> {
    let key = format!(
        "{}#{}#{}",
        path.to_lowercase(),
        index,
        if full { "full" } else { "thumb" }
    );
    let lock = state.inflight_lock(&key);
    let _guard = lock.lock().await;

    let cache = state.cache.clone();
    let decode_sem = state.decode_sem.clone();
    let source = PathBuf::from(&path);
    let index = index as usize;

    let permit = decode_sem
        .clone()
        .acquire_owned()
        .await
        .map_err(|e| format!("semaphore closed: {e}"))?;

    let bytes = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let decode = || formats::ftx::decode_file_frame(&source, index);
        if full {
            cache.ensure_png_full(&source, index, decode)
        } else {
            cache.ensure_png(&source, index, decode)
        }
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
    .map_err(err_string)?;

    Ok(Response::new(bytes))
}
