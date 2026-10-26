//! RSC 脚本资源包：0x10 头 + 0xB4 定长条目 + 内嵌 FMSB 字节码。

use std::path::Path;

use super::arena::{Builder, DocArena, DocMark};
use super::fms;
use super::rdr;
use super::text;
use crate::viz::{GraphEdgeDto, GraphNodeDto, RscGraphDto};

const ENTRY: usize = 0xb4;
const MAX_ENTRIES: u32 = 4096;

/// RSC 引用图：文件 → 条目 → 字节码载荷。
pub fn graph(bytes: &[u8], path: &Path) -> RscGraphDto {
    let mut nodes = vec![GraphNodeDto {
        id: 0,
        label: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "RSC".into()),
        kind: "file".into(),
        detail: Some(format!("{} bytes", bytes.len())),
        path: Some(path.to_string_lossy().into_owned()),
        offset: None,
    }];
    let mut edges = Vec::new();
    let mut truncated = false;

    if bytes.len() < 0x10 {
        return RscGraphDto { nodes, edges, truncated };
    }

    let count = rdr::u32_at(bytes, 0).unwrap_or(0);
    let table_end = 0x10usize + count as usize * ENTRY;
    if table_end > bytes.len() {
        truncated = true;
        return RscGraphDto { nodes, edges, truncated };
    }

    let offsets: Vec<u64> = (0..count)
        .map(|index| {
            let at = 0x10 + index as usize * ENTRY;
            rdr::u32_at(bytes, at + 0xa0).unwrap_or(0) as u64
        })
        .collect();

    for index in 0..count.min(MAX_ENTRIES) {
        let at = 0x10 + index as usize * ENTRY;
        let name = text::utf8_or_sjis(&bytes[at..at + 0x40]);
        let offset = offsets[index as usize];
        let entry_id = nodes.len() as u32;
        let group = rdr::i16_at(bytes, at + 0x60).unwrap_or(0);
        let entry_type = rdr::u32_at(bytes, at + 0xb0).unwrap_or(0);
        nodes.push(GraphNodeDto {
            id: entry_id,
            label: if name.is_empty() { format!("entry[{index}]") } else { name },
            kind: "entry".into(),
            detail: Some(format!(
                "id={} group={} type=0x{entry_type:x}",
                rdr::u32_at(bytes, at + 0x40).unwrap_or(0),
                group
            )),
            path: None,
            offset: Some(offset),
        });
        edges.push(GraphEdgeDto {
            from: 0,
            to: entry_id,
            label: "contains".into(),
        });

        if offset == 0 || offset as usize >= bytes.len() {
            continue;
        }
        let end = offsets
            .iter()
            .copied()
            .filter(|value| *value > offset)
            .min()
            .unwrap_or(bytes.len() as u64)
            .min(bytes.len() as u64);
        let script_id = nodes.len() as u32;
        nodes.push(GraphNodeDto {
            id: script_id,
            label: format!("script · {} B", end - offset),
            kind: "script".into(),
            detail: Some(format!(
                "0x{offset:x}..0x{end:x} · type={entry_type} · group={group}"
            )),
            path: None,
            offset: Some(offset),
        });
        edges.push(GraphEdgeDto {
            from: entry_id,
            to: script_id,
            label: "payload".into(),
        });
    }

    if count > MAX_ENTRIES {
        truncated = true;
    }
    RscGraphDto { nodes, edges, truncated }
}

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
