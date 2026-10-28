use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use catalog::{IndexEntry, IndexState, IndexStatusDto};

struct IndexSlot {
    status: IndexStatusDto,
    entries: Arc<Vec<IndexEntry>>,
}

#[derive(Clone)]
pub struct IndexService {
    slot: Arc<Mutex<IndexSlot>>,
    generation: Arc<AtomicU64>,
    cancel: Arc<Mutex<Arc<AtomicBool>>>,
}

impl Default for IndexService {
    fn default() -> Self {
        Self::new()
    }
}

impl IndexService {
    pub fn new() -> Self {
        Self {
            slot: Arc::new(Mutex::new(IndexSlot {
                status: IndexStatusDto {
                    state: IndexState::Idle,
                    count: 0,
                    root: None,
                    error: None,
                },
                entries: Arc::new(Vec::new()),
            })),
            generation: Arc::new(AtomicU64::new(0)),
            cancel: Arc::new(Mutex::new(Arc::new(AtomicBool::new(false)))),
        }
    }

    pub fn status(&self) -> IndexStatusDto {
        match self.slot.lock() {
            Ok(slot) => slot.status.clone(),
            Err(_) => IndexStatusDto {
                state: IndexState::Failed,
                count: 0,
                root: None,
                error: Some("the index state lock is gone".to_string()),
            },
        }
    }

    pub fn entries(&self) -> Arc<Vec<IndexEntry>> {
        match self.slot.lock() {
            Ok(slot) => slot.entries.clone(),
            Err(_) => Arc::new(Vec::new()),
        }
    }

    pub fn start(&self, root: &Path) {
        let generation = self.generation.fetch_add(1, Ordering::Relaxed) + 1;

        let cancel = Arc::new(AtomicBool::new(false));
        if let Ok(mut current) = self.cancel.lock() {
            std::mem::replace(&mut *current, cancel.clone()).store(true, Ordering::Relaxed);
        }

        self.reset_status(root);

        let slot = self.slot.clone();
        let generation_handle = self.generation.clone();
        let root_path = root.to_path_buf();

        std::thread::spawn(move || {
            let progress_slot = slot.clone();
            let result = catalog::build_index(&root_path, &cancel, move |count| {
                if let Ok(mut guard) = progress_slot.lock() {
                    guard.status.count = count;
                }
            });

            if generation_handle.load(Ordering::Relaxed) != generation {
                return;
            }

            let Ok(mut guard) = slot.lock() else { return };
            match result {
                Ok(entries) => {
                    guard.status.state = IndexState::Ready;
                    guard.status.count = entries.len() as u32;
                    guard.status.error = None;
                    guard.entries = Arc::new(entries);
                }
                Err(e) => {
                    guard.status.state = IndexState::Failed;
                    guard.status.error = Some(e.to_string());
                }
            }
        });
    }

    fn reset_status(&self, root: &Path) {
        let Ok(mut slot) = self.slot.lock() else {
            return;
        };
        slot.status = IndexStatusDto {
            state: IndexState::Building,
            count: 0,
            root: Some(catalog::normalize(root)),
            error: None,
        };
        slot.entries = Arc::new(Vec::new());
    }
}
