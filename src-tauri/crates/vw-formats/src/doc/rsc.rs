//! RSC 脚本资源包：0x10 头 + 0xB4 定长条目 + 内嵌 FMSB 字节码。

use super::arena::{Builder, DocArena, DocMark};
use super::fms;
use super::rdr;
use super::text;

const ENTRY: usize = 0xb4;
const MAX_ENTRIES: u32 = 4096;

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("RSC");

    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < 0x10 {
        builder.warn("文件不足 0x10 字节");
        builder.mark(root, DocMark::Error);
        return builder.finish();
    }

    let count = rdr::u32_at(bytes, 0).unwrap_or(0);
    let checksum = rdr::u32_at(bytes, 4).unwrap_or(0);

    let header = builder.group(root, Some("header"), "Header");
    builder.field(header, "entryCount", count.to_string());
    builder.field(header, "checksum", format!("0x{checksum:08x}"));

    let table_end = 0x10usize + count as usize * ENTRY;
    builder.check(
        "rsc.table",
        "条目表在文件内",
        table_end <= bytes.len(),
        Some(format!("{table_end:#x} vs {:#x}", bytes.len())),
    );
    if table_end > bytes.len() {
        builder.mark(root, DocMark::Error);
        return builder.finish();
    }

    let entries = builder.array(root, Some("entries"), format!("Entries ({count})"));
    let shown = count.min(MAX_ENTRIES);
    if count > shown {
        builder.truncate();
    }

    let all_offsets: Vec<u64> = (0..shown)
        .map(|index| {
            let at = 0x10 + index as usize * ENTRY;
            rdr::u32_at(bytes, at + 0xa0).unwrap_or(0) as u64
        })
        .collect();

    for index in 0..shown {
        let at = 0x10 + index as usize * ENTRY;
        let name = text::utf8_or_sjis(&bytes[at..at + 0x40]);
        let offset = all_offsets[index as usize];

        let group = builder.group(entries, Some(&index.to_string()), format!("entry[{index}]"));
        builder.field(group, "name", name);
        builder.field(group, "id", rdr::u32_at(bytes, at + 0x40).unwrap_or(0).to_string());
        builder.field(group, "group", rdr::i16_at(bytes, at + 0x60).unwrap_or(0).to_string());
        builder.field(group, "groupPad", rdr::i16_at(bytes, at + 0x62).unwrap_or(0).to_string());
        builder.field(group, "offset", format!("0x{offset:x}"));
        builder.field(group, "flagA4", rdr::u32_at(bytes, at + 0xa4).unwrap_or(0).to_string());
        builder.field(group, "flags", format!("0x{:x}", rdr::u32_at(bytes, at + 0xac).unwrap_or(0)));
        builder.field(group, "type", rdr::u32_at(bytes, at + 0xb0).unwrap_or(0).to_string());

        if offset == 0 {
            continue;
        }
        if offset as usize >= bytes.len() {
            builder.warn(format!("entry[{index}] payload 越界: {offset:#x}"));
            builder.mark(group, DocMark::Warn);
            continue;
        }

        let end = all_offsets
            .iter()
            .copied()
            .filter(|value| *value > offset)
            .min()
            .unwrap_or(bytes.len() as u64)
            .min(bytes.len() as u64);
        let payload = &bytes[offset as usize..end as usize];
        builder.bytes(group, Some("payload"), "payload", offset, end - offset);

        if payload.starts_with(b"FMSB") {
            let fms_group = builder.group(group, Some("fmsb"), "FMSB");
            fms::parse(&mut builder, fms_group, payload, offset, "rsc.fms");
        }
    }

    let ascending = all_offsets
        .iter()
        .filter(|value| **value > 0)
        .collect::<Vec<_>>()
        .windows(2)
        .all(|pair| pair[0] <= pair[1]);
    builder.check(
        "rsc.offsets",
        "payload 偏移升序",
        ascending,
        None,
    );

    let min_offset = all_offsets.iter().filter(|value| **value > 0).min().copied();
    if let Some(min) = min_offset {
        builder.check(
            "rsc.payloadStart",
            "最小 payload 偏移 == 条目区末尾",
            min == table_end as u64,
            Some(format!("{min:#x} vs {table_end:#x}")),
        );
    }

    builder.finish()
}
