use std::path::Path;

use tauri::State;

use formats::content::UcbMode;
use formats::field::Render;
use media::ContentDto;

use crate::model::{FieldMapDto, FieldOverviewDto};
use crate::state::AppState;

const MAX_SIDE: u32 = 1920;
const OVERVIEW_SIDE: u32 = 2400;

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
    clock: Option<u64>,
    terrain: Option<bool>,
) -> Result<ContentDto, String> {
    let ext = catalog::fslist::extension(Path::new(&path));
    let width = width.unwrap_or(1920.0).clamp(400.0, 12000.0);
    let render = Render {
        clock: clock.unwrap_or(0),
        terrain: terrain.unwrap_or(false),
    };
    let key = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        path.to_lowercase(),
        layer.clone().unwrap_or_default(),
        x0.map(|value| value.round() as i32).unwrap_or(i32::MIN),
        width.round() as i32,
        mode.clone().unwrap_or_default(),
        render.clock,
        render.terrain,
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
                    render,
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

/// One strip for the whole layer set, used as the minimap behind the window.
#[tauri::command]
pub async fn field_overview(
    state: State<'_, AppState>,
    path: String,
    layer: Option<String>,
) -> Result<FieldOverviewDto, String> {
    if catalog::fslist::extension(Path::new(&path)) != "ufb" {
        return Err("only .ufb files carry a field layout".to_string());
    }
    let (bytes, size, mtime_ms, directory) = super::read_source(&state, &path)?;
    let cache = state.field_cache();
    let key = format!(
        "overview|{}|{}",
        path.to_lowercase(),
        layer.clone().unwrap_or_default()
    );
    if let Some(hit) = cache.overview(&key) {
        return Ok((*hit).clone());
    }

    let worker = cache.clone();
    let dto = tokio::task::spawn_blocking(move || -> Result<FieldOverviewDto, String> {
        let (layout, maps) = worker.open(&path, size, mtime_ms, &bytes)?;
        let dir = directory.ok_or("the file has no directory")?;
        let scope = layer
            .clone()
            .or_else(|| maps.first().map(|entry| entry.name.clone()))
            .ok_or("the field has no layer set")?;
        let content = media::content::render(formats::field::overview_view(
            &bytes,
            &layout,
            Some(&dir),
            &scope,
            OVERVIEW_SIDE,
        ));
        let span = formats::field::scope_span(&layout, &scope).unwrap_or([0.0; 4]);
        Ok(FieldOverviewDto {
            image: content.image,
            span,
        })
    })
    .await
    .map_err(|error| format!("join error: {error}"))??;

    let dto = std::sync::Arc::new(dto);
    cache.store_overview(key, dto.clone());
    Ok((*dto).clone())
}
