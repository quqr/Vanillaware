//! Tauri 命令层。重活全部在 `spawn_blocking` 里，Kaitai 生成的 `Rc/RefCell`
//! 类型（`!Send`）绝不出闭包。

use std::path::PathBuf;

use serde::Serialize;
use tauri::ipc::Response;
use tauri::State;

use vw_core::{DirListingDto, FileMetaDto, IndexStatusDto, SearchResultDto};
use vw_media::TextureInfo;

use crate::scene::SceneDto;
use crate::state::AppState;

fn err_string(e: impl std::fmt::Display) -> String {
    e.to_string()
}

// ─── 库：目录与文件 ─────────────────────────────────

/// 读一层目录。子目录全部返回，文件只返回格式注册表收录的。
#[tauri::command]
pub async fn list_dir(path: String) -> Result<DirListingDto, String> {
    let dir = PathBuf::from(&path);
    tokio::task::spawn_blocking(move || vw_core::list_dir(&dir))
        .await
        .map_err(|e| format!("join error: {e}"))?
        .map_err(err_string)
}

/// 单个文件的完整信息（含同目录同基础名的关联文件）。
#[tauri::command]
pub async fn file_meta(path: String) -> Result<FileMetaDto, String> {
    let target = PathBuf::from(&path);
    tokio::task::spawn_blocking(move || vw_core::file_meta(&target))
        .await
        .map_err(|e| format!("join error: {e}"))?
        .map_err(err_string)
}

/// 文件头部字节（十六进制查看器用）。
#[tauri::command]
pub async fn read_file_head(path: String, max_bytes: u32) -> Result<Response, String> {
    let target = PathBuf::from(&path);
    let max = max_bytes.clamp(1, 64 * 1024) as usize;
    let bytes = tokio::task::spawn_blocking(move || vw_core::read_head(&target, max))
        .await
        .map_err(|e| format!("join error: {e}"))?
        .map_err(err_string)?;
    Ok(Response::new(bytes))
}

// ─── 索引 ───────────────────────────────────────────

/// 换根重建索引。立即返回 `building` 状态，进度由 `index_status` 轮询。
#[tauri::command]
pub fn start_index(state: State<'_, AppState>, root: String) -> IndexStatusDto {
    state.start_index(&PathBuf::from(&root));
    state.index_status()
}

#[tauri::command]
pub fn index_status(state: State<'_, AppState>) -> IndexStatusDto {
    state.index_status()
}

/// 全局搜索。索引未建完时返回已收集的部分，`state` 由前端另行读取。
#[tauri::command]
pub async fn search_files(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<SearchResultDto, String> {
    let entries = state.index_entries();
    let limit = limit.map(|v| v as usize).unwrap_or(vw_core::SEARCH_LIMIT);
    tokio::task::spawn_blocking(move || vw_core::search_index(&entries, &query, limit))
        .await
        .map_err(|e| format!("join error: {e}"))
}

// ─── 帧信息与缩略图 ─────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameMetaDto {
    pub index: u32,
    pub width: u32,
    pub height: u32,
    pub format: u16,
    pub format_name: &'static str,
    pub channels: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameListDto {
    pub path: String,
    pub count: u32,
    pub frames: Vec<FrameMetaDto>,
}

fn frame_metas(infos: &[TextureInfo]) -> Vec<FrameMetaDto> {
    infos
        .iter()
        .enumerate()
        .map(|(index, info)| FrameMetaDto {
            index: index as u32,
            width: info.width,
            height: info.height,
            format: info.format,
            format_name: info.format_name(),
            channels: info.channels,
        })
        .collect()
}

#[tauri::command]
pub async fn list_frames(state: State<'_, AppState>, path: String) -> Result<FrameListDto, String> {
    let cache = state.cache.clone();
    let source = PathBuf::from(&path);

    let infos = tokio::task::spawn_blocking(move || -> Result<Vec<TextureInfo>, String> {
        // header-only 解析：只 seek 不读像素。
        let infos = vw_formats::ftx::probe_file(&source).map_err(err_string)?;
        // 顺手落 meta.json，让帧数徽标可以早于缩略图出现。
        cache.ensure_meta(&source, &infos).map_err(err_string)?;
        Ok(infos)
    })
    .await
    .map_err(|e| format!("join error: {e}"))??;

    Ok(FrameListDto {
        path,
        count: infos.len() as u32,
        frames: frame_metas(&infos),
    })
}

/// 取某一帧的 PNG。`full = true` 走原尺寸缓存（查看器/图集），否则走缩略图缓存。
async fn decode_frame_png(
    state: &State<'_, AppState>,
    path: String,
    index: u32,
    full: bool,
) -> Result<Response, String> {
    let key = format!("{}#{}#{}", path.to_lowercase(), index, if full { "full" } else { "thumb" });
    let lock = state.inflight_lock(&key);
    let _guard = lock.lock().await;

    let cache = state.cache.clone();
    let decode_sem = state.decode_sem.clone();
    let source = PathBuf::from(&path);
    let index = index as usize;

    let permit = decode_sem
        .clone()
        .acquire_owned()
        .await
        .map_err(|e| format!("semaphore closed: {e}"))?;

    let bytes = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let decode = || vw_formats::ftx::decode_file_frame(&source, index);
        if full {
            cache.ensure_png_full(&source, index, decode)
        } else {
            cache.ensure_png(&source, index, decode)
        }
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
    .map_err(err_string)?;

    Ok(Response::new(bytes))
}

#[tauri::command]
pub async fn get_thumb_png(
    state: State<'_, AppState>,
    path: String,
    index: u32,
) -> Result<Response, String> {
    decode_frame_png(&state, path, index, false).await
}

/// 原尺寸帧图（纹理查看器主图）。
#[tauri::command]
pub async fn get_frame_png(
    state: State<'_, AppState>,
    path: String,
    index: u32,
) -> Result<Response, String> {
    decode_frame_png(&state, path, index, true).await
}

/// 列表用的缩略图：按格式注册表分派。目前只有纹理有缩略图。
#[tauri::command]
pub async fn file_thumb(state: State<'_, AppState>, path: String) -> Result<Response, String> {
    let format = vw_core::format::lookup(&vw_core::fslist::extension(std::path::Path::new(
        &path,
    )))
    .ok_or_else(|| "该扩展名不在格式注册表里".to_string())?;

    if format.kind != vw_core::Kind::Texture {
        return Err(format!("{} 没有缩略图", format.label));
    }

    decode_frame_png(&state, path, 0, false).await
}

// ─── 预览与转换（ADR-0007 / ADR-0012）────────────────

/// 打开一个 `.mbs` 并摊平成预览场景。
#[tauri::command]
pub async fn preview_scene(
    state: State<'_, AppState>,
    path: String,
    tag: String,
) -> Result<SceneDto, String> {
    let source = PathBuf::from(&path);
    let cache = state.parse_cache();
    tokio::task::spawn_blocking(move || {
        let loaded = cache.load(&source, &tag)?;
        Ok(loaded.scene(&source, &tag))
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// 图集一帧的 PNG（走与缩略图同一套磁盘缓存）。
#[tauri::command]
pub async fn get_atlas_png(
    state: State<'_, AppState>,
    path: String,
    tex_id: i64,
) -> Result<Response, String> {
    let key = format!("atlas#{}#{}", path.to_lowercase(), tex_id);
    let lock = state.inflight_lock(&key);
    let _guard = lock.lock().await;

    let cache = state.cache.clone();
    let decode_sem = state.decode_sem.clone();
    let source = PathBuf::from(&path);

    let permit = decode_sem
        .clone()
        .acquire_owned()
        .await
        .map_err(|e| format!("semaphore closed: {e}"))?;

    let bytes = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        // 只需要同目录的纹理容器，不必解析 `.mbs` 本身。
        let container = vw_formats::atlas::sibling_container(&source)?
            .ok_or_else(|| vw_formats::Error::BadData("没有同名纹理容器".to_string()))?;
        let index = tex_id as usize;
        cache.ensure_png_full(&container, index, || {
            vw_formats::ftx::decode_file_frame(&container, index)
        })
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
    .map_err(err_string)?;

    Ok(Response::new(bytes))
}

/// 单文件 `V55 → Quad` 的结果与自检。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertDto {
    pub mbs_path: String,
    pub tag: String,
    /// 转出来的 Quad（PHP 原样 schema），前端直接渲染成树。
    pub quad: serde_json::Value,
    /// 数组长度、`name`、索引越界等结构不变量。
    pub self_check: Vec<String>,
}

/// 转换一个 `.mbs`。golden 比对是测试套件（`mbs_golden_quad`）的事，界面不暴露。
#[tauri::command]
pub async fn convert_mbs(
    state: State<'_, AppState>,
    path: String,
    tag: String,
) -> Result<ConvertDto, String> {
    let source = PathBuf::from(&path);
    let cache = state.parse_cache();
    tokio::task::spawn_blocking(move || {
        let loaded = cache.load(&source, &tag)?;
        let self_check = vw_formats::mbs::self_check(&loaded.quad);
        let quad = serde_json::to_value(&loaded.quad).map_err(err_string)?;

        Ok(ConvertDto {
            mbs_path: source.display().to_string(),
            tag,
            quad,
            self_check,
        })
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// 导出 Quad JSON 到指定路径；返回写入的字节数。
#[tauri::command]
pub async fn export_quad(
    state: State<'_, AppState>,
    path: String,
    tag: String,
    out_path: String,
) -> Result<u64, String> {
    let source = PathBuf::from(&path);
    let target = PathBuf::from(&out_path);
    let cache = state.parse_cache();
    tokio::task::spawn_blocking(move || {
        let loaded = cache.load(&source, &tag)?;
        let json = serde_json::to_vec_pretty(&loaded.quad).map_err(err_string)?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(err_string)?;
        }
        std::fs::write(&target, &json).map_err(err_string)?;
        Ok(json.len() as u64)
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

// ─── 缓存 ───────────────────────────────────────────
#[tauri::command]
pub async fn get_cache_info(state: State<'_, AppState>) -> Result<vw_media::CacheInfo, String> {
    let cache = state.cache.clone();
    tokio::task::spawn_blocking(move || cache.info())
        .await
        .map_err(|e| format!("join error: {e}"))
}

#[tauri::command]
pub async fn clear_thumb_cache(state: State<'_, AppState>) -> Result<vw_media::CacheInfo, String> {
    let cache = state.cache.clone();
    tokio::task::spawn_blocking(move || cache.clear())
        .await
        .map_err(|e| format!("join error: {e}"))?
        .map_err(err_string)
}
