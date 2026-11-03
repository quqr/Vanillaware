use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use formats::ftx::TextureInfo;
use formats::{DecodedTexture, Error, Result};
use serde::{Deserialize, Serialize};

use crate::encode;
use crate::hash::fnv1a64_path;

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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheInfo {
    pub root: String,
    pub entries: u64,
    pub bytes: u64,
    pub hits: u64,
    pub misses: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub size: u64,
    pub mtime_ms: u64,
}

impl Stamp {
    pub fn new(size: u64, mtime_ms: u64) -> Self {
        Self { size, mtime_ms }
    }

    pub fn of(path: &Path) -> Self {
        let Ok(metadata) = fs::metadata(path) else {
            return Self {
                size: 0,
                mtime_ms: 0,
            };
        };
        Self {
            size: metadata.len(),
            mtime_ms: mtime_ms(&metadata),
        }
    }
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

    pub fn ensure_meta(
        &self,
        source: &Path,
        stamp: Stamp,
        frames: &[TextureInfo],
    ) -> Result<usize> {
        let meta = self.current_meta(source, stamp)?;
        if let Some(meta) = meta {
            return Ok(meta.frames.len());
        }

        let stored = Meta {
            schema: SCHEMA,
            source: source.display().to_string(),
            size: stamp.size,
            mtime_ms: stamp.mtime_ms,
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

    pub fn ensure_png<F>(
        &self,
        source: &Path,
        stamp: Stamp,
        index: usize,
        decode_frame: F,
    ) -> Result<Vec<u8>>
    where
        F: FnOnce() -> Result<DecodedTexture>,
    {
        let source_lower = source.display().to_string().to_lowercase();
        let dir = self.dir_for(&source_lower);
        let png_path = dir.join(format!("{index}.png"));

        let meta = self.current_meta(source, stamp)?;
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
        let png = encode::render_thumb_png(&decoded)?;

        fs::create_dir_all(&dir).map_err(io_err)?;

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
                    size: stamp.size,
                    mtime_ms: stamp.mtime_ms,
                    frames,
                },
            )?;
        }

        write_atomic(&png_path, &png)?;
        Ok(png)
    }

    pub fn ensure_png_full<F>(
        &self,
        source: &Path,
        stamp: Stamp,
        index: usize,
        decode_frame: F,
    ) -> Result<Vec<u8>>
    where
        F: FnOnce() -> Result<DecodedTexture>,
    {
        let source_lower = source.display().to_string().to_lowercase();
        let dir = self.dir_for(&source_lower).join("full");
        let png_path = dir.join(format!("{index}.png"));

        let meta = self.current_meta(source, stamp)?;
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

    fn current_meta(&self, source: &Path, stamp: Stamp) -> Result<Option<Meta>> {
        let source_lower = source.display().to_string().to_lowercase();
        let meta_path = self.dir_for(&source_lower).join("meta.json");
        let Ok(text) = fs::read_to_string(&meta_path) else {
            return Ok(None);
        };

        let Ok(meta) = serde_json::from_str::<Meta>(&text) else {
            return Ok(None);
        };

        if meta.schema != SCHEMA || meta.size != stamp.size || meta.mtime_ms != stamp.mtime_ms {
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

fn mtime_ms(metadata: &fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
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
