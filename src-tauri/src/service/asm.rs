use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};

use formats::document::asb::AsbModule;

const CAPACITY: usize = 8;

#[derive(Clone, PartialEq, Eq)]
struct AsmKey {
    path: String,
    size: u64,
    mtime_ms: u64,
}

struct Entry {
    key: AsmKey,
    module: Arc<AsbModule>,
}

/// Keep one ASB parse alive so switching functions never re-reads or re-parses the module.
#[derive(Clone, Default)]
pub struct AsmCache {
    entries: Arc<Mutex<VecDeque<Entry>>>,
}

impl AsmCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, path: &str, size: u64, mtime_ms: u64) -> Option<Arc<AsbModule>> {
        let key = key(path, size, mtime_ms);
        let guard = self.entries.lock().ok()?;
        guard
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.module.clone())
    }

    pub fn insert(&self, path: &str, size: u64, mtime_ms: u64, module: Arc<AsbModule>) {
        let key = key(path, size, mtime_ms);
        let Ok(mut guard) = self.entries.lock() else {
            return;
        };
        guard.retain(|entry| entry.key != key);
        guard.push_front(Entry { key, module });
        while guard.len() > CAPACITY {
            guard.pop_back();
        }
    }
}

fn key(path: &str, size: u64, mtime_ms: u64) -> AsmKey {
    AsmKey {
        path: catalog::normalize(Path::new(path)).to_lowercase(),
        size,
        mtime_ms,
    }
}
