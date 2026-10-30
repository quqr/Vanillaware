use super::arena::{Builder, DocArena, DocMark};
use super::rdr;
use super::text;

const HEADER: usize = 0x58;
const MAX_RECORDS: u32 = 4096;

const STRIDES: [usize; 7] = [0x58, 0x20, 0x0c, 0x18, 0x04, 0x01, 0x0c];

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("CTB");

    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < HEADER {
        builder.warn("file is shorter than 0x58 bytes");
        builder.mark(root, DocMark::Error);
        return builder.finish();
    }

    let counts: [u32; 7] = std::array::from_fn(|index| rdr::u32_at(bytes, index * 4).unwrap_or(0));
    let build_tag = rdr::u32_at(bytes, 0x1c).unwrap_or(0);
    let offsets: [u64; 7] =
        std::array::from_fn(|index| rdr::u64_at(bytes, 0x20 + index * 8).unwrap_or(0));

    let header = builder.group(root, Some("header"), "Header");
    const NAMES: [&str; 7] = ["clips", "sec1", "sec2", "sec3", "sec4", "sec5", "sec6"];
    for (index, name) in NAMES.iter().enumerate() {
        builder.field(header, name, counts[index].to_string());
    }
    builder.field(header, "buildTag", format!("0x{build_tag:08x}"));
    for (index, offset) in offsets.iter().enumerate() {
        builder.field(header, &format!("off{index}"), format!("0x{offset:x}"));
    }

    for index in 0..7 {
        let count = counts[index];
        let offset = offsets[index];
        let stride = STRIDES[index];
        let name = NAMES[index];

        let end = offset.saturating_add(count as u64 * stride as u64);
        let size_ok = offset == 0 || end <= bytes.len() as u64;
        builder.check(
            &format!("ctb.{name}"),
            format!("{name} range lies inside the file"),
            size_ok,
            Some(format!("off={offset:#x} count={count} stride={stride:#x}")),
        );
        if !size_ok {
            builder.warn(format!("{name} range out of bounds"));
        }

        match index {
            0 => build_clips(&mut builder, root, bytes, offset, count),
            2 => build_sec2(&mut builder, root, bytes, offset, count),
            3 => build_sec3(&mut builder, root, bytes, offset, count),
            4 => build_sec4(&mut builder, root, bytes, offset, count),
            6 => build_sec6(&mut builder, root, bytes, offset, count),
            1 | 5 if offset != 0 && count > 0 => {
                let group =
                    builder.group(root, Some(name), format!("{name} ({count} × {stride:#x})"));
                builder.bytes(
                    group,
                    Some("raw"),
                    "raw",
                    offset,
                    count as u64 * stride as u64,
                );
            }
            _ => {}
        }
    }

    builder.finish()
}

fn build_clips(builder: &mut Builder, root: u32, bytes: &[u8], offset: u64, count: u32) {
    if offset == 0 {
        return;
    }
    let array = builder.array(root, Some("clips"), format!("Clips ({count})"));
    for index in 0..count.min(MAX_RECORDS) {
        let at = offset as usize + index as usize * 0x58;
        let Some(_) = bytes.get(at..at + 0x58) else {
            break;
        };
        let group = builder.group(array, Some(&index.to_string()), format!("clip[{index}]"));
        builder.field(group, "name", text::utf8_or_sjis(&bytes[at..at + 0x40]));
        for (field, delta) in [
            ("0x40", 0x40),
            ("0x44", 0x44),
            ("0x48", 0x48),
            ("0x4c", 0x4c),
            ("0x50", 0x50),
        ] {
            builder.field(
                group,
                field,
                rdr::u32_at(bytes, at + delta).unwrap_or(0).to_string(),
            );
        }
    }
    if count > MAX_RECORDS {
        builder.truncate();
    }
}

fn build_sec2(builder: &mut Builder, root: u32, bytes: &[u8], offset: u64, count: u32) {
    if offset == 0 {
        return;
    }
    let array = builder.array(root, Some("sec2"), format!("sec2 ({count})"));
    for index in 0..count.min(MAX_RECORDS) {
        let at = offset as usize + index as usize * 0x0c;
        let Some(_) = bytes.get(at..at + 0x0c) else {
            break;
        };
        let group = builder.group(array, Some(&index.to_string()), format!("sec2[{index}]"));
        builder.field(
            group,
            "seq",
            rdr::u32_at(bytes, at).unwrap_or(0).to_string(),
        );
        builder.field(
            group,
            "a",
            rdr::u16_at(bytes, at + 4).unwrap_or(0).to_string(),
        );
        builder.field(
            group,
            "b",
            rdr::u16_at(bytes, at + 6).unwrap_or(0).to_string(),
        );
        builder.field(
            group,
            "pad",
            rdr::u32_at(bytes, at + 8).unwrap_or(0).to_string(),
        );
    }
    if count > MAX_RECORDS {
        builder.truncate();
    }
}

fn build_sec3(builder: &mut Builder, root: u32, bytes: &[u8], offset: u64, count: u32) {
    if offset == 0 {
        return;
    }
    let array = builder.array(root, Some("sec3"), format!("sec3 ({count} × 6f)"));
    for index in 0..count.min(MAX_RECORDS) {
        let at = offset as usize + index as usize * 0x18;
        let Some(_) = bytes.get(at..at + 0x18) else {
            break;
        };
        let key = format!("[{index}]");
        let value = (0..6)
            .map(|slot| {
                rdr::f32_at(bytes, at + slot * 4)
                    .map(|value| format!("{value:.4}"))
                    .unwrap_or_else(|| "—".into())
            })
            .collect::<Vec<_>>()
            .join(", ");
        builder.field(array, &key, value);
    }
    if count > MAX_RECORDS {
        builder.truncate();
    }
}

fn build_sec4(builder: &mut Builder, root: u32, bytes: &[u8], offset: u64, count: u32) {
    if offset == 0 {
        return;
    }
    let array = builder.array(root, Some("sec4"), format!("sec4 ({count} u32)"));
    for index in 0..count.min(MAX_RECORDS) {
        let at = offset as usize + index as usize * 4;
        let Some(value) = rdr::u32_at(bytes, at) else {
            break;
        };
        let key = format!("[{index}]");
        builder.field(array, &key, value.to_string());
    }
    if count > MAX_RECORDS {
        builder.truncate();
    }
}

fn build_sec6(builder: &mut Builder, root: u32, bytes: &[u8], offset: u64, count: u32) {
    if offset == 0 {
        return;
    }
    let array = builder.array(root, Some("sec6"), format!("sec6 ({count} × 3f)"));
    for index in 0..count.min(MAX_RECORDS) {
        let at = offset as usize + index as usize * 0x0c;
        let Some(_) = bytes.get(at..at + 0x0c) else {
            break;
        };
        let key = format!("[{index}]");
        let value = (0..3)
            .map(|slot| {
                rdr::f32_at(bytes, at + slot * 4)
                    .map(|value| format!("{value:.4}"))
                    .unwrap_or_else(|| "—".into())
            })
            .collect::<Vec<_>>()
            .join(", ");
        builder.field(array, &key, value);
    }
    if count > MAX_RECORDS {
        builder.truncate();
    }
}
