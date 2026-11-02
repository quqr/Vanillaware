use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

use archive::CpkArchive;
use catalog::dto::{DirEntryDto, DirListingDto, FileEntryDto};
use catalog::{format, fslist, IndexEntry, Kind};

pub const ARCHIVE_SPILL_DEFAULT: u64 = 32 * 1024 * 1024;

const BYTE_BUDGET_DEFAULT: usize = 64 * 1024 * 1024;
const ARCHIVE_CAPACITY_DEFAULT: usize = 4;

#[derive(Debug)]
pub enum Error {
    Fs(fslist::FsError),
    Archive(archive::Error),
    ArchiveAt {
        path: String,
        source: archive::Error,
    },
    NotFound(String),
    NotADirectory(String),
    Io(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Fs(error) => write!(f, "{error}"),
            Error::Archive(error) => write!(f, "{error}"),
            Error::ArchiveAt { path, source } => write!(f, "{source} ({path})"),
            Error::NotFound(path) => write!(f, "path not found: {path}"),
            Error::NotADirectory(path) => write!(f, "not a directory: {path}"),
            Error::Io(message) => write!(f, "filesystem error: {message}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<fslist::FsError> for Error {
    fn from(error: fslist::FsError) -> Self {
        Error::Fs(error)
    }
}

impl From<archive::Error> for Error {
    fn from(error: archive::Error) -> Self {
        Error::Archive(error)
    }
}

#[derive(Debug, Clone)]
pub enum Split<'a> {
    Disk(&'a str),
    Archive { cpk: &'a str, inner: &'a str },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stat {
    pub size: u64,
    pub mtime_ms: u64,
    pub is_dir: bool,
}

pub enum Target {
    Disk(PathBuf),
    Archived {
        cpk: String,
        inner: String,
        archive: Arc<CpkArchive>,
    },
}

pub fn split(raw: &str) -> Split<'_> {
    let text = raw.trim_end_matches(['\\', '/']);
    let bytes = text.as_bytes();
    let mut component_start = 0usize;

    for index in 0..=bytes.len() {
        let at_end = index == bytes.len();
        if !at_end && bytes[index] != b'\\' && bytes[index] != b'/' {
            continue;
        }

        let component = &text[component_start..index];
        let prefix = &text[..index];
        if is_archive_name(component) && Path::new(prefix).is_file() {
            let inner = if at_end {
                ""
            } else {
                text[index..].trim_start_matches(['\\', '/'])
            };
            return Split::Archive { cpk: prefix, inner };
        }

        if at_end {
            break;
        }
        component_start = index + 1;
    }

    Split::Disk(text)
}

fn is_archive_name(component: &str) -> bool {
    let Some((_, ext)) = component.rsplit_once('.') else {
        return false;
    };
    format::lookup(ext)
        .map(|found| found.kind == Kind::Archive)
        .unwrap_or(false)
}

struct Cached {
    key: Key,
    archive: Arc<CpkArchive>,
}

#[derive(Clone, PartialEq, Eq)]
struct Key {
    path: String,
    size: u64,
    mtime_ms: u64,
}

struct ByteCache {
    budget: usize,
    used: usize,
    order: Vec<String>,
    map: HashMap<String, Arc<Vec<u8>>>,
}

impl ByteCache {
    fn new(budget: usize) -> Self {
        Self {
            budget,
            used: 0,
            order: Vec::new(),
            map: HashMap::new(),
        }
    }

    fn get(&self, key: &str) -> Option<Arc<Vec<u8>>> {
        self.map.get(key).cloned()
    }

    fn put(&mut self, key: String, value: Arc<Vec<u8>>) {
        if value.len() > self.budget {
            return;
        }
        if self.map.contains_key(&key) {
            return;
        }
        while self.used + value.len() > self.budget {
            let Some(evicted) = self.order.first().cloned() else {
                break;
            };
            self.order.remove(0);
            if let Some(previous) = self.map.remove(&evicted) {
                self.used -= previous.len();
            }
        }
        self.used += value.len();
        self.order.push(key.clone());
        self.map.insert(key, value);
    }

    fn clear(&mut self) {
        self.order.clear();
        self.map.clear();
        self.used = 0;
    }
}

pub trait ReadSeek: Read + Seek {}

impl<T: Read + Seek> ReadSeek for T {}

pub struct Mount {
    archives: Mutex<Vec<Cached>>,
    bytes: Mutex<ByteCache>,
    spill: Option<PathBuf>,
    spill_threshold: u64,
    capacity: usize,
}

impl Mount {
    pub fn new(spill: Option<PathBuf>) -> Self {
        Self {
            archives: Mutex::new(Vec::new()),
            bytes: Mutex::new(ByteCache::new(BYTE_BUDGET_DEFAULT)),
            spill,
            spill_threshold: ARCHIVE_SPILL_DEFAULT,
            capacity: ARCHIVE_CAPACITY_DEFAULT,
        }
    }

    pub fn target(&self, raw: &str) -> Result<Target, Error> {
        match split(raw) {
            Split::Disk(path) => Ok(Target::Disk(PathBuf::from(path))),
            Split::Archive { cpk, inner } => Ok(Target::Archived {
                cpk: cpk.to_string(),
                inner: inner.replace('\\', "/"),
                archive: self.archive(cpk)?,
            }),
        }
    }

    pub fn archive(&self, cpk: &str) -> Result<Arc<CpkArchive>, Error> {
        let metadata = std::fs::metadata(cpk).map_err(|_| Error::NotFound(cpk.to_string()))?;
        let key = Key {
            path: cpk.to_string(),
            size: metadata.len(),
            mtime_ms: mtime_ms(&metadata),
        };

        if let Ok(cache) = self.archives.lock() {
            if let Some(hit) = cache.iter().find(|cached| cached.key == key) {
                return Ok(hit.archive.clone());
            }
        }

        let archive =
            Arc::new(
                CpkArchive::open(Path::new(cpk)).map_err(|source| Error::ArchiveAt {
                    path: cpk.to_string(),
                    source,
                })?,
            );
        if let Ok(mut cache) = self.archives.lock() {
            cache.retain(|cached| cached.key != key);
            cache.insert(
                0,
                Cached {
                    key,
                    archive: archive.clone(),
                },
            );
            cache.truncate(self.capacity);
        }
        Ok(archive)
    }

    pub fn list(&self, raw: &str) -> Result<DirListingDto, Error> {
        match self.target(raw)? {
            Target::Disk(path) => self.list_disk(&path),
            Target::Archived {
                cpk,
                inner,
                archive,
            } => Ok(self.list_archive(&cpk, &inner, &archive)),
        }
    }

    fn list_disk(&self, path: &Path) -> Result<DirListingDto, Error> {
        let mut listing = catalog::list_dir(path)?;
        let mut promoted: Vec<DirEntryDto> = Vec::new();

        listing.files.retain(|file| {
            if file.kind == Kind::Archive {
                promoted.push(DirEntryDto {
                    path: file.path.clone(),
                    name: file.name.clone(),
                    kind: Some(Kind::Archive),
                });
                return false;
            }
            true
        });

        if !promoted.is_empty() {
            listing.dirs.append(&mut promoted);
            listing.dirs.sort_by_key(|entry| entry.name.to_lowercase());
        }

        Ok(listing)
    }

    fn list_archive(&self, cpk: &str, inner: &str, archive: &CpkArchive) -> DirListingDto {
        let stamp = cpk_mtime_ms(cpk);

        let dirs: Vec<DirEntryDto> = archive
            .child_dirs(inner)
            .into_iter()
            .map(|name| DirEntryDto {
                path: display_path(cpk, inner, &name),
                name,
                kind: None,
            })
            .collect();

        let mut hidden_files = 0u32;
        let mut files: Vec<FileEntryDto> = Vec::new();
        for index in archive.child_entries(inner) {
            let entry = &archive.entries()[index];
            let ext = extension_of(&entry.name);
            let Some(found) = format::lookup(&ext) else {
                hidden_files += 1;
                continue;
            };
            files.push(FileEntryDto {
                path: display_path(cpk, inner, &entry.name),
                name: entry.name.clone(),
                ext: found.ext.to_string(),
                size: entry.extract_size,
                mtime_ms: stamp,
                kind: found.kind,
                label: found.label,
                icon: found.kind.icon(),
                viewers: found.viewers,
                has_thumb: found.has_thumb,
            });
        }
        files.sort_by_key(|file| file.name.to_lowercase());

        DirListingDto {
            path: display_self(cpk, inner),
            parent: parent_of(cpk, inner),
            dirs,
            files,
            hidden_files,
            warnings: Vec::new(),
        }
    }

    pub fn stat(&self, raw: &str) -> Result<Stat, Error> {
        match self.target(raw)? {
            Target::Disk(path) => {
                let metadata =
                    std::fs::metadata(&path).map_err(|_| Error::NotFound(raw.to_string()))?;
                Ok(Stat {
                    size: metadata.len(),
                    mtime_ms: mtime_ms(&metadata),
                    is_dir: metadata.is_dir(),
                })
            }
            Target::Archived {
                cpk,
                inner,
                archive,
            } => {
                let stamp = cpk_mtime_ms(&cpk);
                if inner.is_empty() {
                    let size = std::fs::metadata(&cpk).map(|meta| meta.len()).unwrap_or(0);
                    return Ok(Stat {
                        size,
                        mtime_ms: stamp,
                        is_dir: true,
                    });
                }
                if let Some(index) = archive.find(&inner) {
                    return Ok(Stat {
                        size: archive.entries()[index].extract_size,
                        mtime_ms: stamp,
                        is_dir: false,
                    });
                }
                if archive.has_dir(&inner) {
                    return Ok(Stat {
                        size: 0,
                        mtime_ms: stamp,
                        is_dir: true,
                    });
                }
                Err(Error::NotFound(raw.to_string()))
            }
        }
    }

    pub fn read(&self, raw: &str) -> Result<Vec<u8>, Error> {
        match self.target(raw)? {
            Target::Disk(path) => {
                std::fs::read(&path).map_err(|error| Error::Io(error.to_string()))
            }
            Target::Archived {
                cpk,
                inner,
                archive,
            } => {
                if inner.is_empty() {
                    return std::fs::read(&cpk).map_err(|error| Error::Io(error.to_string()));
                }
                let index = archive
                    .find(&inner)
                    .ok_or_else(|| Error::NotFound(raw.to_string()))?;
                Ok((*self.archive_bytes(&cpk, &archive, index)?).clone())
            }
        }
    }

    pub fn read_head(&self, raw: &str, max: usize) -> Result<Vec<u8>, Error> {
        let mut data = self.read(raw)?;
        data.truncate(max.clamp(1, 64 * 1024));
        Ok(data)
    }

    pub fn open(&self, raw: &str) -> Result<Box<dyn ReadSeek + Send>, Error> {
        match self.target(raw)? {
            Target::Disk(path) => {
                let file = File::open(&path).map_err(|error| Error::Io(error.to_string()))?;
                Ok(Box::new(BufReader::new(file)))
            }
            Target::Archived {
                cpk,
                inner,
                archive,
            } => {
                if inner.is_empty() {
                    let file = File::open(&cpk).map_err(|error| Error::Io(error.to_string()))?;
                    return Ok(Box::new(BufReader::new(file)));
                }
                let index = archive
                    .find(&inner)
                    .ok_or_else(|| Error::NotFound(raw.to_string()))?;

                if self.wants_spill(index, &archive) {
                    let path = self.materialize_entry(&cpk, &archive, index)?;
                    let file = File::open(&path).map_err(|error| Error::Io(error.to_string()))?;
                    return Ok(Box::new(BufReader::new(file)));
                }

                let bytes = self.archive_bytes(&cpk, &archive, index)?;
                Ok(Box::new(Cursor::new((*bytes).clone())))
            }
        }
    }

    pub fn peers(&self, raw: &str) -> Vec<String> {
        let (directory, _) = split_directory(raw);
        match split(raw) {
            Split::Disk(_) => std::fs::read_dir(Path::new(&directory))
                .map(|entries| {
                    entries
                        .flatten()
                        .filter(|entry| {
                            entry
                                .file_type()
                                .map(|kind| kind.is_file())
                                .unwrap_or(false)
                        })
                        .filter_map(|entry| entry.file_name().into_string().ok())
                        .collect()
                })
                .unwrap_or_default(),
            Split::Archive { cpk, inner } => {
                let parent = parent_inner(inner);
                let Ok(archive) = self.archive(cpk) else {
                    return Vec::new();
                };
                archive
                    .child_entries(&parent)
                    .into_iter()
                    .map(|index| archive.entries()[index].name.clone())
                    .collect()
            }
        }
    }

    pub fn walk_names(&self, raw: &str, depth: u8) -> Vec<String> {
        match split(raw) {
            Split::Disk(path) => {
                let root = Path::new(path);
                if !root.is_dir() {
                    return Vec::new();
                }
                walkdir::WalkDir::new(root)
                    .max_depth(depth as usize + 1)
                    .follow_links(false)
                    .into_iter()
                    .flatten()
                    .filter(|entry| entry.file_type().is_file())
                    .filter_map(|entry| entry.file_name().to_str().map(|name| name.to_string()))
                    .collect()
            }
            Split::Archive { cpk, inner } => {
                let Ok(archive) = self.archive(cpk) else {
                    return Vec::new();
                };
                let base = inner.trim_matches(['\\', '/']).replace('\\', "/");
                let allowed = depth as usize + 1;
                archive
                    .entries()
                    .iter()
                    .filter(|entry| {
                        if base.is_empty() {
                            return entry.dir.matches('/').count() < allowed;
                        }
                        let nested = format!("{base}/");
                        entry.dir.starts_with(&nested)
                            && entry.dir[base.len() + 1..].matches('/').count() < allowed
                    })
                    .map(|entry| entry.name.clone())
                    .collect()
            }
        }
    }

    pub fn real(&self, raw: &str) -> Result<PathBuf, Error> {
        match self.target(raw)? {
            Target::Disk(path) => Ok(path),
            Target::Archived {
                cpk,
                inner,
                archive,
            } => {
                if inner.is_empty() {
                    return Ok(PathBuf::from(cpk));
                }
                if let Some(index) = archive.find(&inner) {
                    return self.materialize_family(&cpk, &archive, index);
                }
                if archive.has_dir(&inner) {
                    return self.materialize_tree(&cpk, &archive, &inner);
                }
                Err(Error::NotFound(raw.to_string()))
            }
        }
    }

    pub fn file_meta(&self, raw: &str) -> Result<catalog::FileMetaDto, Error> {
        match self.target(raw)? {
            Target::Disk(path) => catalog::file_meta(&path).map_err(Error::Fs),
            Target::Archived { cpk, inner, .. } if inner.is_empty() => {
                let found =
                    format::lookup("cpk").ok_or_else(|| Error::NotFound(raw.to_string()))?;
                let metadata = std::fs::metadata(&cpk).map_err(|_| Error::NotFound(cpk.clone()))?;
                let name = Path::new(&cpk)
                    .file_name()
                    .map(|value| value.to_string_lossy().into_owned())
                    .unwrap_or_else(|| cpk.clone());

                Ok(catalog::FileMetaDto {
                    entry: catalog::dto::FileEntryDto {
                        path: raw.to_string(),
                        name,
                        ext: found.ext.to_string(),
                        size: metadata.len(),
                        mtime_ms: mtime_ms(&metadata),
                        kind: found.kind,
                        label: found.label,
                        icon: found.kind.icon(),
                        viewers: found.viewers,
                        has_thumb: found.has_thumb,
                    },
                    related: Vec::new(),
                })
            }
            Target::Archived { .. } => {
                let (parent, name) = split_directory(raw);
                let listing = self.list(&parent)?;
                let entry = listing
                    .files
                    .iter()
                    .find(|file| file.name.eq_ignore_ascii_case(&name))
                    .ok_or_else(|| Error::NotFound(raw.to_string()))?
                    .clone();

                let stem = file_stem(&name);
                let related = listing
                    .files
                    .iter()
                    .filter(|other| !other.name.eq_ignore_ascii_case(&entry.name))
                    .filter(|other| file_stem(&other.name).eq_ignore_ascii_case(&stem))
                    .map(|other| catalog::RelatedFileDto {
                        path: other.path.clone(),
                        name: other.name.clone(),
                        ext: other.ext.clone(),
                        kind: other.kind,
                        label: other.label,
                    })
                    .collect();

                Ok(catalog::FileMetaDto { entry, related })
            }
        }
    }

    pub fn sibling_container(&self, raw: &str) -> Option<String> {
        let (directory, name) = split_directory(raw);
        let stem = file_stem(&name);
        let lower = name.to_lowercase();
        let extension = lower
            .rsplit_once('.')
            .map(|(_, ext)| ext)
            .unwrap_or_default();
        if extension != "ftx" && extension != "ftp" {
            let peers = self.peers(raw);
            for candidate_extension in ["ftx", "ftp"] {
                let candidate = format!("{stem}.{candidate_extension}");
                if peers
                    .iter()
                    .any(|peer| peer.eq_ignore_ascii_case(&candidate))
                {
                    return Some(format!("{directory}\\{candidate}"));
                }
            }
        }

        let peers = self.peers(raw);
        let matches: Vec<&String> = peers
            .iter()
            .filter(|peer| {
                let Some((other_stem, other_extension)) = peer.rsplit_once('.') else {
                    return false;
                };
                if !other_extension.eq_ignore_ascii_case("ftx")
                    && !other_extension.eq_ignore_ascii_case("ftp")
                {
                    return false;
                }
                numeric_suffix(other_stem, &stem)
            })
            .collect();

        if matches.len() == 1 {
            return Some(format!("{directory}\\{}", matches[0]));
        }
        None
    }

    pub fn summary(&self, cpk: &str) -> Result<archive::CpkSummary, Error> {
        let archive = self.archive(cpk)?;
        Ok(archive.summary().clone())
    }

    pub fn export(&self, raw: &str, out_path: &Path) -> Result<u64, Error> {
        let data = self.read(raw)?;
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| Error::Io(error.to_string()))?;
        }
        std::fs::write(out_path, &data).map_err(|error| Error::Io(error.to_string()))?;
        Ok(data.len() as u64)
    }

    pub fn cache_report(&self) -> (usize, usize, Option<PathBuf>) {
        let (used, budget, entries) = self
            .bytes
            .lock()
            .map(|cache| (cache.used, cache.budget, cache.map.len()))
            .unwrap_or((0, 0, 0));
        let _ = entries;
        (used, budget, self.spill.clone())
    }

    pub fn clear(&self) {
        if let Ok(mut cache) = self.bytes.lock() {
            cache.clear();
        }
        if let Ok(mut cache) = self.archives.lock() {
            cache.clear();
        }
        if let Some(spill) = &self.spill {
            let _ = std::fs::remove_dir_all(spill);
        }
    }

    fn wants_spill(&self, index: usize, archive: &CpkArchive) -> bool {
        self.spill.is_some()
            && archive
                .entry(index)
                .map(|entry| entry.extract_size > self.spill_threshold)
                .unwrap_or(false)
    }

    fn bucket(&self, cpk: &str) -> Option<PathBuf> {
        self.spill
            .as_ref()
            .map(|root| root.join(format!("{:016x}", fnv1a64(&cpk.to_lowercase()))))
    }

    fn materialize_entry(
        &self,
        cpk: &str,
        archive: &CpkArchive,
        index: usize,
    ) -> Result<PathBuf, Error> {
        let Some(bucket) = self.bucket(cpk) else {
            return Err(Error::Io(
                "no archive cache directory is configured".to_string(),
            ));
        };
        let entry = archive
            .entry(index)
            .ok_or_else(|| Error::NotFound(cpk.to_string()))?;
        let target = bucket.join(entry_relative(entry));
        if !target.is_file() {
            write_entry(archive, index, &target)?;
        }
        Ok(target)
    }

    fn materialize_family(
        &self,
        cpk: &str,
        archive: &CpkArchive,
        index: usize,
    ) -> Result<PathBuf, Error> {
        let path = self.materialize_entry(cpk, archive, index)?;
        let Some(entry) = archive.entry(index) else {
            return Ok(path);
        };
        let stem = file_stem(&entry.name);
        let directory = entry.dir.clone();

        for (other_index, other) in archive.entries().iter().enumerate() {
            if other_index == index || other.dir != directory {
                continue;
            }
            if !same_family(&file_stem(&other.name), &stem) {
                continue;
            }
            let _ = self.materialize_entry(cpk, archive, other_index);
        }

        if extension_of(&entry.name) == "rsc" {
            let pool = archive.entries().iter().position(|candidate| {
                candidate
                    .dir
                    .eq_ignore_ascii_case(formats::rsc::resolve::POOL_DIR)
                    && candidate
                        .name
                        .eq_ignore_ascii_case(formats::rsc::resolve::POOL_FILE)
            });
            if let Some(pool) = pool {
                let _ = self.materialize_entry(cpk, archive, pool);
            }
        }

        Ok(path)
    }

    fn materialize_tree(
        &self,
        cpk: &str,
        archive: &CpkArchive,
        inner: &str,
    ) -> Result<PathBuf, Error> {
        let Some(bucket) = self.bucket(cpk) else {
            return Err(Error::Io(
                "no archive cache directory is configured".to_string(),
            ));
        };
        let prefix = inner.trim_matches('/');
        let nested = format!("{prefix}/");
        let mut written = 0usize;

        for (index, entry) in archive.entries().iter().enumerate() {
            let inside = prefix.is_empty() || entry.dir == prefix || entry.dir.starts_with(&nested);
            if !inside {
                continue;
            }
            let target = bucket.join(entry_relative(entry));
            if !target.is_file() {
                write_entry(archive, index, &target)?;
            }
            written += 1;
        }

        if written == 0 {
            return Err(Error::NotFound(inner.to_string()));
        }
        Ok(if prefix.is_empty() {
            bucket
        } else {
            bucket.join(prefix.replace('/', "\\"))
        })
    }

    fn archive_bytes(
        &self,
        cpk: &str,
        archive: &CpkArchive,
        index: usize,
    ) -> Result<Arc<Vec<u8>>, Error> {
        let key = format!("{cpk}|{index}");

        if let Ok(cache) = self.bytes.lock() {
            if let Some(hit) = cache.get(&key) {
                return Ok(hit);
            }
        }

        if self.wants_spill(index, archive) {
            let path = self.materialize_entry(cpk, archive, index)?;
            let data = std::fs::read(&path).map_err(|error| Error::Io(error.to_string()))?;
            return Ok(Arc::new(data));
        }

        let data = Arc::new(archive.read(index)?);
        if let Ok(mut cache) = self.bytes.lock() {
            cache.put(key, data.clone());
        }
        Ok(data)
    }
}

pub fn build_index(
    mount: &Mount,
    root: &Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u32),
) -> Result<Vec<IndexEntry>, Error> {
    let mut entries = catalog::build_index(root, cancel, &mut progress)?;

    let archives: Vec<String> = entries
        .iter()
        .filter(|entry| entry.format.kind == Kind::Archive)
        .map(|entry| entry.path.clone())
        .collect();

    for cpk in archives {
        let Ok(archive) = mount.archive(&cpk) else {
            continue;
        };
        for entry in archive.entries() {
            let ext = extension_of(&entry.name);
            let Some(found) = format::lookup(&ext) else {
                continue;
            };
            entries.push(IndexEntry::new(
                display_path(&cpk, &entry.dir, &entry.name),
                entry.name.clone(),
                found,
            ));
        }
        progress(entries.len() as u32);
    }

    Ok(entries)
}

pub fn is_archive_path(path: &str) -> bool {
    matches!(split(path), Split::Archive { .. })
}

pub fn index_archive(mount: &Mount, cpk: &str) -> Result<Vec<IndexEntry>, Error> {
    let archive = mount.archive(cpk)?;
    let mut entries = Vec::with_capacity(archive.entries().len());

    if let Some(found) = format::lookup("cpk") {
        let name = Path::new(cpk)
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_default();
        entries.push(IndexEntry::new(cpk.to_string(), name, found));
    }

    for entry in archive.entries() {
        let ext = extension_of(&entry.name);
        let Some(found) = format::lookup(&ext) else {
            continue;
        };
        entries.push(IndexEntry::new(
            display_path(cpk, &entry.dir, &entry.name),
            entry.name.clone(),
            found,
        ));
    }

    Ok(entries)
}

fn extension_of(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .unwrap_or_default()
}

fn mtime_ms(metadata: &std::fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn cpk_mtime_ms(cpk: &str) -> u64 {
    std::fs::metadata(cpk)
        .map(|metadata| mtime_ms(&metadata))
        .unwrap_or(0)
}

fn display_path(cpk: &str, inner: &str, name: &str) -> String {
    let inner = inner.trim_matches(['\\', '/']);
    if inner.is_empty() {
        format!("{cpk}\\{name}")
    } else {
        format!("{cpk}\\{}\\{name}", inner.replace('/', "\\"))
    }
}

fn display_self(cpk: &str, inner: &str) -> String {
    let inner = inner.trim_matches(['\\', '/']);
    if inner.is_empty() {
        cpk.to_string()
    } else {
        format!("{cpk}\\{}", inner.replace('/', "\\"))
    }
}

fn parent_of(cpk: &str, inner: &str) -> Option<String> {
    let inner = inner.trim_matches(['\\', '/']);
    if inner.is_empty() {
        return Path::new(cpk).parent().map(fslist::normalize);
    }
    match inner.rfind('/') {
        Some(at) => Some(display_self(cpk, &inner[..at])),
        None => Some(cpk.to_string()),
    }
}

fn parent_inner(inner: &str) -> String {
    let inner = inner.trim_matches(['\\', '/']).replace('\\', "/");
    match inner.rfind('/') {
        Some(at) => inner[..at].to_string(),
        None => String::new(),
    }
}

fn split_directory(raw: &str) -> (String, String) {
    let text = raw.trim_end_matches(['\\', '/']);
    match text.rfind(['\\', '/']) {
        Some(at) => (text[..at].to_string(), text[at + 1..].to_string()),
        None => (String::new(), text.to_string()),
    }
}

fn fnv1a64(text: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

impl formats::Source for Mount {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        Mount::read(self, path).ok()
    }

    fn peers(&self, path: &str) -> Vec<String> {
        Mount::peers(self, path)
    }

    fn walk_names(&self, dir: &str, depth: u8) -> Vec<String> {
        Mount::walk_names(self, dir, depth)
    }
}

fn entry_relative(entry: &archive::CpkEntry) -> PathBuf {
    let mut path = PathBuf::new();
    if !entry.dir.is_empty() {
        path.push(entry.dir.replace('/', "\\"));
    }
    path.push(&entry.name);
    path
}

fn write_entry(archive: &CpkArchive, index: usize, target: &Path) -> Result<(), Error> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|error| Error::Io(error.to_string()))?;
    }
    let data = archive.read(index)?;
    let mut temp = target.as_os_str().to_owned();
    temp.push(".part");
    let temp = PathBuf::from(temp);
    std::fs::write(&temp, &data).map_err(|error| Error::Io(error.to_string()))?;
    std::fs::rename(&temp, target).map_err(|error| Error::Io(error.to_string()))?;
    Ok(())
}

fn file_stem(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(stem, _)| stem.to_string())
        .unwrap_or_else(|| name.to_string())
}

fn numeric_suffix(candidate: &str, stem: &str) -> bool {
    let candidate = candidate.to_lowercase();
    let stem = stem.to_lowercase();
    candidate.len() > stem.len()
        && candidate.starts_with(&stem)
        && candidate[stem.len()..]
            .chars()
            .all(|character| character.is_ascii_digit())
}

fn same_family(candidate: &str, stem: &str) -> bool {
    candidate.eq_ignore_ascii_case(stem) || numeric_suffix(candidate, stem)
}
