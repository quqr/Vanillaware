//! ```text
//! ```

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;

const HEADER: usize = 0x10;
const MAX_SHOWN: usize = 64;

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("VS2");
    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < HEADER {
        builder.warn("file is shorter than 0x10 bytes");
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
        "16 + N × 12 equals the file length",
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
        "the id section is non-decreasing",
        monotonic,
        Some(format!("{} entries", count)),
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
