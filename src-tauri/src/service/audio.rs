//! 音频容器索引缓存。
//!
//! 解析一次 ACB 表（`voice_*.acb` 一万三千条 cue）要几十到二百毫秒，
//! 换一条轨道就重解析不合算。缓存键是路径，容量 2：
//! 内嵌 AWB 的字节会跟着索引一起驻留（单个最大 27 MB），所以不放太大。

use std::path::Path;
use std::sync::{Arc, Mutex};

use vw_media::audio::{AudioError, Bundle as AudioBundle, open_acb, open_awb};

/// 同时驻留的容器数。
const CAPACITY: usize = 2;

#[derive(Default)]
pub struct AudioService {
    entries: Mutex<Vec<(String, Arc<AudioBundle>)>>,
}

impl AudioService {
    pub fn new() -> Self {
        Self::default()
    }

    /// 取容器索引；命中缓存直接返回。
    pub fn open(&self, path: &Path) -> Result<Arc<AudioBundle>, AudioError> {
        let key = path.to_string_lossy().to_lowercase();
        if let Ok(mut entries) = self.entries.lock() {
            if let Some(position) = entries.iter().position(|(cached, _)| *cached == key) {
                let hit = entries.remove(position);
                let bundle = hit.1.clone();
                entries.push(hit);
                return Ok(bundle);
            }
        }

        let bundle = Arc::new(if is_awb(path) {
            open_awb(path)?
        } else {
            open_acb(path)?
        });

        if let Ok(mut entries) = self.entries.lock() {
            entries.retain(|(cached, _)| *cached != key);
            entries.push((key, bundle.clone()));
            while entries.len() > CAPACITY {
                entries.remove(0);
            }
        }
        Ok(bundle)
    }

    /// 丢弃所有缓存（切换游戏档案时用）。
    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.clear();
        }
    }
}

fn is_awb(path: &Path) -> bool {
    path.extension()
        .map(|ext| ext.eq_ignore_ascii_case("awb"))
        .unwrap_or(false)
}
