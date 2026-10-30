use std::path::{Path, PathBuf};

use media::content::ContentDto;

#[tauri::command]
pub async fn content_open(path: String) -> Result<ContentDto, String> {
    let target = PathBuf::from(&path);
    let ext = catalog::fslist::extension(Path::new(&path));

    tokio::task::spawn_blocking(move || {
        media::content::open_path(&target).ok_or_else(|| format!(".{ext} has no content view"))
    })
    .await
    .map_err(|error| format!("join error: {error}"))?
}
