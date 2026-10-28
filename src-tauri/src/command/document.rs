//! 文档树命令：打开文档、按节点分页取子树。

use std::path::{Path, PathBuf};

use tauri::State;

use formats::document::{
    AsmDto, BbsLayoutDto, DocOpenDto, DocPageDto, FmsTextDto, PseudoDto, PAGE_LIMIT, PAGE_MAX,
};
use formats::primitives::{FileGraphDto, VizSceneDto};

use crate::state::AppState;

/// 打开文档树；返回根页，后续用 `doc_children` 分页展开。
#[tauri::command]
pub async fn open_doc(
    state: State<'_, AppState>,
    path: String,
    tag: Option<String>,
) -> Result<DocOpenDto, String> {
    let ext = catalog::fslist::extension(Path::new(&path));
    let format = catalog::format::lookup(&ext)
        .ok_or_else(|| format!("该扩展名不在格式注册表里: {ext}"))?;
    if !format.viewers.contains(&"doc") {
        return Err(format!("{} 没有文档查看器", format.label));
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
            .ok_or_else(|| "根节点不存在".to_string())?;
        Ok(DocOpenDto { doc, page })
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// 取某节点的子节点页；句柄被淘汰时返回 `HANDLE_EXPIRED`。
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
        .ok_or_else(|| "HANDLE_EXPIRED: 文档缓存已淘汰，请重新打开".to_string())?;

    let limit = limit.unwrap_or(PAGE_LIMIT).min(PAGE_MAX);
    cached
        .page(handle, node, offset.unwrap_or(0), limit)
        .ok_or_else(|| format!("节点 {node} 不存在"))
}

/// ASB 反汇编：返回函数列表与指定函数的指令行。
#[tauri::command]
pub async fn doc_asm(path: String, function: Option<u32>) -> Result<AsmDto, String> {
    tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        formats::document::asb::disassemble_file(&bytes, function.map(|value| value as usize))
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// OSB 场景组装：节点树 + 资源引用 + 引用 FMD 的灰模网格。
#[tauri::command]
pub async fn doc_scene(path: String) -> Result<VizSceneDto, String> {
    tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        Ok(formats::document::osb::scene(&bytes, Path::new(&path)))
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// 跨文件引用图：RSC（条目 → 载荷 → 资源串）/ OTM（清单 → 资源）/ BBS（装配单 → 名表）。
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
                return Err(format!("{other} 没有引用图"));
            }
        })
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// FMD 可视化首版：非空网格的 positions + 顺序 indices。
#[tauri::command]
pub async fn doc_mesh(path: String) -> Result<VizSceneDto, String> {
    tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        formats::document::fmd::mesh_scene(&bytes)
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// BBS 布局：bbox + 物件 + 雾区四边形 + 相机轨迹锚点。
#[tauri::command]
pub async fn doc_layout(path: String) -> Result<BbsLayoutDto, String> {
    tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        Ok(formats::document::bbs::layout(&bytes))
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// FMS 文本表：全部字符串 + 编码。
#[tauri::command]
pub async fn doc_text(path: String) -> Result<FmsTextDto, String> {
    tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        Ok(formats::document::fms::text_table(&bytes))
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// ASB 伪代码：表达式栈模拟 + label/goto。
#[tauri::command]
pub async fn doc_pseudo(path: String, function: Option<u32>) -> Result<PseudoDto, String> {
    tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        formats::document::asb::pseudo_file(&bytes, function.map(|value| value as usize))
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}
