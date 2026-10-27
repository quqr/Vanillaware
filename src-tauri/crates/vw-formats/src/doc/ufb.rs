//! UFB 大地图图层装配（`FieldBG/Field.ufb`）。
//!
//! 文件 = 头（0x88）+ 记录区 + 网格区 + F 记录 + u32 表 + 字符串池，五段**首尾相接**，
//! 所以「解析到 EOF 且 slack == 0」是这条链路的硬校验。
//!
//! ```text
//! +0x40 u32 记录数 A      +0x44 B      +0x48 C      +0x4c D      +0x50 E
//! +0x54 u32 网格数        +0x58 F 记录数 +0x5c u32 表数
//! +0x60 u32 资源对数      +0x64 采样点名 +0x68 G 名   +0x6c 名数
//!
//! 记录 @0x88 × A：
//!   +0x00 .. +0x20 头（末 4 字节：u16 层数、u8 扩展块数、u8 组数）
//!   +0x24 u8[层数 × 0x40] 层描述
//!   ...   u8[扩展块数 × 0x90]
//!   ...   u8[组数 × { cnt(在 +3), cnt × 0xF0 }]
//!
//! 每个网格：+0x24 i32 x0,y0,x1,y1；+0x3c 起 (x1-x0+1)(y1-y0+1) × u32；共 0x3c + w*h*4
//! F 记录 ×0x18：u16 ×4 + f32 ×4
//! u32 表：u32 × 表数
//! 字符串池：资源对 ×2 串 → 采样名 → G 名 → 名（全部 NUL 结尾）
//! ```
//!
//! 依据是解出的 `FUN_710020f530`（解析）+ `FUN_71000288f0`（记录步长）+
//! `FUN_7100028aa0`（网格步长），147 MB 全文件 slack = 0。

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
        builder.warn("文件不足 0x88 字节");
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
            record_error = Some(format!("记录[{index}] @0x{cursor:x} 越界"));
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
            grid_error = Some(format!("网格[{index}] @0x{cursor:x} 头越界"));
            break;
        };
        let payload = w.saturating_mul(h).saturating_mul(4);
        if cursor + 0x3c + payload > bytes.len() {
            grid_error = Some(format!("网格[{index}] @0x{cursor:x} 数据越界"));
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

    // 字符串池：四段，任何一段读不出来就停在这里报 slack。
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
        "记录数 == 头声明",
        records_end != 0 && record_error.is_none(),
        Some(format!("{rec_count} 条，结束 0x{records_end:x}")),
    );
    builder.check(
        "ufb.grids",
        "网格数 == 头声明",
        grid_error.is_none(),
        Some(format!("{grid_count} 个，结束 0x{grids_end:x}")),
    );
    builder.check(
        "ufb.pairs",
        "资源对数 == 头声明",
        pairs.len() == pair_count,
        Some(format!("{}/{}", pairs.len(), pair_count)),
    );
    builder.check(
        "ufb.slack",
        "字符串池末尾 == 文件长度（slack = 0）",
        strings_end == bytes.len(),
        Some(format!(
            "结束 0x{strings_end:x} / 文件 0x{:x}，slack {}",
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
            "记录 → 0x{records_end:x} · 网格 → 0x{grids_end:x} · F → 0x{f_end:x} · u32 → 0x{words_end:x} · 资源对 → 0x{pairs_end:x} · 采样名 → 0x{samples_end:x} · G 名 → 0x{gnames_end:x}"
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
            "Strings ({} 采样名 · {} G 名 · {} 名)",
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

/// 单条记录的步长与三个计数；任何越界返回 `None`。
fn record_size(bytes: &[u8], at: usize) -> Option<(usize, u16, u8, u8)> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_short_file() {
        for len in 0..0x90 {
            let arena = build(&vec![0u8; len]);
            assert!(arena.total_nodes() > 0);
        }
    }

    #[test]
    fn record_size_counts_layers_and_groups() {
        let mut bytes = vec![0u8; 0x400];
        bytes[0x20..0x22].copy_from_slice(&1u16.to_le_bytes());
        bytes[0x22] = 1;
        bytes[0x23] = 1;
        let after = 0x24 + 0x40 + 0x90;
        bytes[after + 3] = 2;
        let (size, layers, ext, groups) = record_size(&bytes, 0).expect("record");
        assert_eq!((layers, ext, groups), (1, 1, 1));
        assert_eq!(size, after + 2 * 0xF0 + 4);
    }
}
