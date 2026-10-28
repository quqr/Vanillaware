use std::path::{Path, PathBuf};

use media::content::ContentDto;

#[tauri::command]
pub async fn content_open(path: String) -> Result<ContentDto, String> {
    let target = PathBuf::from(&path);
    let ext = catalog::fslist::extension(Path::new(&path));

    tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&target).map_err(|error| error.to_string())?;
        media::content::open(&bytes, &ext).ok_or_else(|| format!(".{ext} has no content view"))
    })
    .await
    .map_err(|error| format!("join error: {error}"))?
}
