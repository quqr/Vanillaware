//! 应用级共享状态：把服务与资源句柄装在一起，交给 Tauri 托管。
//!
//! 这里只做「持有 + 转发」，业务逻辑在 [`crate::service`]。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tokio::sync::Semaphore;

use vw_core::{IndexEntry, IndexStatusDto};
use vw_media::ThumbCache;

use crate::config;
use crate::service::{AudioService, DocCache, IndexService, ParseCache, VideoService};

pub struct AppState {
    /// 缩略图与帧图的磁盘缓存。
    pub cache: Arc<ThumbCache>,
    /// 解码并发闸门。
    pub decode_sem: Arc<Semaphore>,
    /// 同一文件的并发请求合并（probe 与解码共用同一把锁，避免写坏 meta）。
    pub inflight: Arc<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
    /// 文档树构建的并发闸门（与像素解码隔离）。
    pub doc_sem: Arc<Semaphore>,
    /// 音频容器索引（ACB/AWB）。
    pub audio: Arc<AudioService>,
    /// USM 影片索引。
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
            video: Arc::new(VideoService::new()),
            index: IndexService::new(),
            parse: ParseCache::new(),
            doc: DocCache::new(),
        }
    }

    /// 解析缓存句柄（可移进阻塞线程）。
    pub fn parse_cache(&self) -> ParseCache {
        self.parse.clone()
    }

    /// 文档树缓存句柄。
    pub fn doc_cache(&self) -> DocCache {
        self.doc.clone()
    }

    pub fn index_status(&self) -> IndexStatusDto {
        self.index.status()
    }

    /// 当前索引快照。构建中返回已收集的部分。
    pub fn index_entries(&self) -> Arc<Vec<IndexEntry>> {
        self.index.entries()
    }

    /// 换根重建索引。
    pub fn start_index(&self, root: &Path) {
        self.index.start(root);
    }

    /// 取该文件的去重锁（不存在则创建）。
    pub fn inflight_lock(&self, key: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut guard = self.inflight.lock().unwrap();
        guard
            .entry(key.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }
}
