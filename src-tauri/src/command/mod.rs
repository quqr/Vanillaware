//! Tauri 命令层：参数校验 + 阻塞任务调度。
//!
//! 命令只做三件事：把重活丢进 `spawn_blocking`、把服务结果转成 DTO、把错误转成字符串。
//! 业务逻辑在 [`crate::service`]，数据结构在 [`crate::model`]。

pub mod animation;
pub mod audio;
pub mod cache;
pub mod content;
pub mod doc;
pub mod index;
pub mod library;
pub mod texture;
pub mod video;

use std::path::PathBuf;

use tauri::State;
use tauri::ipc::Response;

use vw_media::TextureInfo;

use crate::model::FrameMetaDto;
use crate::state::AppState;

/// 统一把任意错误转成前端可读字符串。
pub(crate) fn err_string(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// 纹理帧元信息 → DTO。
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

/// 取某一帧的 PNG。
///
/// `full = true` 走原尺寸缓存（查看器与图集采样），否则走缩略图缓存。
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
        let decode = || vw_formats::ftx::decode_file_frame(&source, index);
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
