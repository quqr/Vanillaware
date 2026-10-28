//! MPD 四边形绘制记录：0x20 头 + count × 0xA0 记录。

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;
use super::text;

const HEADER: usize = 0x20;
const RECORD: usize = 0xa0;
const MAX_RECORDS: u32 = 4096;

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("MPD");

    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < HEADER {
        builder.warn("文件不足 0x20 字节");
        builder.mark(root, DocMark::Error);
        return builder.finish();
    }

    let magic_ok = &bytes[..4] == b"MPD\0";
    let payload = rdr::u32_at(bytes, 4).unwrap_or(0);
    let data_offset = rdr::u32_at(bytes, 8).unwrap_or(0);
    let version = rdr::u32_at(bytes, 0x10).unwrap_or(0);
    let count = rdr::u32_at(bytes, 0x14).unwrap_or(0);

    let header = builder.group(root, Some("header"), "Header");
    builder.field(header, "magic", rdr::ascii(&bytes[..4]));
    builder.field(header, "payloadSize", payload.to_string());
    builder.field(header, "dataOffset", format!("0x{data_offset:x}"));
    builder.field(header, "version", version.to_string());
    builder.field(header, "recordCount", count.to_string());

    builder.check(
        "mpd.magic",
        "魔数为 MPD\\0",
        magic_ok,
        Some(rdr::hex(&bytes[..4])),
    );
    builder.check(
        "mpd.size",
        "0x20 + count × 0xA0 == 文件长",
        HEADER as u64 + count as u64 * RECORD as u64 == bytes.len() as u64,
        Some(format!("{count} × 0xA0")),
    );
    builder.check(
        "mpd.payload",
        "payloadSize == count × 0xA0",
        payload as u64 == count as u64 * RECORD as u64,
        None,
    );
    builder.check(
        "mpd.offset",
        "dataOffset == 0x20",
        data_offset as usize == HEADER,
        Some(format!("{data_offset:#x}")),
    );

    let records = builder.array(root, Some("records"), format!("Records ({count})"));
    let shown = count.min(MAX_RECORDS);
    if count > shown {
        builder.truncate();
    }

    for index in 0..shown {
        let at = data_offset as usize + index as usize * RECORD;
        let Some(_) = bytes.get(at..at + RECORD) else { break };
        let group = builder.group(records, Some(&index.to_string()), format!("record[{index}]"));
        builder.field(group, "flags", format!("0x{:x}", rdr::u32_at(bytes, at).unwrap_or(0)));
        builder.field(group, "texIndex", rdr::u16_at(bytes, at + 4).unwrap_or(0).to_string());
        builder.field(group, "unk06", rdr::u16_at(bytes, at + 6).unwrap_or(0).to_string());

        for vertex in 0..4usize {
            let base = at + 8 + vertex * 0x18;
            let key = format!("v{vertex}");
            let value = format!(
                "({}, {}, {}) rgba={} uv=({}, {})",
                f32_at(bytes, base),
                f32_at(bytes, base + 4),
                f32_at(bytes, base + 8),
                hex32(bytes, base + 0x0c),
                f32_at(bytes, base + 0x10),
                f32_at(bytes, base + 0x14),
            );
            builder.field(group, &key, value);
        }

        builder.field(
            group,
            "name",
            text::utf8_or_sjis(&bytes[at + 0x68..at + 0x68 + 0x38]),
        );
    }

    builder.finish()
}

fn f32_at(bytes: &[u8], at: usize) -> String {
    rdr::f32_at(bytes, at)
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "—".into())
}

fn hex32(bytes: &[u8], at: usize) -> String {
    rdr::u32_at(bytes, at)
        .map(|value| format!("0x{value:08x}"))
        .unwrap_or_else(|| "—".into())
}
