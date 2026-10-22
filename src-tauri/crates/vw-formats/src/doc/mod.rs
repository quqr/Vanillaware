//! 统一文档树：各格式把结构投影成 arena，前端按节点分页展开。
//!
//! 本模块只依赖字节与格式自身，不依赖 `vw-core`；DTO 直接由命令层返回。

pub mod arena;
pub mod binary;
pub mod ftx;
pub mod mbs;
pub mod rdr;

use std::path::Path;

pub use arena::{
    DocArena, DocCheckDto, DocDto, DocMark, DocNodeDto, DocNodeKind, DocOpenDto, DocPageDto, Builder,
    MAX_DEPTH, MAX_NODES, MAX_STRING, PAGE_LIMIT, PAGE_MAX,
};

use crate::Result;

/// 从磁盘解析一个文件的文档树。
pub fn parse_file(path: &Path, tag: Option<&str>) -> Result<DocArena> {
    let bytes = std::fs::read(path)?;
    let ext = path
        .extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    parse_bytes(&bytes, &ext, tag)
}

/// 从内存字节解析；`ext` 决定分派，未知扩展名走通用 binary 视图。
pub fn parse_bytes(bytes: &[u8], ext: &str, tag: Option<&str>) -> Result<DocArena> {
    match ext.to_ascii_lowercase().as_str() {
        "ftx" | "ftp" => Ok(ftx::build(bytes)),
        "mbs" | "mbp" => {
            let tag = tag.unwrap_or("swi_unic");
            Ok(mbs::build(bytes, tag))
        }
        _ => Ok(binary::build(bytes, ext)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_view_is_truncation_safe() {
        for len in 0..128 {
            let bytes = vec![0x41u8; len];
            let arena = parse_bytes(&bytes, "nnd", None).expect("binary");
            assert!(arena.total_nodes() > 0);
        }
    }

    #[test]
    fn ftx_view_reads_header_and_pages() {
        let mut bytes = vec![0u8; 0x80];
        bytes[..4].copy_from_slice(b"FTEX");
        bytes[4..8].copy_from_slice(&0x20u32.to_le_bytes());
        bytes[8..12].copy_from_slice(&0x50u32.to_le_bytes());
        bytes[12..16].copy_from_slice(&1u32.to_le_bytes());
        bytes[0x20..0x24].copy_from_slice(b"a.nv");

        let arena = parse_bytes(&bytes, "ftx", None).expect("ftx");
        let page = arena.page(1, arena.root(), 0, 100).unwrap();
        assert!(page.total >= 2);
    }

    #[test]
    fn mbs_view_reads_sections() {
        let mut bytes = vec![0u8; 0x130];
        bytes[..4].copy_from_slice(b"FMBS");
        bytes[4..8].copy_from_slice(&0u32.to_le_bytes());
        bytes[0x14..0x16].copy_from_slice(&0x77u16.to_le_bytes());

        let arena = parse_bytes(&bytes, "mbs", Some("swi_unic")).expect("mbs");
        let page = arena.page(1, arena.root(), 0, 100).unwrap();
        assert!(page.total >= 2);
    }

    #[test]
    fn unsupported_error_is_never_panicking() {
        let prefix = b"FTEX";
        for len in 0..prefix.len() {
            let _ = parse_bytes(&prefix[..len], "ftx", None);
        }
    }
}
