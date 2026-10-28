//! ```text
//! ```

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;
use super::text;

const HEADER: usize = 0x88;
const MAX_RECORDS_SHOWN: usize = 64;
const MAX_GRIDS_SHOWN: usize = 64;
const MAX_NAMES_SHOWN: usize = 64;

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("UFB");
    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < HEADER {
        builder.warn("file is shorter than 0x88 bytes");
        builder.mark(root, DocMark::Error);
        return builder.finish();
    }

    let rec_count = rdr::u32_at(bytes, 0x40).unwrap_or(0) as usize;
    let grid_count = rdr::u32_at(bytes, 0x54).unwrap_or(0) as usize;
    let f_count = rdr::u32_at(bytes, 0x58).unwrap_or(0) as usize;
    let word_count = rdr::u32_at(bytes, 0x5c).unwrap_or(0) as usize;
    let pair_count = rdr::u32_at(bytes, 0x60).unwrap_or(0) as usize;
    let sample_count = rdr::u32_at(bytes, 0x64).unwrap_or(0) as usize;
    let gname_count = rdr::u32_at(bytes, 0x68).unwrap_or(0) as usize;
    let name_count = rdr::u32_at(bytes, 0x6c).unwrap_or(0) as usize;

    let header = builder.group(root, Some("header"), "Header");
    builder.field(header, "records", rec_count.to_string());
    builder.field(header, "grids", grid_count.to_string());
    builder.field(header, "fRecords", f_count.to_string());
    builder.field(header, "words", word_count.to_string());
    builder.field(header, "pairs", pair_count.to_string());
    builder.field(header, "samples", sample_count.to_string());
    builder.field(header, "gNames", gname_count.to_string());
    builder.field(header, "names", name_count.to_string());

    let mut cursor = HEADER;
    let mut records = Vec::with_capacity(rec_count.min(MAX_RECORDS_SHOWN));
    let mut layers_total = 0u64;
    let mut ext_total = 0u64;
    let mut group_total = 0u64;
    let mut record_error = None;

    for index in 0..rec_count {
        let Some((size, layers, ext, groups)) = record_size(bytes, cursor) else {
            record_error = Some(format!("record[{index}] @0x{cursor:x} out of range"));
            break;
        };
        layers_total += layers as u64;
        ext_total += ext as u64;
        group_total += groups as u64;
        if records.len() < MAX_RECORDS_SHOWN {
            records.push((cursor, size, layers, ext, groups));
        }
        cursor += size;
    }
    let records_end = cursor;

    let mut grids = Vec::with_capacity(grid_count.min(MAX_GRIDS_SHOWN));
    let mut grid_error = None;
    for index in 0..grid_count {
        let Some((x0, y0, x1, y1, w, h)) = grid_extent(bytes, cursor) else {
            grid_error = Some(format!("grid[{index}] @0x{cursor:x} header out of range"));
            break;
        };
        let payload = w.saturating_mul(h).saturating_mul(4);
        if cursor + 0x3c + payload > bytes.len() {
            grid_error = Some(format!("grid[{index}] @0x{cursor:x} data out of range"));
            break;
        }
        if grids.len() < MAX_GRIDS_SHOWN {
            grids.push((cursor, x0, y0, x1, y1, w, h));
        }
        cursor += 0x3c + payload;
    }
    let grids_end = cursor;

    let mut f_records = Vec::new();
    for _ in 0..f_count {
        if cursor + 0x18 > bytes.len() {
            break;
        }
        let a = rdr::u16_at(bytes, cursor).unwrap_or(0);
        let b = rdr::u16_at(bytes, cursor + 2).unwrap_or(0);
        let c = rdr::u16_at(bytes, cursor + 4).unwrap_or(0);
        let d = rdr::u16_at(bytes, cursor + 6).unwrap_or(0);
        let floats: Vec<f32> = (0..4)
            .map(|index| rdr::f32_at(bytes, cursor + 8 + index * 4).unwrap_or(0.0))
            .collect();
        f_records.push((cursor, a, b, c, d, floats));
        cursor += 0x18;
    }
    let f_end = cursor;

    let mut words = Vec::with_capacity(word_count.min(256));
    for index in 0..word_count {
        let Some(value) = rdr::u32_at(bytes, cursor + index * 4) else {
            break;
        };
        if index < 256 {
            words.push(value);
        }
    }
    cursor += word_count * 4;
    let words_end = cursor;

    let mut pairs = Vec::new();
    for _ in 0..pair_count {
        let Some((left, after)) = read_string(bytes, cursor) else {
            break;
        };
        let Some((right, next)) = read_string(bytes, after) else {
            break;
        };
        pairs.push((left, right));
        cursor = next;
    }
    let pairs_end = cursor;

    let mut samples = Vec::new();
    for _ in 0..sample_count {
        let Some((name, next)) = read_string(bytes, cursor) else {
            break;
        };
        samples.push(name);
        cursor = next;
    }
    let samples_end = cursor;

    let mut gnames = Vec::new();
    for _ in 0..gname_count {
        let Some((name, next)) = read_string(bytes, cursor) else {
            break;
        };
        gnames.push(name);
        cursor = next;
    }
    let gnames_end = cursor;

    let mut names = Vec::new();
    for _ in 0..name_count {
        let Some((name, next)) = read_string(bytes, cursor) else {
            break;
        };
        names.push(name);
        cursor = next;
    }
    let strings_end = cursor;

    if let Some(ref message) = record_error {
        builder.warn(message.clone());
        builder.mark(root, DocMark::Error);
    }
    if let Some(ref message) = grid_error {
        builder.warn(message.clone());
        builder.mark(root, DocMark::Error);
    }

    builder.check(
        "ufb.records",
        "record count matches the header",
        records_end != 0 && record_error.is_none(),
        Some(format!("{rec_count} records, end 0x{records_end:x}")),
    );
    builder.check(
        "ufb.grids",
        "grid count matches the header",
        grid_error.is_none(),
        Some(format!("{grid_count} grids, end 0x{grids_end:x}")),
    );
    builder.check(
        "ufb.pairs",
        "resource pair count matches the header",
        pairs.len() == pair_count,
        Some(format!("{}/{}", pairs.len(), pair_count)),
    );
    builder.check(
        "ufb.slack",
        "the string pool ends at the file length (slack = 0)",
        strings_end == bytes.len(),
        Some(format!(
            "ends at 0x{strings_end:x} / file 0x{:x}, slack {}",
            bytes.len(),
            bytes.len() as i64 - strings_end as i64
        )),
    );

    let summary = builder.group(root, Some("summary"), "Summary");
    builder.field(summary, "layers", layers_total.to_string());
    builder.field(summary, "extBlocks", ext_total.to_string());
    builder.field(summary, "groups", group_total.to_string());
    builder.field(
        summary,
        "layout",
        format!(
            "records -> 0x{records_end:x} · grids -> 0x{grids_end:x} · F -> 0x{f_end:x} · u32 -> 0x{words_end:x} · pairs -> 0x{pairs_end:x} · sample names -> 0x{samples_end:x} · G names -> 0x{gnames_end:x}"
        ),
    );

    let list = builder.array(root, Some("records"), format!("Records ({rec_count})"));
    for (index, (offset, size, layers, ext, groups)) in records.iter().enumerate() {
        let node = builder.group(
            list,
            Some(&index.to_string()),
            format!("[{index}] @0x{offset:x}"),
        );
        builder.field(node, "offset", format!("0x{offset:x}"));
        builder.field(node, "size", format!("0x{size:x}"));
        builder.field(node, "layers", layers.to_string());
        builder.field(node, "extBlocks", ext.to_string());
        builder.field(node, "groups", groups.to_string());
        builder.bytes(node, Some("bytes"), "record", *offset as u64, *size as u64);
    }
    if rec_count > records.len() {
        builder.field(root, "recordsShown", records.len().to_string());
        builder.truncate();
    }

    let list = builder.array(root, Some("grids"), format!("Grids ({grid_count})"));
    for (index, (offset, x0, y0, x1, y1, w, h)) in grids.iter().enumerate() {
        let node = builder.group(list, Some(&index.to_string()), format!("[{index}] {w}×{h}"));
        builder.field(node, "offset", format!("0x{offset:x}"));
        builder.field(node, "x", format!("{x0} .. {x1}"));
        builder.field(node, "y", format!("{y0} .. {y1}"));
        builder.field(node, "cells", (w * h).to_string());
        builder.bytes(
            node,
            Some("cells"),
            "cells",
            (*offset + 0x3c) as u64,
            (w * h * 4) as u64,
        );
    }
    if grid_count > grids.len() {
        builder.field(root, "gridsShown", grids.len().to_string());
        builder.truncate();
    }

    if !f_records.is_empty() {
        let list = builder.array(
            root,
            Some("fRecords"),
            format!("F Records ({})", f_records.len()),
        );
        for (index, (offset, a, b, c, d, floats)) in f_records.iter().enumerate() {
            let node = builder.group(list, Some(&index.to_string()), format!("[{index}]"));
            builder.field(node, "offset", format!("0x{offset:x}"));
            builder.field(node, "a", a.to_string());
            builder.field(node, "b", b.to_string());
            builder.field(node, "c", c.to_string());
            builder.field(node, "d", d.to_string());
            builder.field(
                node,
                "f32",
                floats
                    .iter()
                    .map(|value| format!("{value:.3}"))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
    }

    if !words.is_empty() {
        let list = builder.array(root, Some("words"), format!("u32 ({word_count})"));
        for (index, value) in words.iter().enumerate() {
            builder.field(list, &index.to_string(), value.to_string());
        }
    }

    if !pairs.is_empty() {
        let list = builder.array(
            root,
            Some("pairs"),
            format!("Resource Pairs ({})", pairs.len()),
        );
        for (index, (left, right)) in pairs.iter().enumerate() {
            let node = builder.group(list, Some(&index.to_string()), format!("[{index}]"));
            builder.text(node, "model", "model", left.clone(), None);
            builder.text(node, "texture", "texture", right.clone(), None);
        }
    }

    let names_group = builder.group(
        root,
        Some("strings"),
        format!(
            "Strings ({} sample names · {} G names · {} names)",
            samples.len(),
            gnames.len(),
            names.len()
        ),
    );
    append_strings(
        &mut builder,
        names_group,
        "samples",
        "Sample Names",
        &samples,
    );
    append_strings(&mut builder, names_group, "gnames", "G Names", &gnames);
    append_strings(&mut builder, names_group, "names", "Names", &names);

    builder.finish()
}

fn append_strings(builder: &mut Builder, parent: u32, key: &str, title: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    let list = builder.array(parent, Some(key), format!("{title} ({})", items.len()));
    for (index, text) in items.iter().take(MAX_NAMES_SHOWN).enumerate() {
        builder.text(
            list,
            &index.to_string(),
            format!("[{index}]"),
            text.clone(),
            None,
        );
    }
}

pub fn record_size(bytes: &[u8], at: usize) -> Option<(usize, u16, u8, u8)> {
    let layers = rdr::u16_at(bytes, at + 0x20)?;
    let ext = rdr::u8_at(bytes, at + 0x22)?;
    let groups = rdr::u8_at(bytes, at + 0x23)?;
    let mut size = 0x24 + layers as usize * 0x40 + ext as usize * 0x90;
    let mut cursor = at + 0x24 + layers as usize * 0x40 + ext as usize * 0x90;
    if cursor > bytes.len() {
        return None;
    }
    for _ in 0..groups {
        let count = rdr::u8_at(bytes, cursor + 3)? as usize;
        let step = count * 0xF0 + 4;
        size += step;
        cursor += step;
        if cursor > bytes.len() {
            return None;
        }
    }
    if at + size > bytes.len() {
        return None;
    }
    Some((size, layers, ext, groups))
}

fn grid_extent(bytes: &[u8], at: usize) -> Option<(i32, i32, i32, i32, usize, usize)> {
    let x0 = rdr::i32_at(bytes, at + 0x24)?;
    let y0 = rdr::i32_at(bytes, at + 0x28)?;
    let x1 = rdr::i32_at(bytes, at + 0x2c)?;
    let y1 = rdr::i32_at(bytes, at + 0x30)?;
    if x1 < x0 || y1 < y0 {
        return None;
    }
    let w = (x1 - x0 + 1) as usize;
    let h = (y1 - y0 + 1) as usize;
    if w == 0 || h == 0 || w > 1 << 20 || h > 1 << 20 {
        return None;
    }
    Some((x0, y0, x1, y1, w, h))
}

fn read_string(bytes: &[u8], at: usize) -> Option<(String, usize)> {
    if at >= bytes.len() {
        return None;
    }
    let rest = &bytes[at..];
    let end = rest
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(rest.len());
    Some((text::utf8_or_sjis(&rest[..end]), at + end + 1))
}
