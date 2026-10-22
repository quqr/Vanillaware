//! 应用级共享状态：缩略图缓存、解码闸门、内存索引。

use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::Semaphore;

use vw_core::{IndexEntry, IndexState, IndexStatusDto};
use vw_media::ThumbCache;

use crate::scene::{self, LoadedMbs};

/// 解析缓存的键：文件被改动（size/mtime 变）即视为不同条目。
#[derive(PartialEq, Eq)]
struct ParseKey {
    path: String,
    tag: String,
    size: u64,
    mtime_ms: u64,
}

/// 解析缓存容量。一份 Quad 约几 MB～十几 MB，4 份足够覆盖「来回切两个文件」。
const PARSE_CACHE_CAPACITY: usize = 4;

/// 缓存条目表：`(键, 解析结果)` 的先进先出队列。
type ParseEntries = Arc<Mutex<VecDeque<(ParseKey, Arc<LoadedMbs>)>>>;

/// 解析缓存的可克隆句柄：可安全移进 `spawn_blocking`。
#[derive(Clone)]
pub struct ParseCache {
    entries: ParseEntries,
}

impl ParseCache {
    fn new() -> Self {
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
            let guard = self.entries.lock().unwrap();
            if let Some((_, loaded)) = guard.iter().find(|(k, _)| *k == key) {
                return Ok(loaded.clone());
            }
        }

        // 解析在锁外进行：同一文件并发请求会各解析一次，但不会阻塞其他文件。
        let loaded = Arc::new(scene::load(path, tag).map_err(|e| e.to_string())?);

        let mut guard = self.entries.lock().unwrap();
        guard.retain(|(k, _)| *k != key);
        guard.push_front((key, loaded.clone()));
        guard.truncate(PARSE_CACHE_CAPACITY);

        Ok(loaded)
    }
}

/// 索引的可变部分：状态 + 条目快照。
pub struct IndexSlot {
    pub status: IndexStatusDto,
    /// 整体替换而非原地改，读侧克隆 `Arc` 即可无锁遍历。
    pub entries: Arc<Vec<IndexEntry>>,
}

pub struct AppState {
    pub cache: Arc<ThumbCache>,
    /// 同时解码的帧数上限：列表滚动时避免内存峰值。
    pub decode_sem: Arc<Semaphore>,
    /// 同一文件的并发请求合并（probe 与解码共用同一把锁，避免写坏 meta）。
    pub inflight: Arc<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,

    index: Arc<Mutex<IndexSlot>>,
    /// 索引任务代际：每次换根 +1，旧任务的完成结果直接丢弃。
    index_generation: Arc<AtomicU64>,
    /// 当前索引任务的取消位。
    index_cancel: Mutex<Arc<AtomicBool>>,
    /// 最近解析过的 `.mbs`（play 与 structure 共用同一份）。
    parsed: ParseCache,
}

impl AppState {
    pub fn new(cache_root: std::path::PathBuf) -> Self {
        Self {
            cache: Arc::new(ThumbCache::new(cache_root)),
            decode_sem: Arc::new(Semaphore::new(3)),
            inflight: Arc::new(Mutex::new(HashMap::new())),
            index: Arc::new(Mutex::new(IndexSlot {
                status: IndexStatusDto {
                    state: IndexState::Idle,
                    count: 0,
                    root: None,
                    error: None,
                },
                entries: Arc::new(Vec::new()),
            })),
            index_generation: Arc::new(AtomicU64::new(0)),
            index_cancel: Mutex::new(Arc::new(AtomicBool::new(false))),
            parsed: ParseCache::new(),
        }
    }

    /// 解析缓存句柄（可移进阻塞线程）。
    pub fn parse_cache(&self) -> ParseCache {
        self.parsed.clone()
    }

    pub fn index_status(&self) -> IndexStatusDto {
        self.index.lock().unwrap().status.clone()
    }

    /// 当前索引快照。构建中返回已收集的部分，前端据此提示「结果可能不完整」。
    pub fn index_entries(&self) -> Arc<Vec<IndexEntry>> {
        self.index.lock().unwrap().entries.clone()
    }

    /// 换根重建索引。旧任务会被取消，其结果不会覆盖新任务。
    pub fn start_index(&self, root: &Path) {
        let generation = self.index_generation.fetch_add(1, Ordering::Relaxed) + 1;

        let cancel = Arc::new(AtomicBool::new(false));
        {
            let previous = std::mem::replace(&mut *self.index_cancel.lock().unwrap(), cancel.clone());
            previous.store(true, Ordering::Relaxed);
        }

        {
            let mut slot = self.index.lock().unwrap();
            slot.status = IndexStatusDto {
                state: IndexState::Building,
                count: 0,
                root: Some(vw_core::normalize(root)),
                error: None,
            };
            slot.entries = Arc::new(Vec::new());
        }

        let slot = self.index.clone();
        let generation_handle = self.index_generation.clone();
        let root_path = root.to_path_buf();

        std::thread::spawn(move || {
            let progress_slot = slot.clone();
            let result = vw_core::build_index(&root_path, &cancel, move |count| {
                if let Ok(mut guard) = progress_slot.lock() {
                    guard.status.count = count;
                }
            });

            // 期间换过根：本次结果作废。
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

    /// 取该文件的去重锁（不存在则创建）。
    pub fn inflight_lock(&self, key: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut guard = self.inflight.lock().unwrap();
        guard
            .entry(key.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }
}
