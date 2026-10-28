//! 缩略图磁盘缓存。
//!
//! 布局：`<root>/<fnv1a64(小写路径)>/{meta.json, 0.png, 1.png …}`
//! 失效判定：`schema + size + mtime`。写入一律 `.tmp → rename` 原子替换。
//! 并发去重（同一文件多个请求合并）由上层 command 负责，这里只保证单次操作原子。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use formats::ftx::TextureInfo;
use formats::{DecodedTexture, Error, Result};

use crate::encode;
use crate::hash::fnv1a64_path;

/// 缓存目录结构版本：修改解码算法或缓存文件布局时递增，使旧缓存自动失效。
///
/// 历史：v1 使用原始 Tegra X1 swizzle（分档位表），对大于 64 块的纹理块映射错误；
/// v2 按 rip-vanillaware 的 GOB 瓦片算法重写了解码管线；
/// v3 修正 BC7 解码高度换算与块连续序列重排（此前与 PHP 参考仅 45.88% 一致）；
/// v4 区分缩略图与原尺寸帧图：此前查看器误用缩略图，图集被压到 256px；
/// v5 缩略图降到 128px 并改用快速压缩档。
const SCHEMA: u32 = 5;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct FrameInfo {
    w: u32,
    h: u32,
    format: u16,
    channels: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Meta {
    schema: u32,
    source: String,
    size: u64,
    mtime_ms: u64,
    frames: Vec<FrameInfo>,
}

/// 缓存状态汇总。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheInfo {
    pub root: String,
    pub entries: u64,
    pub bytes: u64,
    pub hits: u64,
    pub misses: u64,
}

pub struct ThumbCache {
    root: PathBuf,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl ThumbCache {
    pub fn new<P: Into<PathBuf>>(root: P) -> Self {
        Self {
            root: root.into(),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    pub fn misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    fn dir_for(&self, source_lower: &str) -> PathBuf {
        self.root.join(fnv1a64_path(source_lower))
    }

    /// 把 probe 结果落盘成 `meta.json`。返回落盘的帧数。
    ///
    /// 缓存已有效时直接返回（不重写），所以可以每次都调用。
    pub fn ensure_meta(&self, source: &Path, frames: &[TextureInfo]) -> Result<usize> {
        let meta = self.current_meta(source)?;
        if let Some(meta) = meta {
            return Ok(meta.frames.len());
        }

        let stored = Meta {
            schema: SCHEMA,
            source: source.display().to_string(),
            size: file_size(source)?,
            mtime_ms: file_mtime_ms(source),
            frames: frames
                .iter()
                .map(|f| FrameInfo {
                    w: f.width,
                    h: f.height,
                    format: f.format,
                    channels: f.channels,
                })
                .collect(),
        };

        self.write_meta(source, &stored)?;
        Ok(stored.frames.len())
    }

    /// 取第 `index` 帧的 PNG 字节。
    ///
    /// 命中时直接读缓存（`hits`）；未命中时调用 `decode_frame` 补一张（`misses`）。
    /// `frames` 来自 probe，用于写 meta；两者索引必须一致。
    pub fn ensure_png<F>(&self, source: &Path, index: usize, decode_frame: F) -> Result<Vec<u8>>
    where
        F: FnOnce() -> Result<DecodedTexture>,
    {
        let source_lower = source.display().to_string().to_lowercase();
        let dir = self.dir_for(&source_lower);
        let png_path = dir.join(format!("{index}.png"));

        let meta = self.current_meta(source)?;
        let valid = meta
            .as_ref()
            .is_some_and(|m| m.schema == SCHEMA && index < m.frames.len());

        if valid {
            match fs::read(&png_path) {
                Ok(bytes) => {
                    self.hits.fetch_add(1, Ordering::Relaxed);
                    return Ok(bytes);
                }
                Err(_) => {
                    // meta 有效但 PNG 缺失：走重建，同时补 meta。
                }
            }
        }

        self.misses.fetch_add(1, Ordering::Relaxed);

        let decoded = decode_frame()?;
        let png = encode::render_thumb_png(&decoded)?;

        fs::create_dir_all(&dir).map_err(io_err)?;

        // meta 无效则用解码结果重建（保证至少包含被请求的这一帧）。
        if !valid {
            let mut frames = meta.map(|m| m.frames).unwrap_or_default();
            if index >= frames.len() {
                frames.resize(
                    index + 1,
                    FrameInfo {
                        w: 0,
                        h: 0,
                        format: 0,
                        channels: 4,
                    },
                );
            }
            frames[index] = FrameInfo {
                w: decoded.width,
                h: decoded.height,
                format: decoded.format,
                channels: decoded.channels,
            };

            self.write_meta(
                source,
                &Meta {
                    schema: SCHEMA,
                    source: source.display().to_string(),
                    size: file_size(source)?,
                    mtime_ms: file_mtime_ms(source),
                    frames,
                },
            )?;
        }

        write_atomic(&png_path, &png)?;
        Ok(png)
    }

    /// 取第 `index` 帧的**原尺寸** PNG（查看器与图集采样用）。
    ///
    /// 与 [`Self::ensure_png`] 共用 meta 失效判定，但落在 `full/{index}.png`，
    /// 避免把查看器请求的图也压成缩略图。
    pub fn ensure_png_full<F>(
        &self,
        source: &Path,
        index: usize,
        decode_frame: F,
    ) -> Result<Vec<u8>>
    where
        F: FnOnce() -> Result<DecodedTexture>,
    {
        let source_lower = source.display().to_string().to_lowercase();
        let dir = self.dir_for(&source_lower).join("full");
        let png_path = dir.join(format!("{index}.png"));

        let meta = self.current_meta(source)?;
        let valid = meta
            .as_ref()
            .is_some_and(|m| m.schema == SCHEMA && index < m.frames.len());

        if valid {
            if let Ok(bytes) = fs::read(&png_path) {
                self.hits.fetch_add(1, Ordering::Relaxed);
                return Ok(bytes);
            }
        }

        self.misses.fetch_add(1, Ordering::Relaxed);
        let decoded = decode_frame()?;
        let png = encode::render_full_png(&decoded)?;
        fs::create_dir_all(&dir).map_err(io_err)?;
        write_atomic(&png_path, &png)?;
        Ok(png)
    }

    /// 遍历缓存目录统计体积。目录很大时这会慢，只在「缓存信息」按钮里调用。
    pub fn info(&self) -> CacheInfo {
        let (entries, bytes) = walk_stats(&self.root);
        CacheInfo {
            root: self.root.display().to_string(),
            entries,
            bytes,
            hits: self.hits(),
            misses: self.misses(),
        }
    }

    /// 清空缓存并重建空目录。返回清掉的内容。
    pub fn clear(&self) -> Result<CacheInfo> {
        let (entries, bytes) = walk_stats(&self.root);
        let info = CacheInfo {
            root: self.root.display().to_string(),
            entries,
            bytes,
            hits: self.hits(),
            misses: self.misses(),
        };

        if self.root.exists() {
            fs::remove_dir_all(&self.root).map_err(io_err)?;
        }
        fs::create_dir_all(&self.root).map_err(io_err)?;

        Ok(info)
    }

    fn current_meta(&self, source: &Path) -> Result<Option<Meta>> {
        let source_lower = source.display().to_string().to_lowercase();
        let meta_path = self.dir_for(&source_lower).join("meta.json");
        let Ok(text) = fs::read_to_string(&meta_path) else {
            return Ok(None);
        };

        let Ok(meta) = serde_json::from_str::<Meta>(&text) else {
            return Ok(None);
        };

        if meta.schema != SCHEMA
            || meta.size != file_size(source)?
            || meta.mtime_ms != file_mtime_ms(source)
        {
            return Ok(None);
        }

        Ok(Some(meta))
    }

    fn write_meta(&self, source: &Path, meta: &Meta) -> Result<()> {
        let source_lower = source.display().to_string().to_lowercase();
        let dir = self.dir_for(&source_lower);
        fs::create_dir_all(&dir).map_err(io_err)?;

        let path = dir.join("meta.json");
        let text = serde_json::to_string(meta).map_err(json_err)?;
        write_atomic(&path, text.as_bytes())
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let temp = path.with_extension("tmp");
    fs::write(&temp, bytes).map_err(io_err)?;
    fs::rename(&temp, path).map_err(io_err)?;
    Ok(())
}

fn file_size(path: &Path) -> Result<u64> {
    Ok(fs::metadata(path).map_err(io_err)?.len())
}

fn file_mtime_ms(path: &Path) -> u64 {
    fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn io_err(e: std::io::Error) -> Error {
    Error::Io(e)
}

fn json_err(e: serde_json::Error) -> Error {
    Error::BadData(format!("json error: {e}"))
}

fn walk_stats(root: &Path) -> (u64, u64) {
    let mut entries = 0u64;
    let mut bytes = 0u64;

    let Ok(read) = fs::read_dir(root) else {
        return (0, 0);
    };

    for dir in read.flatten() {
        if !dir.path().is_dir() {
            continue;
        }

        let has_meta = dir.path().join("meta.json").is_file();
        if !has_meta {
            continue;
        }

        entries += 1;
        if let Ok(files) = fs::read_dir(dir.path()) {
            for file in files.flatten() {
                if let Ok(meta) = file.metadata() {
                    bytes += meta.len();
                }
            }
        }
    }

    (entries, bytes)
}
