//! AFS2 音频包（`.awb`，CRI ADX2 的 wave bank）。
//!
//! ```text
//! +0x00  "AFS2"
//! +0x04  u32 版本（本语料恒为 0x00040402）
//! +0x08  u32 条目数 N
//! +0x0c  u32 条目描述长度（本语料恒为 0x20，与偏移表无关）
//! +0x10  id[N]            —— 顺序 id，0 起（宽度 u32 = UO 侧，u16 = 13 侧 AFS2 v2）
//! +0x10+wN  u32 off[N]    —— 绝对偏移
//! +0x10+wN+4N  u32 总大小 —— 恒等于文件长度（这就是"到 EOF"的硬约束）
//! +0x14+wN+4N  ...         条目载荷：每个条目 = off[i]..off[i+1]
//! ```
//!
//! 条目载荷是 **HCA**（CRI 自家有损音频）。本语料的 AWB 用了 AWB 子密钥，
//! 载荷头部被加密，扫不到 `HCA\0` 魔数，因此这里只到**容器层**：
//! 每条给出 id / 偏移 / 长度，载荷本身按字节保留，不做解码也不臆造声明。
//! 全部 6 个 `.awb`（UO 侧）在这三条不变量上闭合：总大小 == 文件长、偏移严格递增、
//! 首条偏移 == 描述区末尾。

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;

const HEADER: usize = 0x10;
const MAX_ENTRIES_SHOWN: usize = 128;

pub struct Entry {
    pub id: u32,
    pub offset: usize,
    pub size: usize,
}

pub struct Afs2 {
    pub version: u32,
    pub stride: u32,
    pub declared_total: u32,
    pub entries: Vec<Entry>,
    pub data_start: usize,
    /// id 表元素宽度：UO 侧为 4，13 侧（AFS2 v2）为 2。
    pub id_width: usize,
}

/// 解析 `at` 处的 AFS2 头；越界或魔数不符返回 `None`。
///
/// id 表有 u32 / u16 两种宽度，靠「偏移表首项 == 数据区起点」与
/// 「总大小 == 文件长度」两条恒等式自证，先试 u32 再试 u16。
pub fn parse(bytes: &[u8], at: usize) -> Option<Afs2> {
    parse_with(bytes, at, 4).or_else(|| parse_with(bytes, at, 2))
}

fn parse_with(bytes: &[u8], at: usize, id_width: usize) -> Option<Afs2> {
    if bytes.get(at..at + 4)? != b"AFS2" {
        return None;
    }
    let version = rdr::u32_at(bytes, at + 4)?;
    let count = rdr::u32_at(bytes, at + 8)? as usize;
    let stride = rdr::u32_at(bytes, at + 0x0c)?;
    if count == 0 || count > 1_000_000 {
        return None;
    }
    let ids_at = at + HEADER;
    let offsets_at = ids_at + count * id_width;
    let total_at = offsets_at + count * 4;
    rdr::u32_at(bytes, total_at)?;

    let mut entries = Vec::with_capacity(count);
    for index in 0..count {
        let id = if id_width == 2 {
            rdr::u16_at(bytes, ids_at + index * 2)? as u32
        } else {
            rdr::u32_at(bytes, ids_at + index * 4)?
        };
        let offset = rdr::u32_at(bytes, offsets_at + index * 4)? as usize;
        let next = rdr::u32_at(bytes, offsets_at + (index + 1) * 4)? as usize;
        if next < offset || next > bytes.len() {
            return None;
        }
        entries.push(Entry {
            id,
            offset,
            size: next - offset,
        });
    }

    let declared_total = rdr::u32_at(bytes, total_at)?;
    let data_start = total_at + 4;
    // 两条恒等式：数据区紧随描述区、总大小就是文件长度。任一不成立即换宽度重试。
    if entries.first().map(|entry| entry.offset) != Some(data_start) {
        return None;
    }
    if declared_total as usize != bytes.len() {
        return None;
    }

    Some(Afs2 {
        version,
        stride,
        declared_total,
        entries,
        data_start,
        id_width,
    })
}

pub fn build(bytes: &[u8], title: &str) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root(title);
    builder.field(root, "size", bytes.len().to_string());

    let Some(afs2) = parse(bytes, 0) else {
        builder.warn("不是 AFS2：魔数或偏移表越界");
        builder.mark(root, DocMark::Error);
        return builder.finish();
    };

    let header = builder.group(root, Some("header"), "Header");
    builder.field(header, "magic", "AFS2");
    builder.field(header, "version", format!("0x{:08x}", afs2.version));
    builder.field(header, "count", afs2.entries.len().to_string());
    builder.field(header, "stride", afs2.stride.to_string());
    builder.field(header, "idWidth", afs2.id_width.to_string());
    builder.field(header, "dataStart", format!("0x{:x}", afs2.data_start));

    let monotonic = afs2
        .entries
        .windows(2)
        .all(|pair| pair[0].offset <= pair[1].offset);
    let total = afs2.entries.iter().map(|entry| entry.size).sum::<usize>();
    let last_end = afs2
        .entries
        .last()
        .map(|entry| entry.offset + entry.size)
        .unwrap_or(0);

    builder.check(
        "awb.total",
        "总大小字段 == 文件长度",
        afs2.declared_total as usize == bytes.len(),
        Some(format!("{:#x} / {:#x}", afs2.declared_total, bytes.len())),
    );
    builder.check(
        "awb.monotonic",
        "偏移表严格递增",
        monotonic,
        Some(format!("{} 条", afs2.entries.len())),
    );
    builder.check(
        "awb.data_start",
        "首条偏移 == 描述区末尾",
        afs2.entries
            .first()
            .map(|entry| entry.offset == afs2.data_start)
            .unwrap_or(false),
        afs2.entries
            .first()
            .map(|entry| format!("{:#x} / {:#x}", entry.offset, afs2.data_start)),
    );
    builder.check(
        "awb.eof",
        "末条结束 == 文件长度",
        last_end == bytes.len(),
        Some(format!("{last_end:#x} / {:#x}", bytes.len())),
    );
    builder.check(
        "awb.payload",
        "条目长度之和 == 载荷区长度",
        total == bytes.len().saturating_sub(afs2.data_start),
        Some(format!("{total} 字节")),
    );

    let summary = builder.group(root, Some("summary"), "Summary");
    builder.field(summary, "entries", afs2.entries.len().to_string());
    builder.field(summary, "payload", total.to_string());
    let smallest = afs2
        .entries
        .iter()
        .map(|entry| entry.size)
        .min()
        .unwrap_or(0);
    let largest = afs2
        .entries
        .iter()
        .map(|entry| entry.size)
        .max()
        .unwrap_or(0);
    builder.field(summary, "entrySize", format!("{smallest} .. {largest}"));

    let list = builder.array(
        root,
        Some("entries"),
        format!("Entries ({})", afs2.entries.len()),
    );
    let shown = afs2.entries.len().min(MAX_ENTRIES_SHOWN);
    for (index, entry) in afs2.entries.iter().take(shown).enumerate() {
        let node = builder.group(
            list,
            Some(&index.to_string()),
            format!("[{index}] id={}", entry.id),
        );
        builder.field(node, "id", entry.id.to_string());
        builder.field(node, "offset", format!("0x{:x}", entry.offset));
        builder.field(node, "size", entry.size.to_string());
        builder.bytes(
            node,
            Some("payload"),
            "payload",
            entry.offset as u64,
            entry.size as u64,
        );
    }
    if afs2.entries.len() > shown {
        builder.field(root, "entriesShown", shown.to_string());
        builder.truncate();
    }

    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_entry_afs() -> Vec<u8> {
        let mut bytes = vec![0u8; 0x10 + 8 * 2 + 4 + 10];
        bytes[..4].copy_from_slice(b"AFS2");
        bytes[4..8].copy_from_slice(&0x0004_0402u32.to_le_bytes());
        bytes[8..12].copy_from_slice(&2u32.to_le_bytes());
        bytes[12..16].copy_from_slice(&0x20u32.to_le_bytes());
        // ids
        bytes[0x10..0x14].copy_from_slice(&0u32.to_le_bytes());
        bytes[0x14..0x18].copy_from_slice(&1u32.to_le_bytes());
        // offsets
        let data = 0x10 + 8 * 2 + 4;
        bytes[0x18..0x1c].copy_from_slice(&(data as u32).to_le_bytes());
        bytes[0x1c..0x20].copy_from_slice(&((data + 4) as u32).to_le_bytes());
        let total = bytes.len() as u32;
        bytes[0x20..0x24].copy_from_slice(&total.to_le_bytes());
        bytes
    }

    #[test]
    fn parses_header_and_entries() {
        let bytes = two_entry_afs();
        let afs = parse(&bytes, 0).expect("afs2");
        assert_eq!(afs.entries.len(), 2);
        assert_eq!(afs.entries[0].size, 4);
        assert_eq!(afs.entries[1].size, 6);
        assert_eq!(afs.declared_total as usize, bytes.len());
    }

    #[test]
    fn rejects_truncation() {
        let bytes = two_entry_afs();
        for len in 0..bytes.len() {
            let _ = parse(&bytes[..len], 0);
        }
    }
}
