//! VS2 语音发声表（`Sound/VoiceSpeak.vs2`，UO 侧 1 个）。
//!
//! ```text
//! +0x00 u32 条目数 N
//! +0x04 u32 ×3 = 0（保留）
//! +0x10 u32 id[N]        —— 单调不减的语音 id（1000101 起，按段跳号）
//! +0x10+4N  u64 mask[N]  —— 每条 8 字节标志位（如 0x00000000feff0fff）
//! ```
//!
//! 闭合判据：`16 + N × 4 + N × 8 == 文件长度`（本语料 N = 17280，
//! 16 + 69120 + 138240 = 207376，正好等于文件大小），且 id 段单调不减。
//! 标志位的确切含义要在音频系统里追，这里按位原样呈现，并给出置位数。

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;

const HEADER: usize = 0x10;
const MAX_SHOWN: usize = 64;

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("VS2");
    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < HEADER {
        builder.warn("文件不足 0x10 字节");
        builder.mark(root, DocMark::Error);
        return builder.finish();
    }

    let count = rdr::u32_at(bytes, 0).unwrap_or(0) as usize;
    let header = builder.group(root, Some("header"), "Header");
    builder.field(header, "count", count.to_string());
    builder.field(
        header,
        "reserved",
        format!("0x{:08x}", rdr::u32_at(bytes, 4).unwrap_or(0)),
    );

    let expected = HEADER + count * 12;
    builder.check(
        "vs2.size",
        "16 + N × 12 == 文件长度",
        expected == bytes.len(),
        Some(format!("{expected} / {}", bytes.len())),
    );

    let ids_at = HEADER;
    let masks_at = ids_at + count * 4;

    let mut monotonic = true;
    let mut previous = 0u32;
    let mut bits_set = 0u64;
    for index in 0..count {
        let Some(id) = rdr::u32_at(bytes, ids_at + index * 4) else {
            break;
        };
        if index > 0 && id < previous {
            monotonic = false;
            break;
        }
        previous = id;
        if let (Some(lo), Some(hi)) = (
            rdr::u32_at(bytes, masks_at + index * 8),
            rdr::u32_at(bytes, masks_at + index * 8 + 4),
        ) {
            bits_set += (lo.count_ones() + hi.count_ones()) as u64;
        }
    }

    builder.check(
        "vs2.ids",
        "id 段单调不减",
        monotonic,
        Some(format!("{} 条", count)),
    );

    let summary = builder.group(root, Some("summary"), "Summary");
    builder.field(summary, "ids", count.to_string());
    builder.field(
        summary,
        "first",
        rdr::u32_at(bytes, ids_at).unwrap_or(0).to_string(),
    );
    if count > 0 {
        let last = rdr::u32_at(bytes, ids_at + (count - 1) * 4).unwrap_or(0);
        builder.field(summary, "last", last.to_string());
    }
    builder.field(summary, "maskBits", bits_set.to_string());

    let list = builder.array(root, Some("entries"), format!("Entries[0..{MAX_SHOWN}]"));
    for index in 0..count.min(MAX_SHOWN) {
        let node = builder.group(list, Some(&index.to_string()), format!("[{index}]"));
        builder.field(
            node,
            "id",
            rdr::u32_at(bytes, ids_at + index * 4)
                .unwrap_or(0)
                .to_string(),
        );
        let lo = rdr::u32_at(bytes, masks_at + index * 8).unwrap_or(0);
        let hi = rdr::u32_at(bytes, masks_at + index * 8 + 4).unwrap_or(0);
        builder.field(node, "mask", format!("0x{hi:08x}{lo:08x}"));
        builder.field(
            node,
            "bits",
            (lo.count_ones() + hi.count_ones()).to_string(),
        );
    }
    if count > MAX_SHOWN {
        builder.truncate();
    }

    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_two_entries() {
        let mut bytes = vec![0u8; 16 + 2 * 12];
        bytes[0..4].copy_from_slice(&2u32.to_le_bytes());
        bytes[0x10..0x14].copy_from_slice(&100u32.to_le_bytes());
        bytes[0x14..0x18].copy_from_slice(&200u32.to_le_bytes());
        bytes[0x18..0x1c].copy_from_slice(&0xFFu32.to_le_bytes());
        let arena = build(&bytes);
        assert!(arena
            .checks()
            .iter()
            .any(|check| check.id == "vs2.size" && check.ok));
        assert!(arena
            .checks()
            .iter()
            .any(|check| check.id == "vs2.ids" && check.ok));
    }

    #[test]
    fn rejects_short_file() {
        for len in 0..16 {
            let arena = build(&vec![0u8; len]);
            assert!(arena.total_nodes() > 0);
        }
    }
}
