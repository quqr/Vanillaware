use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use walkdir::WalkDir;

use crate::dto::{IndexedFileDto, SearchResultDto};
use crate::format::{self, Format};
use crate::fslist::{self, FsError};

pub const SEARCH_LIMIT: usize = 500;

const PROGRESS_INTERVAL: u32 = 256;

#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub path: String,
    pub name: String,
    needle: String,
    pub format: &'static Format,
}

impl IndexEntry {
    pub fn new(path: String, name: String, format: &'static Format) -> Self {
        Self {
            path,
            needle: name.to_lowercase(),
            name,
            format,
        }
    }

    fn to_dto(&self) -> IndexedFileDto {
        IndexedFileDto {
            path: self.path.clone(),
            name: self.name.clone(),
            ext: self.format.ext.to_string(),
            kind: self.format.kind,
            label: self.format.label,
            icon: self.format.kind.icon(),
            viewers: self.format.viewers,
        }
    }
}

pub fn build(
    root: &Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u32),
) -> Result<Vec<IndexEntry>, FsError> {
    if !root.is_dir() {
        return Err(if root.exists() {
            FsError::NotADirectory(fslist::normalize(root))
        } else {
            FsError::NotFound(fslist::normalize(root))
        });
    }

    let mut entries: Vec<IndexEntry> = Vec::new();
    let mut since_report = 0u32;

    let walker = WalkDir::new(root).follow_links(false);
    for entry in walker.into_iter().flatten() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let Some(format) = format::lookup(&fslist::extension(path)) else {
            continue;
        };
        let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else {
            continue;
        };

        entries.push(IndexEntry::new(fslist::normalize(path), name, format));

        since_report += 1;
        if since_report >= PROGRESS_INTERVAL {
            since_report = 0;
            progress(entries.len() as u32);
        }
    }

    progress(entries.len() as u32);
    Ok(entries)
}

pub fn search(entries: &[IndexEntry], query: &str, limit: usize) -> SearchResultDto {
    let needle = query.trim().to_lowercase();
    let limit = limit.max(1);

    if needle.is_empty() {
        return SearchResultDto {
            items: Vec::new(),
            truncated: false,
            total: 0,
        };
    }

    let mut items: Vec<IndexedFileDto> = Vec::new();
    let mut total = 0u32;

    for entry in entries {
        if !entry.needle.contains(&needle) {
            continue;
        }
        total += 1;
        if items.len() < limit {
            items.push(entry.to_dto());
        }
    }

    SearchResultDto {
        truncated: total as usize > items.len(),
        total,
        items,
    }
}
