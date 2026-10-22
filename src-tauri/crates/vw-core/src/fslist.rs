//! 目录读取与文件信息：库视图的唯一数据来源。
//!
//! 只做文件系统访问与**注册表过滤**，不解码任何内容——想在这里解码都做不到（本 crate 不依赖
//! `vw-formats`）。被过滤掉的文件只报数量，不报明细：库只服务原生二进制资源。

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
            FsError::NotFound(p) => write!(f, "路径不存在: {p}"),
            FsError::NotADirectory(p) => write!(f, "不是目录: {p}"),
            FsError::NotAFile(p) => write!(f, "不是文件: {p}"),
            FsError::NotCatalogued(p) => write!(f, "该扩展名不在格式注册表里: {p}"),
            FsError::Io(msg) => write!(f, "文件系统错误: {msg}"),
        }
    }
}

impl std::error::Error for FsError {}

/// 读一层目录。子目录全部返回（含空目录），文件只返回注册表收录的。
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
                push_warning(&mut warnings, format!("读取条目失败: {e}"));
                continue;
            }
        };

        let path = entry.path();
        // 符号链接按真实类型判断，不跟随。
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(e) => {
                push_warning(
                    &mut warnings,
                    format!("读取类型失败 {}: {e}", path.display()),
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
                format!("读取元信息失败 {}: {e}", path.display()),
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

/// 单个已收录文件的完整信息，含同目录同基础名的关联文件。
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

/// 构造已收录文件的条目；字段全部来自注册表 + 文件元信息。
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
        viewers: format.kind.viewers(),
        has_thumb: format.kind == format::Kind::Texture,
    }
}

/// 同目录、同基础名、不同扩展名的已收录文件。
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

/// 读文件头部若干字节（十六进制查看器用）。不足则返回实际长度。
pub fn read_head(path: &Path, max_bytes: usize) -> Result<Vec<u8>, FsError> {
    use std::io::Read;

    let file = std::fs::File::open(path).map_err(|e| FsError::Io(e.to_string()))?;
    let mut buffer = Vec::with_capacity(max_bytes.min(64 * 1024));
    file.take(max_bytes as u64)
        .read_to_end(&mut buffer)
        .map_err(|e| FsError::Io(e.to_string()))?;
    Ok(buffer)
}

/// 小写、不含点的扩展名；无扩展名时为空串。
pub fn extension(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

/// 去掉 Windows 的 `\\?\` / `\\?\UNC\` 前缀，统一成普通显示路径。
pub fn normalize(path: &Path) -> String {
    let raw = path.to_string_lossy();
    let stripped = raw
        .strip_prefix(r"\\?\UNC\")
        .map(|rest| format!(r"\\{rest}"))
        .unwrap_or_else(|| raw.strip_prefix(r"\\?\").unwrap_or(&raw).to_string());

    stripped.replace('/', "\\")
}

/// Unix 毫秒；取不到时为 0。
pub fn mtime_ms(metadata: &std::fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 把路径拼成绝对路径但不做 `canonicalize`（后者会带 `\\?\` 前缀）。
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
