use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveSummaryDto {
    pub path: String,
    pub magic: String,
    pub version: u16,
    pub revision: u16,
    pub align: u16,
    pub file_size: u64,
    pub entries: u32,
    pub directories: u32,
    pub declared_files: u32,
    pub total_extract: u64,
    pub compressed: u32,
    pub out_of_range: u32,
    pub toc_offset: u64,
    pub content_offset: u64,
    pub header_encrypted: bool,
    pub toc_encrypted: bool,
}

impl From<archive::CpkSummary> for ArchiveSummaryDto {
    fn from(summary: archive::CpkSummary) -> Self {
        Self {
            path: summary.path,
            magic: summary.magic,
            version: summary.version,
            revision: summary.revision,
            align: summary.align,
            file_size: summary.file_size,
            entries: summary.entries as u32,
            directories: summary.directories as u32,
            declared_files: summary.declared_files as u32,
            total_extract: summary.total_extract,
            compressed: summary.compressed as u32,
            out_of_range: summary.out_of_range as u32,
            toc_offset: summary.toc_offset,
            content_offset: summary.content_offset,
            header_encrypted: summary.header_encrypted,
            toc_encrypted: summary.toc_encrypted,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveCacheDto {
    pub used_bytes: u64,
    pub budget_bytes: u64,
    pub root: Option<String>,
}
