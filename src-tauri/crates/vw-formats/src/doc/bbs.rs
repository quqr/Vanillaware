//! UBBS 战斗背景装配单的结构视图（UO，152/152 闭合）。
//!
//! 依据 `uo_bbs.py`：头 0x338 + 9 个段偏移 + 名表/档位/物件/rec4..7/尾部表。

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;

const OFFSETS_AT: usize = 0x338;
const SEGMENTS_AT: usize = 0x380;
const MAX_RECORDS: u32 = 4096;

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("UBBS");

    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < SEGMENTS_AT {
        builder.warn("文件不足 0x380 字节");
        builder.mark(root, DocMark::Error);
        return builder.finish();
    }

    let magic = rdr::ascii(&bytes[..4]);
    let header = builder.group(root, Some("header"), "Header");
    builder.field(header, "magic", magic.clone());
    builder.field(header, "version", rdr::u32_at(bytes, 4).unwrap_or(0).to_string());
    builder.field(
        header,
        "headerSize",
        format!("0x{:x}", rdr::u32_at(bytes, 8).unwrap_or(0)),
    );
    builder.check(
        "bbs.magic",
        "魔数为 UBBS",
        magic == "UBBS",
        Some(format!("读到 {magic}")),
    );

    let counts = read_counts(bytes);
    let counts_group = builder.group(root, Some("counts"), "Counts");
    const COUNT_NAMES: [&str; 8] = [
        "mbs", "ftx", "grades", "objects", "rec4", "rec5", "rec6", "rec7",
    ];
    for (index, name) in COUNT_NAMES.iter().enumerate() {
        builder.field(counts_group, name, counts[index].to_string());
    }

    build_bounds(&mut builder, bytes);
    build_lights(&mut builder, bytes);

    let offsets = read_offsets(bytes);
    let offset_group = builder.group(root, Some("offsets"), "SegmentOffsets");
    for (index, offset) in offsets.iter().enumerate() {
        builder.field(offset_group, &format!("seg{index}"), format!("0x{offset:x}"));
    }

    let monotonic = offsets.windows(2).all(|pair| pair[0] <= pair[1]);
    builder.check(
        "bbs.offsets",
        "9 个段偏移单调不减",
        monotonic,
        Some(format!("{offsets:?}")),
    );

    let rec6_ok = offsets.get(7).zip(offsets.get(6)).is_none_or(|(end, start)| {
        *end >= *start && *end - *start == counts[6] as u64 * 0x18
    });
    builder.check(
        "bbs.rec6",
        "offs[7] − offs[6] == rec6_count × 0x18",
        rec6_ok,
        Some(format!("offs6={:#x} offs7={:#x}", offsets[6], offsets[7])),
    );

    build_names(&mut builder, root, bytes, offsets[0], counts[0], "mbsNames", "mbs");
    build_names(&mut builder, root, bytes, offsets[1], counts[1], "ftxNames", "ftx");
    build_grades(&mut builder, root, bytes, offsets[2], counts[2]);
    build_objects(&mut builder, root, bytes, offsets[3], counts[3]);
    build_rec4(&mut builder, root, bytes, offsets[4], counts[4]);
    build_rec5(&mut builder, root, bytes, offsets[5], counts[5]);
    build_rec6(&mut builder, root, bytes, offsets[6], counts[6]);
    build_rec7(&mut builder, root, bytes, offsets[7], counts[7]);

    let tail = builder.array(root, Some("tail"), "Tail");
    let mut end = offsets[8] as usize;
    let mut count = 0u32;
    while end + 16 <= bytes.len() && count < MAX_RECORDS {
        let group = builder.group(tail, Some(&count.to_string()), format!("tail[{count}]"));
        builder.field(group, "value", hex32(bytes, end));
        builder.field(group, "index", rdr::u16_at(bytes, end + 4).unwrap_or(0).to_string());
        builder.field(group, "tag", rdr::u16_at(bytes, end + 6).unwrap_or(0).to_string());
        builder.field(group, "one", rdr::u32_at(bytes, end + 8).unwrap_or(0).to_string());
        builder.field(group, "zero", rdr::u32_at(bytes, end + 12).unwrap_or(0).to_string());
        end += 16;
        count += 1;
    }
    if count == MAX_RECORDS {
        builder.truncate();
    }

    builder.check(
        "bbs.closed",
        "尾部表精确闭合到 EOF",
        end == bytes.len(),
        Some(format!("{end:#x} vs {:#x}", bytes.len())),
    );

    builder.finish()
}

fn read_counts(bytes: &[u8]) -> [u32; 8] {
    let mut counts = [0u32; 8];
    for (index, slot) in counts.iter_mut().enumerate() {
        *slot = rdr::u16_at(bytes, 0x20 + index * 2).unwrap_or(0) as u32;
    }
    counts
}

fn read_offsets(bytes: &[u8]) -> [u64; 9] {
    let mut offsets = [0u64; 9];
    for (index, slot) in offsets.iter_mut().enumerate() {
        *slot = rdr::u64_at(bytes, OFFSETS_AT + index * 8).unwrap_or(0);
    }
    offsets
}

fn build_bounds(builder: &mut Builder, bytes: &[u8]) {
    let bounds = builder.group(0, Some("bounds"), "Bounds");
    for (index, name) in ["bbox", "camera0", "camera1", "camera2", "camera3"].iter().enumerate() {
        let at = 0x40 + index * 0x10;
        builder.field(bounds, name, f32s(bytes, at, 4));
    }
    builder.field(bounds, "view", f32s(bytes, 0x90, 2));
    builder.field(bounds, "flagsA", rdr::hex(&bytes[0xa0..0xb0]));
    builder.field(bounds, "flagsB", rdr::hex(&bytes[0xb0..0xc0]));
}

fn build_lights(builder: &mut Builder, bytes: &[u8]) {
    let lights = builder.array(0, Some("lights"), "Lights (7)");
    for index in 0..7u32 {
        let base = 0xb4 + index as usize * 0x5c;
        let group = builder.group(lights, Some(&index.to_string()), format!("Light {index}"));
        builder.field(group, "id", hex32(bytes, base));
        builder.field(group, "f0", f32_at(bytes, base + 4));
        builder.field(group, "f1", f32_at(bytes, base + 8));
        builder.field(group, "colorA", rdr::hex(&bytes[base + 0x0c..base + 0x1c]));
        builder.field(group, "kind", rdr::u32_at(bytes, base + 0x1c).unwrap_or(0).to_string());
        builder.field(group, "f2", f32_at(bytes, base + 0x20));
        builder.field(group, "colorB", rdr::hex(&bytes[base + 0x24..base + 0x34]));
        builder.field(group, "f3", f32_at(bytes, base + 0x34));
        builder.field(group, "u38", hex32(bytes, base + 0x38));
        builder.field(group, "colorC", hex32(bytes, base + 0x3c));
        builder.field(group, "f4", f32_at(bytes, base + 0x40));
        builder.field(group, "f5", f32_at(bytes, base + 0x44));
        builder.field(group, "u48", hex32(bytes, base + 0x48));
        builder.field(group, "u4c", hex32(bytes, base + 0x4c));
        builder.field(group, "u50", hex32(bytes, base + 0x50));
        builder.field(group, "u54", hex32(bytes, base + 0x54));
        builder.field(group, "u58", hex32(bytes, base + 0x58));
    }
}

fn build_names(
    builder: &mut Builder,
    root: u32,
    bytes: &[u8],
    offset: u64,
    count: u32,
    key: &str,
    prefix: &str,
) {
    let array = builder.array(root, Some(key), format!("{prefix} names"));
    let shown = count.min(MAX_RECORDS);
    if count > shown {
        builder.truncate();
    }
    for index in 0..shown {
        let at = offset as usize + index as usize * 32;
        let Some(raw) = bytes.get(at..at + 32) else {
            builder.warn(format!("{prefix} 名表 {index} 越界"));
            break;
        };
        let name = slot_utf8(raw);
        let item_key = format!("[{index}]");
        builder.text(array, &item_key, format!("{prefix}[{index}]"), name, Some(at as u64));
    }
}

fn build_grades(builder: &mut Builder, root: u32, bytes: &[u8], offset: u64, count: u32) {
    let array = builder.array(root, Some("grades"), "Grades");
    for index in 0..count.min(MAX_RECORDS) {
        let at = offset as usize + index as usize * 16;
        let Some(_) = bytes.get(at..at + 16) else { break };
        let group = builder.group(array, Some(&index.to_string()), format!("grade[{index}]"));
        builder.field(group, "gain", f32s(bytes, at, 3));
        let mark = rdr::u32_at(bytes, at + 12).unwrap_or(0);
        builder.field(group, "mark", format!("0x{mark:08x}"));
        builder.field(group, "neutral", (mark == 0x19f701).to_string());
    }
    if count > MAX_RECORDS {
        builder.truncate();
    }
}

fn build_objects(builder: &mut Builder, root: u32, bytes: &[u8], offset: u64, count: u32) {
    let array = builder.array(root, Some("objects"), "Objects");
    for index in 0..count.min(MAX_RECORDS) {
        let at = offset as usize + index as usize * 0x48;
        let Some(_) = bytes.get(at..at + 0x48) else { break };
        let group = builder.group(array, Some(&index.to_string()), format!("object[{index}]"));
        builder.field(group, "mbs", rdr::u16_at(bytes, at).unwrap_or(0).to_string());
        builder.field(group, "ftx", rdr::u16_at(bytes, at + 2).unwrap_or(0).to_string());
        builder.field(group, "kind", rdr::u16_at(bytes, at + 4).unwrap_or(0).to_string());
        builder.field(group, "group", rdr::u16_at(bytes, at + 6).unwrap_or(0).to_string());
        builder.field(group, "pos", f32s(bytes, at + 8, 3));
        builder.field(group, "rot", f32_at(bytes, at + 0x1c));
        builder.field(group, "scale", f32s(bytes, at + 0x20, 2));
        builder.field(group, "tint", hex32(bytes, at + 0x28));
        builder.field(group, "color", hex32(bytes, at + 0x2c));
        builder.field(group, "flags", hex32(bytes, at + 0x30));
        builder.field(group, "priority", rdr::u32_at(bytes, at + 0x34).unwrap_or(0).to_string());
        builder.field(group, "size", f32_at(bytes, at + 0x38));
        builder.field(group, "fade", f32_at(bytes, at + 0x3c));
        builder.field(group, "pivot", f32_at(bytes, at + 0x40));
        builder.field(group, "id", hex32(bytes, at + 0x44));
    }
    if count > MAX_RECORDS {
        builder.truncate();
    }
}

fn build_rec4(builder: &mut Builder, root: u32, bytes: &[u8], offset: u64, count: u32) {
    let array = builder.array(root, Some("rec4"), "rec4");
    for index in 0..count.min(MAX_RECORDS) {
        let at = offset as usize + index as usize * 0x6c;
        let Some(_) = bytes.get(at..at + 0x6c) else { break };
        let group = builder.group(array, Some(&index.to_string()), format!("rec4[{index}]"));
        builder.field(group, "id", hex32(bytes, at));
        builder.field(group, "quad", quad(bytes, at + 4));
        builder.field(group, "colorsA", rdr::hex(&bytes[at + 0x24..at + 0x34]));
        builder.field(group, "colorsB", rdr::hex(&bytes[at + 0x34..at + 0x44]));
        builder.field(group, "frames", rdr::u32_at(bytes, at + 0x64).unwrap_or(0).to_string());
        builder.field(group, "dist", f32_at(bytes, at + 0x68));
    }
    if count > MAX_RECORDS {
        builder.truncate();
    }
}

fn build_rec5(builder: &mut Builder, root: u32, bytes: &[u8], offset: u64, count: u32) {
    let array = builder.array(root, Some("rec5"), "rec5");
    for index in 0..count.min(MAX_RECORDS) {
        let at = offset as usize + index as usize * 0x44;
        let Some(_) = bytes.get(at..at + 0x44) else { break };
        let group = builder.group(array, Some(&index.to_string()), format!("rec5[{index}]"));
        builder.field(group, "id", hex32(bytes, at));
        builder.field(group, "params", f32s(bytes, at + 4, 6));
        builder.field(group, "colorA", hex32(bytes, at + 0x1c));
        builder.field(group, "colorB", hex32(bytes, at + 0x20));
        builder.field(group, "dist", f32_at(bytes, at + 0x24));
        builder.field(group, "extra", f32s(bytes, at + 0x30, 5));
    }
    if count > MAX_RECORDS {
        builder.truncate();
    }
}

fn build_rec6(builder: &mut Builder, root: u32, bytes: &[u8], offset: u64, count: u32) {
    let array = builder.array(root, Some("rec6"), "rec6");
    for index in 0..count.min(MAX_RECORDS) {
        let at = offset as usize + index as usize * 0x18;
        let Some(_) = bytes.get(at..at + 0x18) else { break };
        let group = builder.group(array, Some(&index.to_string()), format!("rec6[{index}]"));
        builder.field(group, "slot", rdr::u16_at(bytes, at).unwrap_or(0).to_string());
        builder.field(group, "local", rdr::u16_at(bytes, at + 2).unwrap_or(0).to_string());
        builder.field(group, "pos", f32s(bytes, at + 4, 3));
        builder.field(group, "type", hex32(bytes, at + 0x10));
        builder.field(group, "next", hex32(bytes, at + 0x14));
    }
    if count > MAX_RECORDS {
        builder.truncate();
    }
}

fn build_rec7(builder: &mut Builder, root: u32, bytes: &[u8], offset: u64, count: u32) {
    let array = builder.array(root, Some("rec7"), "rec7");
    for index in 0..count.min(MAX_RECORDS) {
        let at = offset as usize + index as usize * 0x80;
        let Some(_) = bytes.get(at..at + 0x80) else { break };
        let group = builder.group(array, Some(&index.to_string()), format!("rec7[{index}]"));
        builder.field(group, "colors", rdr::hex(&bytes[at..at + 0x20]));
        builder.field(group, "id", hex32(bytes, at + 0x20));
        builder.field(group, "value", hex32(bytes, at + 0x24));
        builder.field(group, "params", f32s(bytes, at + 0x28, 11));
        builder.field(group, "quad", quad(bytes, at + 0x5c));
    }
    if count > MAX_RECORDS {
        builder.truncate();
    }
}

fn slot_utf8(raw: &[u8]) -> String {
    let end = raw.iter().position(|&byte| byte == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).into_owned()
}

fn f32_at(bytes: &[u8], at: usize) -> String {
    rdr::f32_at(bytes, at)
        .map(|value| format!("{value:.4}"))
        .unwrap_or_else(|| "—".into())
}

fn f32s(bytes: &[u8], at: usize, count: usize) -> String {
    (0..count)
        .map(|index| f32_at(bytes, at + index * 4))
        .collect::<Vec<_>>()
        .join(", ")
}

fn quad(bytes: &[u8], at: usize) -> String {
    let mut parts = Vec::with_capacity(4);
    for index in 0..4 {
        let offset = at + index * 8;
        parts.push(format!(
            "({}, {})",
            f32_at(bytes, offset),
            f32_at(bytes, offset + 4)
        ));
    }
    parts.join(" ")
}

fn hex32(bytes: &[u8], at: usize) -> String {
    rdr::u32_at(bytes, at)
        .map(|value| format!("0x{value:08x}"))
        .unwrap_or_else(|| "—".into())
}
