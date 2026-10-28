use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, UNIX_EPOCH};

use formats::document::{DocArena, DocDto, DocPageDto};

const CAPACITY: usize = 8;
const MAX_BYTES: usize = 96 * 1024 * 1024;

#[derive(Clone, PartialEq, Eq)]
struct DocKey {
    path: String,
    tag: String,
    size: u64,
    mtime_ms: u64,
}

pub struct CachedDoc {
    pub arena: DocArena,
    path: String,
    ext: String,
    label: String,
    size: u64,
    build_ms: u32,
}

impl CachedDoc {
    pub fn doc(&self, handle: u64) -> DocDto {
        self.arena.doc(
            handle,
            self.path.clone(),
            self.ext.clone(),
            self.label.clone(),
            self.size,
            self.build_ms,
        )
    }

    pub fn page(&self, handle: u64, node: u32, offset: u32, limit: u32) -> Option<DocPageDto> {
        self.arena.page(handle, node, offset, limit)
    }
}

struct Entry {
    handle: u64,
    key: DocKey,
    doc: Arc<CachedDoc>,
}

#[derive(Clone)]
pub struct DocCache {
    entries: Arc<Mutex<VecDeque<Entry>>>,
    next_handle: Arc<AtomicU64>,
}

impl Default for DocCache {
    fn default() -> Self {
        Self::new()
    }
}

impl DocCache {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::new())),
            next_handle: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn open(&self, path: &Path, tag: Option<&str>) -> Result<(u64, Arc<CachedDoc>), String> {
        let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
        let ext = catalog::fslist::extension(path);
        let key = DocKey {
            path: catalog::normalize(path).to_lowercase(),
            tag: tag.unwrap_or("").to_string(),
            size: metadata.len(),
            mtime_ms: metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis() as u64)
                .unwrap_or(0),
        };

        {
            let guard = self.entries.lock().map_err(|e| e.to_string())?;
            if let Some(entry) = guard.iter().find(|entry| entry.key == key) {
                return Ok((entry.handle, entry.doc.clone()));
            }
        }

        let started = Instant::now();
        let arena = formats::document::parse_file(path, tag).map_err(|e| e.to_string())?;
        let build_ms = started.elapsed().as_millis() as u32;
        let label = catalog::format::lookup(&ext)
            .map(|format| format.label)
            .unwrap_or("BIN");

        let doc = Arc::new(CachedDoc {
            arena,
            path: catalog::normalize(path),
            ext,
            label: label.to_string(),
            size: metadata.len(),
            build_ms,
        });

        let handle = self.next_handle.fetch_add(1, Ordering::Relaxed) + 1;
        let mut guard = self.entries.lock().map_err(|e| e.to_string())?;
        guard.push_front(Entry {
            handle,
            key,
            doc: doc.clone(),
        });
        evict(&mut guard);

        Ok((handle, doc))
    }

    pub fn get(&self, handle: u64) -> Option<Arc<CachedDoc>> {
        let guard = self.entries.lock().ok()?;
        guard
            .iter()
            .find(|entry| entry.handle == handle)
            .map(|entry| entry.doc.clone())
    }
}

fn evict(entries: &mut VecDeque<Entry>) {
    while entries.len() > CAPACITY {
        entries.pop_back();
    }
    while entries.len() > 1 {
        let bytes: usize = entries
            .iter()
            .map(|entry| entry.doc.arena.approx_bytes() + 1024)
            .sum();
        if bytes <= MAX_BYTES {
            break;
        }
        entries.pop_back();
    }
}
