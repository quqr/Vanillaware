use std::path::Path;

use tauri::State;

use formats::content::UcbMode;
use media::ContentDto;

use crate::model::FieldMapDto;
use crate::state::AppState;

const MAX_SIDE: u32 = 1920;

#[tauri::command]
pub async fn field_maps(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<FieldMapDto>, String> {
    let (bytes, size, mtime_ms, _) = super::read_source(&state, &path)?;
    if catalog::fslist::extension(Path::new(&path)) != "ufb" {
        return Ok(Vec::new());
    }
    let cache = state.field_cache();
    tokio::task::spawn_blocking(move || {
        let (_, maps) = cache.open(&path, size, mtime_ms, &bytes)?;
        Ok(maps.iter().map(crate::service::field::to_dto).collect())
    })
    .await
    .map_err(|error| format!("join error: {error}"))?
}

#[tauri::command]
pub async fn field_render(
    state: State<'_, AppState>,
    path: String,
    layer: Option<String>,
    x0: Option<f32>,
    width: Option<f32>,
    mode: Option<String>,
) -> Result<ContentDto, String> {
    let ext = catalog::fslist::extension(Path::new(&path));
    let width = width.unwrap_or(1920.0).clamp(400.0, 12000.0);
    let key = format!(
        "{}|{}|{}|{}|{}|{}",
        path.to_lowercase(),
        layer.clone().unwrap_or_default(),
        x0.map(|value| value.round() as i32).unwrap_or(i32::MIN),
        width.round() as i32,
        mode.clone().unwrap_or_default(),
        ext
    );

    let cache = state.field_cache();
    if let Some(hit) = cache.cached(&key) {
        return Ok((*hit).clone());
    }

    let (bytes, size, mtime_ms, directory) = super::read_source(&state, &path)?;
    let mode = match mode.as_deref() {
        Some("class") => UcbMode::Class,
        _ => UcbMode::Area,
    };

    let worker = cache.clone();
    let dto = tokio::task::spawn_blocking(move || -> Result<ContentDto, String> {
        let projection = match ext.as_str() {
            "ufb" => {
                let (layout, _) = worker.open(&path, size, mtime_ms, &bytes)?;
                formats::field::view_layout(
                    &layout,
                    &bytes,
                    directory.as_deref(),
                    layer.as_deref(),
                    x0,
                    width,
                    MAX_SIDE,
                )
            }
            "ucb" => formats::content::ucb_at(&bytes, mode),
            other => return Err(format!(".{other} has no field view")),
        };
        Ok(media::content::render(projection))
    })
    .await
    .map_err(|error| format!("join error: {error}"))??;

    let dto = std::sync::Arc::new(dto);
    cache.store(key, dto.clone());
    Ok((*dto).clone())
}
