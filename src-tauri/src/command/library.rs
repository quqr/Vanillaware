//! 库视图命令：目录列表、文件详情、列表缩略图、头部字节。

use std::path::PathBuf;

use tauri::State;
use tauri::ipc::Response;

use catalog::{DirListingDto, FileMetaDto, FormatDto, GameProfileDto};

use super::{decode_frame_png, err_string};
use crate::state::AppState;

/// 读一层目录。子目录全部返回，文件只返回格式注册表收录的。
#[tauri::command]
pub async fn list_dir(path: String) -> Result<DirListingDto, String> {
    let dir = PathBuf::from(&path);
    tokio::task::spawn_blocking(move || catalog::list_dir(&dir))
        .await
        .map_err(|e| format!("join error: {e}"))?
        .map_err(err_string)
}

/// 单个文件的完整信息（含同目录同基础名的关联文件）。
#[tauri::command]
pub async fn file_meta(path: String) -> Result<FileMetaDto, String> {
    let target = PathBuf::from(&path);
    tokio::task::spawn_blocking(move || catalog::file_meta(&target))
        .await
        .map_err(|e| format!("join error: {e}"))?
        .map_err(err_string)
}

/// 注册表快照：工具栏过滤项与图标语义的唯一真值。
#[tauri::command]
pub fn list_formats() -> Vec<FormatDto> {
    catalog::FORMATS.iter().map(FormatDto::from).collect()
}

/// 游戏档案嗅探：有界目录扫描，不读文件内容。
#[tauri::command]
pub async fn detect_profile(root: String) -> Result<GameProfileDto, String> {
    tokio::task::spawn_blocking(move || {
        catalog::detect_game(std::path::Path::new(&root))
    })
    .await
    .map_err(|e| format!("join error: {e}"))
}

/// 列表/网格用的缩略图：能力来自格式声明。
#[tauri::command]
pub async fn file_thumb(state: State<'_, AppState>, path: String) -> Result<Response, String> {
    let format = catalog::format::lookup(&catalog::fslist::extension(std::path::Path::new(&path)))
        .ok_or_else(|| "该扩展名不在格式注册表里".to_string())?;

    if !format.has_thumb {
        return Err(format!("{} 没有缩略图", format.label));
    }

    decode_frame_png(&state, path, 0, false).await
}

/// 文件头部字节（十六进制查看器用）。
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
