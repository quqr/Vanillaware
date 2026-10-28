//! 统一文档树：各格式把结构投影成 arena，前端按节点分页展开。
//!
//! 本模块只依赖字节与格式自身，不依赖 `vw-core`；DTO 直接由命令层返回。

pub mod acb;
pub mod arena;
pub mod asb;
pub mod asb_ops;
pub mod awb;
pub mod bbs;
pub mod binary;
pub mod bst;
pub mod bundle;
pub mod ctb;
pub mod drg;
pub mod fmd;
pub mod fms;
pub mod fsb;
pub mod ftx;
pub mod mbs;
pub mod mpd;
pub mod nnd;
pub mod oeb;
pub mod orb;
pub mod osb;
pub mod otm;
pub mod rdr;
pub mod resolve;
pub mod rsc;
pub mod scb;
pub mod text;
pub mod ucb;
pub mod ufb;
pub mod usm;
pub mod utf;
pub mod vs2;

use std::path::Path;

pub use arena::{
    Builder, DocArena, DocCheckDto, DocDto, DocMark, DocNodeDto, DocNodeKind, DocOpenDto,
    DocPageDto, MAX_DEPTH, MAX_NODES, MAX_STRING, PAGE_LIMIT, PAGE_MAX,
};
pub use asb::{AsmDto, PseudoDto};
pub use bbs::BbsLayoutDto;
pub use fms::FmsTextDto;

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
        "bbs" => Ok(bbs::build(bytes)),
        "osb" => Ok(osb::build(bytes)),
        "fms" => Ok(fms::build(bytes)),
        "rsc" => Ok(rsc::build(bytes)),
        "otm" => Ok(otm::build(bytes)),
        "mpd" => Ok(mpd::build(bytes)),
        "ctb" => Ok(ctb::build(bytes)),
        "fmd" => Ok(fmd::build(bytes)),
        "fsb" => Ok(fsb::build(bytes)),
        "asb" => asb::build(bytes).map_err(crate::Error::BadData),
        "scb" => Ok(scb::build(bytes)),
        "nnd" => Ok(nnd::build(bytes)),
        "acb" => Ok(acb::build(bytes, "ACB")),
        "acf" => Ok(utf::build(bytes, "ACF")),
        "awb" => Ok(awb::build(bytes, "AFS2")),
        "usm" => Ok(usm::build(bytes)),
        "ufb" => Ok(ufb::build(bytes)),
        "ucb" => Ok(ucb::build(bytes)),
        "drg" => Ok(drg::build(bytes)),
        "vs2" => Ok(vs2::build(bytes)),
        "inc" => Ok(text::build_plain(bytes, "INC")),
        "oeb" => Ok(oeb::build(bytes)),
        "orb" => Ok(orb::build(bytes)),
        "bst" => Ok(bst::build(bytes)),
        _ => Ok(binary::build(bytes, ext)),
    }
}
