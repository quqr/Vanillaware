pub mod animation;
pub mod archive;
pub mod audio;
pub mod cache;
pub mod content;
pub mod dialogue;
pub mod document;
pub mod field;
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

pub(crate) fn read_bytes(state: &AppState, path: &str) -> Result<Vec<u8>, String> {
    state.mount.read(path).map_err(err_string)
}

pub(crate) fn read_source(
    state: &AppState,
    path: &str,
) -> Result<(Vec<u8>, u64, u64, Option<PathBuf>), String> {
    let stat = state.mount.stat(path).map_err(err_string)?;
    let bytes = state.mount.read(path).map_err(err_string)?;
    let text = path.trim_end_matches(['\\', '/']);
    let directory = match text.rfind(['\\', '/']) {
        Some(at) if at > 0 => state.mount.real(&text[..at]).ok(),
        _ => None,
    };
    Ok((bytes, stat.size, stat.mtime_ms, directory))
}

pub(crate) fn real_path(state: &AppState, path: &str) -> Result<PathBuf, String> {
    state.mount.real(path).map_err(err_string)
}

pub(crate) fn stamp(state: &AppState, path: &str) -> Result<media::cache::Stamp, String> {
    let stat = state.mount.stat(path).map_err(err_string)?;
    Ok(media::cache::Stamp::new(stat.size, stat.mtime_ms))
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
    let mount = state.mount.clone();
    let decode_sem = state.decode_sem.clone();
    let index = index as usize;

    let permit = decode_sem
        .clone()
        .acquire_owned()
        .await
        .map_err(|e| format!("semaphore closed: {e}"))?;

    let bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        let _permit = permit;
        let stat = mount.stat(&path).map_err(err_string)?;
        let stamp = media::cache::Stamp::new(stat.size, stat.mtime_ms);
        let raw = mount.read(&path).map_err(err_string)?;
        let source = PathBuf::from(&path);
        let decode = || formats::ftx::decode_bytes_frame(&raw, index);
        let result = if full {
            cache.ensure_png_full(&source, stamp, index, decode)
        } else {
            cache.ensure_png(&source, stamp, index, decode)
        };
        result.map_err(err_string)
    })
    .await
    .map_err(|e| format!("join error: {e}"))??;

    Ok(Response::new(bytes))
}
