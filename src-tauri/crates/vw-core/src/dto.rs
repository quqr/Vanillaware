//! 跨语言传输对象。字段名统一 camelCase，前端直接用。
//!
//! 这里只描述**已收录的原生二进制资源**。目录结构（子目录）与文件条目分开传输：
//! 目录树需要的是路径与名字，文件列表需要的是格式声明。

use serde::Serialize;

use crate::format::{Format, Kind};

/// 注册表里的一条格式声明（工具栏过滤项的唯一真值）。
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

/// 一个子目录。
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DirEntryDto {
    /// 绝对路径，已去掉 Windows 的 `\\?\` 前缀。
    pub path: String,
    pub name: String,
}

/// 一个已收录的文件条目。
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileEntryDto {
    pub path: String,
    /// 含扩展名的文件名。
    pub name: String,
    /// 小写、不含点。
    pub ext: String,
    pub size: u64,
    /// Unix 毫秒；取不到时为 0。
    pub mtime_ms: u64,
    pub kind: Kind,
    /// 给人看的格式名，如 `FTEX`。
    pub label: &'static str,
    /// 图标语义名。
    pub icon: &'static str,
    /// 可用查看器 id，按展示顺序。
    pub viewers: &'static [&'static str],
    /// 是否支持缩略图。
    pub has_thumb: bool,
}

/// 一次目录读取的结果。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirListingDto {
    /// 当前目录绝对路径。
    pub path: String,
    /// 上一级；根目录的上一级为 `None`。
    pub parent: Option<String>,
    pub dirs: Vec<DirEntryDto>,
    pub files: Vec<FileEntryDto>,
    /// 因不在注册表里而被过滤掉的文件数。让用户知道「这里有东西，但本工具不收录」。
    pub hidden_files: u32,
    /// 权限、坏路径等非致命问题，最多 50 条。
    pub warnings: Vec<String>,
}

/// 一个文件的完整信息（选中时取）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMetaDto {
    pub entry: FileEntryDto,
    /// 同目录下同基础名的其他已收录文件。
    pub related: Vec<RelatedFileDto>,
}

/// 关联文件：同目录、同基础名、不同扩展名。
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RelatedFileDto {
    pub path: String,
    pub name: String,
    pub ext: String,
    pub kind: Kind,
    pub label: &'static str,
}

/// 索引里的一个文件。
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IndexedFileDto {
    pub path: String,
    pub name: String,
    /// 小写、不含点。
    pub ext: String,
    pub kind: Kind,
    pub label: &'static str,
    pub icon: &'static str,
    /// 可用查看器 id（由格式决定，索引期就能确定）。
    pub viewers: &'static [&'static str],
}

/// 全局搜索结果。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultDto {
    pub items: Vec<IndexedFileDto>,
    /// 命中数超过上限时为 true（结果被截断）。
    pub truncated: bool,
    /// 命中总数（可能大于 `items.len()`）。
    pub total: u32,
}

/// 索引状态。前端据此显示进度，并避免把「未建完」误读成「没有匹配」。
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
    /// 已收集的条目数（构建中即为进度）。
    pub count: u32,
    pub root: Option<String>,
    pub error: Option<String>,
}
