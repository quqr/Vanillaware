use std::path::{Path, PathBuf};

use tauri::State;

use formats::document::{
    AsmDto, BbsLayoutDto, DocOpenDto, DocPageDto, FmsTextDto, PseudoDto, PAGE_LIMIT, PAGE_MAX,
};
use formats::primitives::{FileGraphDto, VizSceneDto};

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

    let cache = state.doc_cache();
    let target = PathBuf::from(&path);

    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let (handle, cached) = cache.open(&target, tag.as_deref())?;
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
    let cached = state
        .doc_cache()
        .get(handle)
        .ok_or_else(|| "HANDLE_EXPIRED: the document cache evicted this handle, reopen the file".to_string())?;

    let limit = limit.unwrap_or(PAGE_LIMIT).min(PAGE_MAX);
    cached
        .page(handle, node, offset.unwrap_or(0), limit)
        .ok_or_else(|| format!("node {node} does not exist"))
}

#[tauri::command]
pub async fn doc_asm(path: String, function: Option<u32>) -> Result<AsmDto, String> {
    tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        formats::document::asb::disassemble_file(&bytes, function.map(|value| value as usize))
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

#[tauri::command]
pub async fn doc_scene(path: String) -> Result<VizSceneDto, String> {
    tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        Ok(formats::document::osb::scene(&bytes, Path::new(&path)))
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

#[tauri::command]
pub async fn doc_graph(path: String) -> Result<FileGraphDto, String> {
    tokio::task::spawn_blocking(move || {
        let file = Path::new(&path);
        let bytes = std::fs::read(file).map_err(|e| e.to_string())?;
        let ext = catalog::fslist::extension(file);
        Ok(match ext.as_str() {
            "rsc" => formats::document::rsc::graph(&bytes, file),
            "otm" => formats::document::otm::graph(&bytes, file),
            "bbs" => formats::document::bbs::graph(&bytes, file),
            other => {
                return Err(format!("{other} has no reference graph"));
            }
        })
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

#[tauri::command]
pub async fn doc_mesh(path: String) -> Result<VizSceneDto, String> {
    tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        formats::document::fmd::mesh_scene(&bytes)
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

#[tauri::command]
pub async fn doc_layout(path: String) -> Result<BbsLayoutDto, String> {
    tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        Ok(formats::document::bbs::layout(&bytes))
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

#[tauri::command]
pub async fn doc_text(path: String) -> Result<FmsTextDto, String> {
    tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        Ok(formats::document::fms::text_table(&bytes))
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

#[tauri::command]
pub async fn doc_pseudo(path: String, function: Option<u32>) -> Result<PseudoDto, String> {
    tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        formats::document::asb::pseudo_file(&bytes, function.map(|value| value as usize))
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}
