//! ```text
//! +0x00  "AFS2"
//! ```

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
    pub id_width: usize,
}

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
        builder.warn("not AFS2: bad magic or offset table out of range");
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
        "total size field equals the file length",
        afs2.declared_total as usize == bytes.len(),
        Some(format!("{:#x} / {:#x}", afs2.declared_total, bytes.len())),
    );
    builder.check(
        "awb.monotonic",
        "offset table is strictly increasing",
        monotonic,
        Some(format!("{} entries", afs2.entries.len())),
    );
    builder.check(
        "awb.data_start",
        "first offset equals the end of the descriptor area",
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
        "last entry ends at the file length",
        last_end == bytes.len(),
        Some(format!("{last_end:#x} / {:#x}", bytes.len())),
    );
    builder.check(
        "awb.payload",
        "entry lengths add up to the payload area",
        total == bytes.len().saturating_sub(afs2.data_start),
        Some(format!("{total} bytes")),
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
