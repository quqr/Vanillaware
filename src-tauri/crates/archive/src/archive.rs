use std::collections::BTreeSet;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::table::Table;
use crate::{container, crilayla, toc, Error, Result};

#[derive(Debug, Clone)]
pub struct CpkEntry {
    pub dir: String,
    pub name: String,
    pub size: u64,
    pub extract_size: u64,
    pub offset: u64,
    pub user_string: Option<String>,
}

impl CpkEntry {
    pub fn path(&self) -> String {
        if self.dir.is_empty() {
            self.name.clone()
        } else {
            format!("{}/{}", self.dir, self.name)
        }
    }

    pub fn compressed_size(&self) -> u64 {
        self.size
    }
}

#[derive(Debug, Clone)]
pub struct CpkSummary {
    pub path: String,
    pub file_size: u64,
    pub magic: String,
    pub version: u16,
    pub revision: u16,
    pub align: u16,
    pub entries: usize,
    pub directories: usize,
    pub total_extract: u64,
    pub compressed: usize,
    pub out_of_range: usize,
    pub declared_files: u64,
    pub toc_offset: u64,
    pub content_offset: u64,
    pub content_base_is_toc: bool,
    pub header_encrypted: bool,
    pub toc_encrypted: bool,
    pub header_padding: usize,
    pub toc_padding: usize,
}

pub struct CpkArchive {
    path: PathBuf,
    file_len: u64,
    file: Mutex<File>,
    entries: Vec<CpkEntry>,
    summary: CpkSummary,
}

impl CpkArchive {
    pub fn open(path: &Path) -> Result<Self> {
        let mut file = File::open(path)?;
        let file_len = file.metadata()?.len();

        let header = container::read(&mut file, 0, file_len)?;
        let magic = String::from_utf8_lossy(&header.magic).into_owned();
        let header_encrypted = header.encrypted;
        let header_table = Table::parse(header.body)?;

        let locate = toc::locate(&header_table)?;

        let body = container::read(&mut file, locate.toc_offset, file_len)?;
        let toc_encrypted = body.encrypted;
        let toc_table = Table::parse(body.body)?;

        let entries = toc::entries(&toc_table, locate.content_offset)?;

        let directories = entries
            .iter()
            .map(|entry| entry.dir.as_str())
            .collect::<BTreeSet<_>>()
            .len();

        let out_of_range = entries
            .iter()
            .filter(|entry| entry.offset + entry.size > file_len)
            .count();

        let mut archive = Self {
            path: path.to_path_buf(),
            file_len,
            file: Mutex::new(file),
            summary: CpkSummary {
                path: path.to_string_lossy().into_owned(),
                file_size: file_len,
                magic,
                version: toc::value(&header_table, "Version").unwrap_or(0) as u16,
                revision: toc::value(&header_table, "Revision").unwrap_or(0) as u16,
                align: toc::value(&header_table, "Align").unwrap_or(0) as u16,
                entries: entries.len(),
                directories,
                total_extract: entries.iter().map(|entry| entry.extract_size).sum(),
                compressed: 0,
                out_of_range,
                declared_files: toc::value(&header_table, "Files").unwrap_or(0),
                toc_offset: locate.toc_offset,
                content_offset: locate.content_offset,
                content_base_is_toc: locate.content_base_is_toc,
                header_encrypted,
                toc_encrypted,
                header_padding: header_table.padding(),
                toc_padding: toc_table.padding(),
            },
            entries,
        };

        archive.summary.compressed = archive.count_compressed()?;
        Ok(archive)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn len(&self) -> u64 {
        self.file_len
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[CpkEntry] {
        &self.entries
    }

    pub fn entry(&self, index: usize) -> Option<&CpkEntry> {
        self.entries.get(index)
    }

    pub fn summary(&self) -> &CpkSummary {
        &self.summary
    }

    pub fn find(&self, inner: &str) -> Option<usize> {
        let needle = inner.trim_matches('/');
        self.entries
            .iter()
            .position(|entry| entry.path().eq_ignore_ascii_case(needle))
    }

    pub fn read_raw(&self, index: usize) -> Result<Vec<u8>> {
        let entry = self.entries.get(index).ok_or(Error::OutOfRange {
            what: "entry index",
            value: index as u64,
            limit: self.entries.len() as u64,
        })?;

        let mut data = self.read_at(entry.offset, entry.size as usize)?;
        if !crilayla::is_compressed(&data) {
            return Ok(data);
        }

        if let Some(needed) = crilayla::compressed_len(&data) {
            if (data.len() as u64) < needed {
                data = self.read_at(entry.offset, needed as usize)?;
            }
        }

        Ok(data)
    }

    pub fn read(&self, index: usize) -> Result<Vec<u8>> {
        let data = self.read_raw(index)?;
        if crilayla::is_compressed(&data) {
            crilayla::decompress(&data)
        } else {
            Ok(data)
        }
    }

    pub fn read_path(&self, inner: &str) -> Result<Vec<u8>> {
        let index = self.find(inner).ok_or(Error::OutOfRange {
            what: "entry path",
            value: 0,
            limit: self.entries.len() as u64,
        })?;
        self.read(index)
    }

    pub fn read_at(&self, offset: u64, len: usize) -> Result<Vec<u8>> {
        if offset.saturating_add(len as u64) > self.file_len {
            return Err(Error::OutOfRange {
                what: "read range",
                value: offset.saturating_add(len as u64),
                limit: self.file_len,
            });
        }

        let mut file = self
            .file
            .lock()
            .map_err(|_| Error::Corrupt("archive handle is poisoned"))?;
        file.seek(SeekFrom::Start(offset))?;
        let mut buf = vec![0u8; len];
        file.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn count_compressed(&self) -> Result<usize> {
        let mut count = 0;
        for entry in &self.entries {
            let head_len = entry.size.min(crilayla::MAGIC.len() as u64) as usize;
            if head_len < crilayla::MAGIC.len() {
                continue;
            }
            let head = self.read_at(entry.offset, head_len)?;
            if crilayla::is_compressed(&head) {
                count += 1;
            }
        }
        Ok(count)
    }

    pub fn child_dirs(&self, inner: &str) -> Vec<String> {
        let prefix = inner.trim_matches('/');
        let mut dirs = BTreeSet::new();
        for entry in &self.entries {
            let rest = if prefix.is_empty() {
                entry.dir.as_str()
            } else if entry.dir.len() > prefix.len()
                && entry.dir.starts_with(prefix)
                && entry.dir.as_bytes()[prefix.len()] == b'/'
            {
                &entry.dir[prefix.len() + 1..]
            } else {
                continue;
            };
            if let Some(head) = rest.split('/').next() {
                if !head.is_empty() {
                    dirs.insert(head.to_string());
                }
            }
        }
        dirs.into_iter().collect()
    }

    pub fn child_entries(&self, inner: &str) -> Vec<usize> {
        let prefix = inner.trim_matches('/');
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.dir == prefix)
            .map(|(index, _)| index)
            .collect()
    }

    pub fn has_dir(&self, inner: &str) -> bool {
        let prefix = inner.trim_matches('/');
        if prefix.is_empty() {
            return true;
        }
        let nested = format!("{prefix}/");
        self.entries
            .iter()
            .any(|entry| entry.dir == prefix || entry.dir.starts_with(&nested))
    }
}
