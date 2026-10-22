//! 运行时配置：缓存位置与并发上限。
//!
//! 只放「可调参数」，不放业务逻辑。改缓存布局时记得同步 `vw_media::ThumbCache` 的 `SCHEMA`。

use std::path::PathBuf;

use tauri::Manager;

/// 缩略图/帧图缓存相对 `app_cache_dir` 的子目录。
const CACHE_SUBDIR: [&str; 2] = ["thumbs", "v1"];

/// 同时解码的帧数上限：列表滚动时避免 CPU 与内存峰值。
pub const DECODE_PERMITS: usize = 3;

/// 缓存根目录（绝对路径）。
pub fn cache_root(app: &tauri::AppHandle) -> tauri::Result<PathBuf> {
    let mut path = app.path().app_cache_dir()?;
    for part in CACHE_SUBDIR {
        path.push(part);
    }
    Ok(path)
}
