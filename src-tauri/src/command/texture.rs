//! 纹理命令：帧清单、缩略帧、原尺寸帧、图集。

use std::path::PathBuf;

use tauri::State;
use tauri::ipc::Response;

use vw_media::TextureInfo;

use super::{decode_frame_png, err_string, frame_metas};
use crate::model::FrameListDto;
use crate::state::AppState;

/// 帧清单（header-only 解析，不读像素）。
#[tauri::command]
pub async fn list_frames(state: State<'_, AppState>, path: String) -> Result<FrameListDto, String> {
    let cache = state.cache.clone();
    let source = PathBuf::from(&path);

    let infos = tokio::task::spawn_blocking(move || -> Result<Vec<TextureInfo>, String> {
        let infos = vw_formats::ftx::probe_file(&source).map_err(err_string)?;
        // 顺手落 meta.json，让帧数徽标可以早于缩略图出现。
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

/// 缩略帧（列表、网格）。
#[tauri::command]
pub async fn get_thumb_png(
    state: State<'_, AppState>,
    path: String,
    index: u32,
) -> Result<Response, String> {
    decode_frame_png(&state, path, index, false).await
}

/// 原尺寸帧（纹理查看器主图）。
#[tauri::command]
pub async fn get_frame_png(
    state: State<'_, AppState>,
    path: String,
    index: u32,
) -> Result<Response, String> {
    decode_frame_png(&state, path, index, true).await
}

/// 图集帧（动画预览的 UV 采样目标，必须原尺寸）。
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
        // 只需要同目录的纹理容器，不必解析 `.mbs` 本身。
        let container = vw_formats::atlas::sibling_container(&source)?
            .ok_or_else(|| vw_formats::Error::BadData("没有同名纹理容器".to_string()))?;
        let index = tex_id as usize;
        cache.ensure_png_full(&container, index, || {
            vw_formats::ftx::decode_file_frame(&container, index)
        })
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
    .map_err(err_string)?;

    Ok(Response::new(bytes))
}
