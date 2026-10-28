use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};

use super::scene::{self, LoadedMbs};

const CAPACITY: usize = 4;

#[derive(PartialEq, Eq)]
struct ParseKey {
    path: String,
    tag: String,
    size: u64,
    mtime_ms: u64,
}

type Entries = Arc<Mutex<VecDeque<(ParseKey, Arc<LoadedMbs>)>>>;

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

    pub fn load(&self, path: &Path, tag: &str) -> Result<Arc<LoadedMbs>, String> {
        let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
        let key = ParseKey {
            path: catalog::normalize(path).to_lowercase(),
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

        let loaded = Arc::new(scene::load(path, tag).map_err(|e| e.to_string())?);

        let mut guard = self.entries.lock().map_err(|e| e.to_string())?;
        guard.retain(|(k, _)| *k != key);
        guard.push_front((key, loaded.clone()));
        guard.truncate(CAPACITY);

        Ok(loaded)
    }
}
