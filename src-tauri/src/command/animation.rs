//! 动画命令：预览场景、结构转换、导出 Quad。
//!
//! 三者共用 [`crate::service::ParseCache`]，同一个文件只会解析一次。

use std::path::PathBuf;

use tauri::State;

use super::err_string;
use crate::model::ConvertDto;
use crate::model::SceneDto;
use crate::state::AppState;

/// 打开一个 `.mbs` 并摊平成预览场景。
#[tauri::command]
pub async fn preview_scene(
    state: State<'_, AppState>,
    path: String,
    tag: String,
) -> Result<SceneDto, String> {
    let source = PathBuf::from(&path);
    let cache = state.parse_cache();
    tokio::task::spawn_blocking(move || {
        let loaded = cache.load(&source, &tag)?;
        Ok(loaded.scene(&source, &tag))
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// 转换一个 `.mbs`。golden 比对是测试套件（`mbs_golden_quad`）的事，界面不暴露。
#[tauri::command]
pub async fn convert_mbs(
    state: State<'_, AppState>,
    path: String,
    tag: String,
) -> Result<ConvertDto, String> {
    let source = PathBuf::from(&path);
    let cache = state.parse_cache();
    tokio::task::spawn_blocking(move || {
        let loaded = cache.load(&source, &tag)?;
        let self_check = vw_formats::mbs::self_check(&loaded.quad);
        let quad = serde_json::to_value(&loaded.quad).map_err(err_string)?;

        Ok(ConvertDto {
            mbs_path: source.display().to_string(),
            tag,
            quad,
            self_check,
        })
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// 导出 Quad JSON 到指定路径；返回写入的字节数。
#[tauri::command]
pub async fn export_quad(
    state: State<'_, AppState>,
    path: String,
    tag: String,
    out_path: String,
) -> Result<u64, String> {
    let source = PathBuf::from(&path);
    let target = PathBuf::from(&out_path);
    let cache = state.parse_cache();
    tokio::task::spawn_blocking(move || {
        let loaded = cache.load(&source, &tag)?;
        let json = serde_json::to_vec_pretty(&loaded.quad).map_err(err_string)?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(err_string)?;
        }
        std::fs::write(&target, &json).map_err(err_string)?;
        Ok(json.len() as u64)
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}
