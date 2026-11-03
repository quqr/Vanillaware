use std::path::Path;
use std::sync::Arc;

use tauri::State;

use formats::document::asb::AsbModule;
use formats::document::{
    AsmDto, BbsLayoutDto, DocOpenDto, DocPageDto, FmsTextDto, PAGE_LIMIT, PAGE_MAX, PseudoDto,
};
use formats::primitives::{FileGraphDto, VizSceneDto};

use super::{err_string, read_bytes, real_path};
use crate::state::AppState;

#[tauri::command]
pub async fn open_doc(
    state: State<'_, AppState>,
    path: String,
    tag: Option<String>,
) -> Result<DocOpenDto, String> {
    let ext = catalog::fslist::extension(Path::new(&path));
    let format = catalog::format::lookup(&ext)
        .ok_or_else(|| format!("extension is not in the format registry: {ext}"))?;
    if !format.viewers.contains(&"doc") {
        return Err(format!("{} has no document viewer", format.label));
    }

    let permit = state
        .doc_sem
        .clone()
        .acquire_owned()
        .await
        .map_err(|e| format!("semaphore closed: {e}"))?;

    let bytes = read_bytes(&state, &path)?;
    let stat = state.mount.stat(&path).map_err(err_string)?;
    let cache = state.doc_cache();

    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let (handle, cached) =
            cache.open(&path, &bytes, stat.size, stat.mtime_ms, tag.as_deref())?;
        let doc = cached.doc(handle);
        let page = cached
            .page(handle, doc.root, 0, PAGE_LIMIT)
            .ok_or_else(|| "root node is missing".to_string())?;
        Ok(DocOpenDto { doc, page })
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

#[tauri::command]
pub fn doc_children(
    state: State<'_, AppState>,
    handle: u64,
    node: u32,
    offset: Option<u32>,
    limit: Option<u32>,
) -> Result<DocPageDto, String> {
    let cached = state.doc_cache().get(handle).ok_or_else(|| {
        "HANDLE_EXPIRED: the document cache evicted this handle, reopen the file".to_string()
    })?;

    let limit = limit.unwrap_or(PAGE_LIMIT).min(PAGE_MAX);
    cached
        .page(handle, node, offset.unwrap_or(0), limit)
        .ok_or_else(|| format!("node {node} does not exist"))
}

/// 复用 doc 信号量并缓存解析结果，避免每切换一个函数就重读整个模块。
async fn load_asm_module(state: &AppState, path: &str) -> Result<Arc<AsbModule>, String> {
    let stat = state.mount.stat(path).map_err(err_string)?;
    let size = stat.size;
    let mtime_ms = stat.mtime_ms;
    if let Some(module) = state.asm_cache().get(path, size, mtime_ms) {
        return Ok(module);
    }

    let permit = state
        .doc_sem
        .clone()
        .acquire_owned()
        .await
        .map_err(|e| format!("semaphore closed: {e}"))?;
    let bytes = read_bytes(state, path)?;
    let cache = state.asm_cache();
    let key = path.to_string();

    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let module = Arc::new(AsbModule::parse(&bytes)?);
        cache.insert(&key, size, mtime_ms, module.clone());
        Ok(module)
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

#[tauri::command]
pub async fn doc_asm(
    state: State<'_, AppState>,
    path: String,
    function: Option<u32>,
) -> Result<AsmDto, String> {
    let module = load_asm_module(state.inner(), &path).await?;
    tokio::task::spawn_blocking(move || module.disassemble(function.map(|value| value as usize)))
        .await
        .map_err(|e| format!("join error: {e}"))
}

#[tauri::command]
pub async fn doc_scene(state: State<'_, AppState>, path: String) -> Result<VizSceneDto, String> {
    let bytes = read_bytes(&state, &path)?;
    let real = real_path(&state, &path)?;
    tokio::task::spawn_blocking(move || Ok(formats::document::osb::scene(&bytes, &real)))
        .await
        .map_err(|e| format!("join error: {e}"))?
}

#[tauri::command]
pub async fn doc_graph(state: State<'_, AppState>, path: String) -> Result<FileGraphDto, String> {
    let bytes = read_bytes(&state, &path)?;
    let real = real_path(&state, &path)?;
    let ext = catalog::fslist::extension(Path::new(&path));

    tokio::task::spawn_blocking(move || {
        Ok(match ext.as_str() {
            "rsc" => formats::document::rsc::graph(&bytes, &real),
            "otm" => formats::document::otm::graph(&bytes, &real),
            "bbs" => formats::document::bbs::graph(&bytes, &real),
            other => {
                return Err(format!("{other} has no reference graph"));
            }
        })
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

#[tauri::command]
pub async fn doc_mesh(state: State<'_, AppState>, path: String) -> Result<VizSceneDto, String> {
    let bytes = read_bytes(&state, &path)?;
    tokio::task::spawn_blocking(move || formats::document::fmd::mesh_scene(&bytes))
        .await
        .map_err(|e| format!("join error: {e}"))?
}

#[tauri::command]
pub async fn doc_layout(state: State<'_, AppState>, path: String) -> Result<BbsLayoutDto, String> {
    let bytes = read_bytes(&state, &path)?;
    tokio::task::spawn_blocking(move || Ok(formats::document::bbs::layout(&bytes)))
        .await
        .map_err(|e| format!("join error: {e}"))?
}

#[tauri::command]
pub async fn doc_text(state: State<'_, AppState>, path: String) -> Result<FmsTextDto, String> {
    let bytes = read_bytes(&state, &path)?;
    tokio::task::spawn_blocking(move || Ok(formats::document::fms::text_table(&bytes)))
        .await
        .map_err(|e| format!("join error: {e}"))?
}

#[tauri::command]
pub async fn doc_pseudo(
    state: State<'_, AppState>,
    path: String,
    function: Option<u32>,
) -> Result<PseudoDto, String> {
    let module = load_asm_module(state.inner(), &path).await?;
    tokio::task::spawn_blocking(move || module.pseudo(function.map(|value| value as usize)))
        .await
        .map_err(|e| format!("join error: {e}"))
}
