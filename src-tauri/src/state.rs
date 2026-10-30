use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tokio::sync::Semaphore;

use catalog::{IndexEntry, IndexStatusDto};
use media::ThumbCache;

use crate::config;
use crate::service::{
    AudioService, DialogueService, DocCache, IndexService, ParseCache, VideoService,
};

pub struct AppState {
    pub cache: Arc<ThumbCache>,
    pub decode_sem: Arc<Semaphore>,
    pub inflight: Arc<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
    pub doc_sem: Arc<Semaphore>,
    pub audio: Arc<AudioService>,
    pub dialogue: Arc<DialogueService>,
    pub video: Arc<VideoService>,
    index: IndexService,
    parse: ParseCache,
    doc: DocCache,
}

impl AppState {
    pub fn new(cache_root: PathBuf) -> Self {
        Self {
            cache: Arc::new(ThumbCache::new(cache_root)),
            decode_sem: Arc::new(Semaphore::new(config::DECODE_PERMITS)),
            doc_sem: Arc::new(Semaphore::new(config::DOC_PERMITS)),
            inflight: Arc::new(Mutex::new(HashMap::new())),
            audio: Arc::new(AudioService::new()),
            dialogue: Arc::new(DialogueService::new()),
            video: Arc::new(VideoService::new()),
            index: IndexService::new(),
            parse: ParseCache::new(),
            doc: DocCache::new(),
        }
    }

    pub fn parse_cache(&self) -> ParseCache {
        self.parse.clone()
    }

    pub fn doc_cache(&self) -> DocCache {
        self.doc.clone()
    }

    pub fn index_status(&self) -> IndexStatusDto {
        self.index.status()
    }

    pub fn index_entries(&self) -> Arc<Vec<IndexEntry>> {
        self.index.entries()
    }

    pub fn start_index(&self, root: &Path) {
        self.index.start(root);
    }

    pub fn inflight_lock(&self, key: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut guard = self.inflight.lock().unwrap();
        guard
            .entry(key.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }
}
