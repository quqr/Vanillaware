//! ```text
//! +0x00  "CRID"
//! ```

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;
use super::utf;

const CHUNK_HEADER: usize = 0x20;
const MAX_CHUNKS_SHOWN: usize = 96;

pub struct Chunk {
    pub kind: String,
    pub offset: usize,
    pub size: usize,
}

pub struct Header {
    pub header_size: usize,
    pub version: u32,
    pub chunks: Vec<Chunk>,
    pub end: usize,
}

pub fn parse(bytes: &[u8]) -> Option<Header> {
    if bytes.get(..4)? != b"CRID" {
        return None;
    }
    let header_size = rdr::u32_be_at(bytes, 4)? as usize + 8;
    let version = rdr::u32_be_at(bytes, 8)?;
    if header_size < 0x20 || header_size > bytes.len() {
        return None;
    }

    let mut chunks = Vec::new();
    let mut cursor = header_size;
    while cursor + 8 <= bytes.len() {
        let magic = bytes.get(cursor..cursor + 4)?;
        if magic[0] != b'@' {
            return None;
        }
        let kind = String::from_utf8_lossy(magic).to_string();
        let size = rdr::u32_be_at(bytes, cursor + 4)? as usize;
        if size < CHUNK_HEADER || cursor + size > bytes.len() {
            return None;
        }
        chunks.push(Chunk {
            kind,
            offset: cursor,
            size,
        });
        cursor += size.div_ceil(CHUNK_HEADER) * CHUNK_HEADER;
        if chunks.len() > 1_000_000 {
            return None;
        }
    }

    Some(Header {
        header_size,
        version,
        chunks,
        end: cursor,
    })
}

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("USM");
    builder.field(root, "size", bytes.len().to_string());

    let Some(header) = parse(bytes) else {
        builder.warn("not USM: bad CRID header or the chunk chain does not close");
        builder.mark(root, DocMark::Error);
        return builder.finish();
    };

    let mut counts: Vec<(String, usize)> = Vec::new();
    for chunk in &header.chunks {
        match counts.iter_mut().find(|(kind, _)| kind == &chunk.kind) {
            Some((_, count)) => *count += 1,
            None => counts.push((chunk.kind.clone(), 1)),
        }
    }

    let head = builder.group(root, Some("header"), "Header");
    builder.field(head, "magic", "CRID");
    builder.field(head, "headerSize", format!("0x{:x}", header.header_size));
    builder.field(head, "version", format!("0x{:08x}", header.version));
    builder.field(head, "chunks", header.chunks.len().to_string());

    if let Some(table) = utf::parse_table(bytes, 0x20) {
        let meta = builder.group(root, Some("metadata"), "Metadata");
        builder.field(meta, "columns", table.columns.len().to_string());
        builder.field(meta, "rows", table.row_count.to_string());
        if table.row_count >= 1 && table.computed_width() == table.row_width {
            let at = table.rows_at();
            let row = builder.group(meta, Some("row0"), "row[0]");
            let mut cursor = at;
            for column in &table.columns {
                if !column.row_storage {
                    continue;
                }
                let value = utf::cell_text(bytes, &table, cursor, column);
                if !column.name.is_empty() {
                    builder.field(row, &column.name.clone(), value);
                }
                cursor += column.size;
            }
        }
    }

    let summary = builder.group(root, Some("summary"), "Summary");
    for (kind, count) in &counts {
        builder.field(summary, kind, count.to_string());
    }
    builder.field(summary, "end", format!("0x{:x}", header.end));

    builder.check(
        "usm.chain",
        "the chunk chain ends at the file length",
        header.end == bytes.len(),
        Some(format!("{:#x} / {:#x}", header.end, bytes.len())),
    );
    builder.check(
        "usm.header",
        "header length lies inside the file and is at least 0x20",
        header.header_size >= 0x20 && header.header_size <= bytes.len(),
        Some(format!("{:#x}", header.header_size)),
    );

    let list = builder.array(
        root,
        Some("chunks"),
        format!("Chunks ({})", header.chunks.len()),
    );
    let shown = header.chunks.len().min(MAX_CHUNKS_SHOWN);
    for (index, chunk) in header.chunks.iter().take(shown).enumerate() {
        let node = builder.group(
            list,
            Some(&index.to_string()),
            format!("[{index}] {}", chunk.kind),
        );
        builder.field(node, "kind", chunk.kind.clone());
        builder.field(node, "offset", format!("0x{:x}", chunk.offset));
        builder.field(node, "size", chunk.size.to_string());
        builder.field(node, "payload", (chunk.size - CHUNK_HEADER).to_string());
    }
    if header.chunks.len() > shown {
        builder.field(root, "chunksShown", shown.to_string());
        builder.truncate();
    }

    builder.finish()
}
