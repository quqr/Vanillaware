use std::path::PathBuf;

use tauri::Manager;

const CACHE_SUBDIR: [&str; 2] = ["thumbs", "v1"];

pub const DECODE_PERMITS: usize = 3;

pub const DOC_PERMITS: usize = 2;

pub fn cache_root(app: &tauri::AppHandle) -> tauri::Result<PathBuf> {
    let mut path = app.path().app_cache_dir()?;
    for part in CACHE_SUBDIR {
        path.push(part);
    }
    Ok(path)
}
