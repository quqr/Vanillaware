use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::dto::{DirEntryDto, DirListingDto, FileEntryDto, FileMetaDto, RelatedFileDto};
use crate::format;

const MAX_WARNINGS: usize = 50;

#[derive(Debug)]
pub enum FsError {
    NotFound(String),
    NotADirectory(String),
    NotAFile(String),
    NotCatalogued(String),
    Io(String),
}

impl std::fmt::Display for FsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FsError::NotFound(p) => write!(f, "path not found: {p}"),
            FsError::NotADirectory(p) => write!(f, "not a directory: {p}"),
            FsError::NotAFile(p) => write!(f, "not a file: {p}"),
            FsError::NotCatalogued(p) => write!(f, "extension is not in the format registry: {p}"),
            FsError::Io(msg) => write!(f, "filesystem error: {msg}"),
        }
    }
}

impl std::error::Error for FsError {}

pub fn list_dir(dir: &Path) -> Result<DirListingDto, FsError> {
    if !dir.is_dir() {
        return Err(if dir.exists() {
            FsError::NotADirectory(normalize(dir))
        } else {
            FsError::NotFound(normalize(dir))
        });
    }

    let mut dirs: Vec<DirEntryDto> = Vec::new();
    let mut files: Vec<FileEntryDto> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut hidden_files = 0u32;

    let read = std::fs::read_dir(dir).map_err(|e| FsError::Io(e.to_string()))?;
    for entry in read {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                push_warning(&mut warnings, format!("failed to read entry: {e}"));
                continue;
            }
        };

        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(e) => {
                push_warning(
                    &mut warnings,
                    format!("failed to read file type {}: {e}", path.display()),
                );
                continue;
            }
        };

        if file_type.is_dir() {
            dirs.push(DirEntryDto {
                path: normalize(&path),
                name: entry.file_name().to_string_lossy().into_owned(),
            });
            continue;
        }

        if !file_type.is_file() {
            continue;
        }

        let Some(format) = format::lookup(&extension(&path)) else {
            hidden_files += 1;
            continue;
        };

        match entry.metadata() {
            Ok(metadata) => files.push(file_entry(&path, &metadata, format)),
            Err(e) => push_warning(
                &mut warnings,
                format!("failed to read metadata {}: {e}", path.display()),
            ),
        }
    }

    dirs.sort_by_key(|entry| entry.name.to_lowercase());
    files.sort_by_key(|entry| entry.name.to_lowercase());

    Ok(DirListingDto {
        path: normalize(dir),
        parent: dir.parent().map(normalize),
        dirs,
        files,
        hidden_files,
        warnings,
    })
}

pub fn file_meta(path: &Path) -> Result<FileMetaDto, FsError> {
    if !path.exists() {
        return Err(FsError::NotFound(normalize(path)));
    }
    if !path.is_file() {
        return Err(FsError::NotAFile(normalize(path)));
    }
    let Some(format) = format::lookup(&extension(path)) else {
        return Err(FsError::NotCatalogued(normalize(path)));
    };
    let metadata = std::fs::metadata(path).map_err(|e| FsError::Io(e.to_string()))?;
    let entry = file_entry(path, &metadata, format);

    Ok(FileMetaDto {
        related: related_files(path),
        entry,
    })
}

fn file_entry(
    path: &Path,
    metadata: &std::fs::Metadata,
    format: &'static format::Format,
) -> FileEntryDto {
    FileEntryDto {
        path: normalize(path),
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        ext: format.ext.to_string(),
        size: metadata.len(),
        mtime_ms: mtime_ms(metadata),
        kind: format.kind,
        label: format.label,
        icon: format.kind.icon(),
        viewers: format.viewers,
        has_thumb: format.has_thumb,
    }
}

fn related_files(path: &Path) -> Vec<RelatedFileDto> {
    let (Some(parent), Some(stem)) = (path.parent(), path.file_stem()) else {
        return Vec::new();
    };
    let stem = stem.to_string_lossy().to_lowercase();

    let mut out: Vec<RelatedFileDto> = Vec::new();
    let Ok(read) = std::fs::read_dir(parent) else {
        return out;
    };

    for entry in read.flatten() {
        let candidate = entry.path();
        if candidate == path || !candidate.is_file() {
            continue;
        }
        let Some(candidate_stem) = candidate.file_stem() else {
            continue;
        };
        if candidate_stem.to_string_lossy().to_lowercase() != stem {
            continue;
        }
        let Some(format) = format::lookup(&extension(&candidate)) else {
            continue;
        };
        out.push(RelatedFileDto {
            path: normalize(&candidate),
            name: entry.file_name().to_string_lossy().into_owned(),
            ext: format.ext.to_string(),
            kind: format.kind,
            label: format.label,
        });
    }

    out.sort_by(|a, b| a.ext.cmp(&b.ext));
    out
}

pub fn read_head(path: &Path, max_bytes: usize) -> Result<Vec<u8>, FsError> {
    use std::io::Read;

    let file = std::fs::File::open(path).map_err(|e| FsError::Io(e.to_string()))?;
    let mut buffer = Vec::with_capacity(max_bytes.min(64 * 1024));
    file.take(max_bytes as u64)
        .read_to_end(&mut buffer)
        .map_err(|e| FsError::Io(e.to_string()))?;
    Ok(buffer)
}

pub fn extension(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

pub fn normalize(path: &Path) -> String {
    let raw = path.to_string_lossy();
    let stripped = raw
        .strip_prefix(r"\\?\UNC\")
        .map(|rest| format!(r"\\{rest}"))
        .unwrap_or_else(|| raw.strip_prefix(r"\\?\").unwrap_or(&raw).to_string());

    stripped.replace('/', "\\")
}

pub fn mtime_ms(metadata: &std::fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn absolutize(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    let Ok(cwd) = std::env::current_dir() else {
        return path.to_path_buf();
    };
    let mut out = cwd;
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn push_warning(warnings: &mut Vec<String>, message: String) {
    if warnings.len() < MAX_WARNINGS {
        warnings.push(message);
    }
}
