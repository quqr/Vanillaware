//! `.mbs` 解析缓存：按「路径 + tag + 大小 + mtime」命中最近若干份结果。
//!
//! 播放、结构、导出三个入口共用，避免同一个文件被解析多次。

use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};

use super::scene::{self, LoadedMbs};

/// 缓存容量。一份 Quad 约几 MB～十几 MB，4 份足够覆盖「来回切两个文件」。
const CAPACITY: usize = 4;

/// 缓存键：文件被改动（size/mtime 变）即视为不同条目。
#[derive(PartialEq, Eq)]
struct ParseKey {
    path: String,
    tag: String,
    size: u64,
    mtime_ms: u64,
}

type Entries = Arc<Mutex<VecDeque<(ParseKey, Arc<LoadedMbs>)>>>;

/// 解析缓存的可克隆句柄：可安全移进 `spawn_blocking`。
#[derive(Clone)]
pub struct ParseCache {
    entries: Entries,
}

impl Default for ParseCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ParseCache {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// 取（或解析）一个 `.mbs`。命中缓存时不再 parse/convert。
    pub fn load(&self, path: &Path, tag: &str) -> Result<Arc<LoadedMbs>, String> {
        let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
        let key = ParseKey {
            path: vw_core::normalize(path).to_lowercase(),
            tag: tag.to_string(),
            size: metadata.len(),
            mtime_ms: metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        };

        {
            let guard = self.entries.lock().map_err(|e| e.to_string())?;
            if let Some((_, loaded)) = guard.iter().find(|(k, _)| *k == key) {
                return Ok(loaded.clone());
            }
        }

        // 解析在锁外进行：同一文件的并发请求会各解析一次，但不会阻塞其他文件。
        let loaded = Arc::new(scene::load(path, tag).map_err(|e| e.to_string())?);

        let mut guard = self.entries.lock().map_err(|e| e.to_string())?;
        guard.retain(|(k, _)| *k != key);
        guard.push_front((key, loaded.clone()));
        guard.truncate(CAPACITY);

        Ok(loaded)
    }
}
