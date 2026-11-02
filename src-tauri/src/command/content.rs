use std::path::Path;

use tauri::State;

use media::content::ContentDto;

use super::{err_string, read_bytes};
use crate::state::AppState;

#[tauri::command]
pub async fn content_open(state: State<'_, AppState>, path: String) -> Result<ContentDto, String> {
    let ext = catalog::fslist::extension(Path::new(&path));

    if ext == "ufb" {
        let (parent, name) = split_directory(&path);
        let directory = state.mount.real(&parent).map_err(err_string)?;
        let target = directory.join(name);
        return tokio::task::spawn_blocking(move || {
            media::content::open_path(&target).ok_or_else(|| format!(".{ext} has no content view"))
        })
        .await
        .map_err(|error| format!("join error: {error}"))?;
    }

    let bytes = read_bytes(&state, &path)?;

    tokio::task::spawn_blocking(move || {
        media::content::open(&bytes, &ext).ok_or_else(|| format!(".{ext} has no content view"))
    })
    .await
    .map_err(|error| format!("join error: {error}"))?
}

fn split_directory(raw: &str) -> (String, String) {
    let text = raw.trim_end_matches(['\\', '/']);
    match text.rfind(['\\', '/']) {
        Some(at) => (text[..at].to_string(), text[at + 1..].to_string()),
        None => (String::new(), text.to_string()),
    }
}
