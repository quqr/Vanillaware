//! 纯内存目录索引：只收集**已收录资源**的元信息，服务全局搜索。
//!
//! 不读文件内容、不做解码、不落盘。换根即整份丢弃重建，因此没有版本号与失效判定，
//! 也不会与缩略图磁盘缓存形成第二套失效机制。

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use walkdir::WalkDir;

use crate::dto::{IndexedFileDto, SearchResultDto};
use crate::format::{self, Kind};
use crate::fslist::{self, FsError};

/// 搜索结果的默认上限。
pub const SEARCH_LIMIT: usize = 500;

/// 进度回调的节流间隔（条目数）。
const PROGRESS_INTERVAL: u32 = 256;

/// 索引里的一条记录。
#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub path: String,
    pub name: String,
    needle: String,
    pub kind: Kind,
    pub label: &'static str,
}

impl IndexEntry {
    fn to_dto(&self) -> IndexedFileDto {
        IndexedFileDto {
            path: self.path.clone(),
            name: self.name.clone(),
            kind: self.kind,
            label: self.label,
            viewers: self.kind.viewers(),
        }
    }
}

/// 递归收集 `root` 下所有已收录资源。`cancel` 置位则提前返回已收集部分。
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

    // 不排序：索引只服务搜索，排序会让每个目录都先读完再遍历。
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

        entries.push(IndexEntry {
            path: fslist::normalize(path),
            needle: name.to_lowercase(),
            name,
            kind: format.kind,
            label: format.label,
        });

        since_report += 1;
        if since_report >= PROGRESS_INTERVAL {
            since_report = 0;
            progress(entries.len() as u32);
        }
    }

    progress(entries.len() as u32);
    Ok(entries)
}

/// 按文件名子串搜索（大小写不敏感），`limit` 为返回条数上限。
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

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str) -> IndexEntry {
        IndexEntry {
            path: format!("C:\\x\\{name}"),
            name: name.to_string(),
            needle: name.to_lowercase(),
            kind: Kind::Binary,
            label: "ASB",
        }
    }

    #[test]
    fn search_is_case_insensitive_and_reports_truncation() {
        let entries = vec![entry("Alpha.asb"), entry("alpha2.asb"), entry("Beta.ftx")];
        let hit = search(&entries, "ALPHA", 10);
        assert_eq!(hit.total, 2);
        assert!(!hit.truncated);

        let truncated = search(&entries, "alpha", 1);
        assert_eq!(truncated.items.len(), 1);
        assert_eq!(truncated.total, 2);
        assert!(truncated.truncated);
    }

    #[test]
    fn empty_query_matches_nothing() {
        let entries = vec![entry("Alpha.asb")];
        assert_eq!(search(&entries, "   ", 10).total, 0);
    }
}
