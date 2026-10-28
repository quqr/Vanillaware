//! 目录索引服务：后台构建、状态查询、条目快照。
//!
//! 索引只存元信息（路径/名称/类别），只服务全局搜索；目录列表永远来自实时 `read_dir`。
//! 换根即整份重建，旧任务被取消且结果作废。

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use catalog::{IndexEntry, IndexState, IndexStatusDto};

/// 索引的可变部分：状态 + 条目快照。
struct IndexSlot {
    status: IndexStatusDto,
    /// 整体替换而非原地改，读侧克隆 `Arc` 即可无锁遍历。
    entries: Arc<Vec<IndexEntry>>,
}

/// 可克隆句柄：后台线程与命令层共用同一个索引。
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
                error: Some("索引状态锁已失效".to_string()),
            },
        }
    }

    /// 当前快照。构建中返回已收集的部分，前端据此提示「结果可能不完整」。
    pub fn entries(&self) -> Arc<Vec<IndexEntry>> {
        match self.slot.lock() {
            Ok(slot) => slot.entries.clone(),
            Err(_) => Arc::new(Vec::new()),
        }
    }

    /// 换根重建。旧任务会被取消，其结果不会覆盖新任务。
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
