use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use formats::rsc::{self, TextPool};

struct PoolEntry {
    path: PathBuf,
    size: u64,
    mtime_ms: u64,
    pool: Arc<TextPool>,
}

#[derive(Clone)]
pub struct DialogueService {
    pool: Arc<Mutex<Option<PoolEntry>>>,
}

impl Default for DialogueService {
    fn default() -> Self {
        Self::new()
    }
}

impl DialogueService {
    pub fn new() -> Self {
        Self {
            pool: Arc::new(Mutex::new(None)),
        }
    }

    pub fn open(&self, script: &Path) -> Result<rsc::Dialogue, String> {
        let bytes = std::fs::read(script).map_err(|error| error.to_string())?;
        let pool = self.text_pool(script)?;
        rsc::dialogue::parse(&bytes, pool.as_deref()).map_err(|error| error.to_string())
    }

    fn text_pool(&self, script: &Path) -> Result<Option<Arc<TextPool>>, String> {
        let Some(path) = rsc::resolve::locate(script) else {
            return Ok(None);
        };
        let metadata = std::fs::metadata(&path).map_err(|error| error.to_string())?;
        let size = metadata.len();
        let mtime_ms = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|delta| delta.as_millis() as u64)
            .unwrap_or(0);

        {
            let guard = self.pool.lock().map_err(|error| error.to_string())?;
            if let Some(entry) = guard.as_ref()
                && entry.path == path
                && entry.size == size
                && entry.mtime_ms == mtime_ms
            {
                return Ok(Some(entry.pool.clone()));
            }
        }

        let pool = Arc::new(TextPool::load(&path).map_err(|error| error.to_string())?);

        let mut guard = self.pool.lock().map_err(|error| error.to_string())?;
        *guard = Some(PoolEntry {
            path,
            size,
            mtime_ms,
            pool: pool.clone(),
        });
        Ok(Some(pool))
    }
}
