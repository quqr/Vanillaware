//! USM 索引缓存。
//!
//! 建索引要顺序扫一遍整条码流（76 MB 约 1 秒，两百 MB 约两三秒），
//! 所以只留最近一个文件；换文件就重扫。

use std::path::Path;
use std::sync::{Arc, Mutex};

use media::video::{UsmIndex, open};

#[derive(Default)]
pub struct VideoService {
    last: Mutex<Option<(String, Arc<UsmIndex>)>>,
}

impl VideoService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&self, path: &Path) -> Result<Arc<UsmIndex>, String> {
        let key = path.to_string_lossy().to_lowercase();
        if let Ok(guard) = self.last.lock() {
            if let Some((cached, index)) = guard.as_ref() {
                if *cached == key {
                    return Ok(index.clone());
                }
            }
        }
        let index = Arc::new(open(path).map_err(|error| error.to_string())?);
        if let Ok(mut guard) = self.last.lock() {
            *guard = Some((key, index.clone()));
        }
        Ok(index)
    }
}
