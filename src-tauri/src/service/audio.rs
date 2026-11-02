use std::path::Path;
use std::sync::{Arc, Mutex};

use media::audio::{AudioError, Bundle as AudioBundle, open_acb, open_awb};

const CAPACITY: usize = 2;

#[derive(Default)]
pub struct AudioService {
    entries: Mutex<Vec<(String, Arc<AudioBundle>)>>,
}

impl AudioService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&self, path: &Path) -> Result<Arc<AudioBundle>, AudioError> {
        let key = path.to_string_lossy().to_lowercase();
        if let Ok(mut entries) = self.entries.lock()
            && let Some(position) = entries.iter().position(|(cached, _)| *cached == key)
        {
            let hit = entries.remove(position);
            let bundle = hit.1.clone();
            entries.push(hit);
            return Ok(bundle);
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
