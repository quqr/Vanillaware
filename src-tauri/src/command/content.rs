//! 内容视图命令：一次调用返回表格 / 位图 / 折线 / 文本。

use std::path::{Path, PathBuf};

use media::content::ContentDto;

/// 读取文件并投影成内容视图。
#[tauri::command]
pub async fn content_open(path: String) -> Result<ContentDto, String> {
    let target = PathBuf::from(&path);
    let ext = catalog::fslist::extension(Path::new(&path));

    tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&target).map_err(|error| error.to_string())?;
        media::content::open(&bytes, &ext).ok_or_else(|| format!(".{ext} 没有内容视图"))
    })
    .await
    .map_err(|error| format!("join error: {error}"))?
}
