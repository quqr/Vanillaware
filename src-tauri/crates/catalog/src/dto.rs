use serde::Serialize;

use crate::format::{Format, Kind};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FormatDto {
    pub ext: &'static str,
    pub kind: Kind,
    pub kind_label: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
    pub viewers: &'static [&'static str],
    pub has_thumb: bool,
}

impl From<&Format> for FormatDto {
    fn from(format: &Format) -> Self {
        Self {
            ext: format.ext,
            kind: format.kind,
            kind_label: format.kind.label(),
            label: format.label,
            icon: format.kind.icon(),
            viewers: format.viewers,
            has_thumb: format.has_thumb,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DirEntryDto {
    pub path: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<Kind>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileEntryDto {
    pub path: String,
    pub name: String,
    pub ext: String,
    pub size: u64,
    pub mtime_ms: u64,
    pub kind: Kind,
    pub label: &'static str,
    pub icon: &'static str,
    pub viewers: &'static [&'static str],
    pub has_thumb: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirListingDto {
    pub path: String,
    pub parent: Option<String>,
    pub dirs: Vec<DirEntryDto>,
    pub files: Vec<FileEntryDto>,
    pub hidden_files: u32,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMetaDto {
    pub entry: FileEntryDto,
    pub related: Vec<RelatedFileDto>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RelatedFileDto {
    pub path: String,
    pub name: String,
    pub ext: String,
    pub kind: Kind,
    pub label: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IndexedFileDto {
    pub path: String,
    pub name: String,
    pub ext: String,
    pub kind: Kind,
    pub label: &'static str,
    pub icon: &'static str,
    pub viewers: &'static [&'static str],
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultDto {
    pub items: Vec<IndexedFileDto>,
    pub truncated: bool,
    pub total: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexState {
    Idle,
    Building,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStatusDto {
    pub state: IndexState,
    pub count: u32,
    pub root: Option<String>,
    pub error: Option<String>,
}
