//! 库视图命令：目录列表、文件详情、列表缩略图、头部字节。

use std::path::PathBuf;

use tauri::State;
use tauri::ipc::Response;

use vw_core::{DirListingDto, FileMetaDto, Kind};

use super::{decode_frame_png, err_string};
use crate::state::AppState;

/// 读一层目录。子目录全部返回，文件只返回格式注册表收录的。
#[tauri::command]
pub async fn list_dir(path: String) -> Result<DirListingDto, String> {
    let dir = PathBuf::from(&path);
    tokio::task::spawn_blocking(move || vw_core::list_dir(&dir))
        .await
        .map_err(|e| format!("join error: {e}"))?
        .map_err(err_string)
}

/// 单个文件的完整信息（含同目录同基础名的关联文件）。
#[tauri::command]
pub async fn file_meta(path: String) -> Result<FileMetaDto, String> {
    let target = PathBuf::from(&path);
    tokio::task::spawn_blocking(move || vw_core::file_meta(&target))
        .await
        .map_err(|e| format!("join error: {e}"))?
        .map_err(err_string)
}

/// 列表/网格用的缩略图：按格式注册表分派，目前只有纹理有缩略图。
#[tauri::command]
pub async fn file_thumb(state: State<'_, AppState>, path: String) -> Result<Response, String> {
    let format = vw_core::format::lookup(&vw_core::fslist::extension(std::path::Path::new(&path)))
        .ok_or_else(|| "该扩展名不在格式注册表里".to_string())?;

    if format.kind != Kind::Texture {
        return Err(format!("{} 没有缩略图", format.label));
    }

    decode_frame_png(&state, path, 0, false).await
}

/// 文件头部字节（十六进制查看器用）。
#[tauri::command]
pub async fn read_file_head(path: String, max_bytes: u32) -> Result<Response, String> {
    let target = PathBuf::from(&path);
    let max = max_bytes.clamp(1, 64 * 1024) as usize;
    let bytes = tokio::task::spawn_blocking(move || vw_core::read_head(&target, max))
        .await
        .map_err(|e| format!("join error: {e}"))?
        .map_err(err_string)?;
    Ok(Response::new(bytes))
}
