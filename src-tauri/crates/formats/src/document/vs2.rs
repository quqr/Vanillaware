use kaitai::{BytesReader, KStruct};
use schema::gen::vs2::Vs2;

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
    let sized = expected == bytes.len();
    builder.check(
        "vs2.size",
        "16 + N × 12 equals the file length",
        sized,
        Some(format!("{expected} / {}", bytes.len())),
    );
    if !sized {
        builder.mark(root, DocMark::Error);
        return builder.finish();
    }

    let io = BytesReader::from(bytes);
    let parsed = match <Vs2 as KStruct>::read_into::<_, Vs2>(&io, None, None) {
        Ok(parsed) => parsed,
        Err(error) => {
            builder.warn(format!("kaitai parse error: {error:?}"));
            builder.mark(root, DocMark::Error);
            return builder.finish();
        }
    };

    let ids = parsed.ids();
    let masks = parsed.masks();

    let mut bits_set = 0u64;
    for mask in masks.iter() {
        bits_set += (*mask.low()).count_ones() as u64 + (*mask.high()).count_ones() as u64;
    }
    let monotonic = ids.windows(2).all(|pair| pair[0] <= pair[1]);

    builder.check(
        "vs2.ids",
        "the id section is non-decreasing",
        monotonic,
        Some(format!("{} entries", count)),
    );

    let summary = builder.group(root, Some("summary"), "Summary");
    builder.field(summary, "ids", count.to_string());
    builder.field(summary, "first", ids.first().copied().unwrap_or(0).to_string());
    if let Some(last) = ids.last() {
        builder.field(summary, "last", last.to_string());
    }
    builder.field(summary, "maskBits", bits_set.to_string());

    let list = builder.array(root, Some("entries"), format!("Entries[0..{MAX_SHOWN}]"));
    for (index, id) in ids.iter().take(MAX_SHOWN).enumerate() {
        let node = builder.group(list, Some(&index.to_string()), format!("[{index}]"));
        builder.field(node, "id", id.to_string());
        let Some(mask) = masks.get(index) else {
            continue;
        };
        let low = *mask.low();
        let high = *mask.high();
        builder.field(node, "mask", format!("0x{high:08x}{low:08x}"));
        builder.field(
            node,
            "bits",
            (low.count_ones() + high.count_ones()).to_string(),
        );
    }
    if count > MAX_SHOWN {
        builder.truncate();
    }

    builder.finish()
}
