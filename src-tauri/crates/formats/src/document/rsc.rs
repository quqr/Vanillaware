use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::arena::{Builder, DocArena, DocMark};
use super::fms;
use super::rdr;
use super::text;
use crate::primitives::{FileGraphDto, GraphEdgeDto, GraphNodeDto};

const ENTRY: usize = 0xb4;
const MAX_ENTRIES: u32 = 4096;

const ASSET_EXTS: [&str; 10] = [
    ".fmd", ".ftx", ".mbs", ".fsb", ".osb", ".rsc", ".fms", ".otm", ".mpd", ".ctb",
];

pub fn graph(bytes: &[u8], path: &Path) -> FileGraphDto {
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
        status: None,
    }];
    let mut edges = Vec::new();
    let mut truncated = false;

    if bytes.len() < 0x10 {
        return FileGraphDto {
            nodes,
            edges,
            truncated,
        };
    }

    let paired = path.with_extension("fms");
    let paired_ok = paired.is_file();
    nodes.push(GraphNodeDto {
        id: 1,
        label: paired
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "fms".into()),
        kind: "external".into(),
        detail: Some("text sheet".into()),
        path: Some(paired.to_string_lossy().into_owned()),
        offset: None,
        status: Some(if paired_ok { "resolved" } else { "missing" }.into()),
    });
    edges.push(GraphEdgeDto {
        from: 0,
        to: 1,
        label: "text".into(),
    });

    let count = rdr::u32_at(bytes, 0).unwrap_or(0);
    let table_end = 0x10usize + count as usize * ENTRY;
    if table_end > bytes.len() {
        truncated = true;
        return FileGraphDto {
            nodes,
            edges,
            truncated,
        };
    }

    let base = path.parent().unwrap_or_else(|| Path::new("."));

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
            label: if name.is_empty() {
                format!("entry[{index}]")
            } else {
                name
            },
            kind: "entry".into(),
            detail: Some(format!(
                "id={} group={} type=0x{entry_type:x}",
                rdr::u32_at(bytes, at + 0x40).unwrap_or(0),
                group
            )),
            path: None,
            offset: Some(offset),
            status: None,
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
            status: None,
        });
        edges.push(GraphEdgeDto {
            from: entry_id,
            to: script_id,
            label: "payload".into(),
        });

        for (rel, at) in asset_refs(bytes, offset as usize, end as usize) {
            let (resolved, ok) = resolve_asset(base, &rel);
            let ref_id = nodes.len() as u32;
            nodes.push(GraphNodeDto {
                id: ref_id,
                label: rel,
                kind: "external".into(),
                detail: Some("resource".into()),
                path: Some(resolved.to_string_lossy().into_owned()),
                offset: Some(at as u64),
                status: Some(if ok { "resolved" } else { "missing" }.into()),
            });
            edges.push(GraphEdgeDto {
                from: entry_id,
                to: ref_id,
                label: "loads".into(),
            });
        }
    }

    if count > MAX_ENTRIES {
        truncated = true;
    }
    FileGraphDto {
        nodes,
        edges,
        truncated,
    }
}

fn asset_refs(code: &[u8], start: usize, end: usize) -> Vec<(String, usize)> {
    let end = end.min(code.len());
    let mut refs = Vec::new();
    let mut seen = HashSet::new();
    let mut pos = start;
    while pos < end {
        let mut cur = pos;
        while cur < end && (0x20..0x7f).contains(&code[cur]) {
            cur += 1;
        }
        if cur == pos {
            pos += 1;
            continue;
        }
        let len = cur - pos;
        if (4..=0x100).contains(&len) && cur < end && code[cur] == 0 {
            if let Ok(text) = std::str::from_utf8(&code[pos..cur]) {
                let lower = text.to_ascii_lowercase();
                if text.contains('/')
                    && ASSET_EXTS.iter().any(|ext| lower.ends_with(ext))
                    && seen.insert(lower)
                {
                    refs.push((text.to_string(), pos));
                }
            }
        }
        pos = cur;
    }
    refs
}

fn resolve_asset(base: &Path, rel: &str) -> (PathBuf, bool) {
    let mut dir = Some(base);
    for _ in 0..3 {
        let Some(current) = dir else { break };
        let candidate = current.join(rel);
        if candidate.is_file() {
            return (candidate, true);
        }
        dir = current.parent();
    }
    (base.join(rel), false)
}

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("RSC");

    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < 0x10 {
        builder.warn("file is shorter than 0x10 bytes");
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
        "the entry table lies inside the file",
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
        builder.field(
            group,
            "id",
            rdr::u32_at(bytes, at + 0x40).unwrap_or(0).to_string(),
        );
        builder.field(
            group,
            "group",
            rdr::i16_at(bytes, at + 0x60).unwrap_or(0).to_string(),
        );
        builder.field(
            group,
            "groupPad",
            rdr::i16_at(bytes, at + 0x62).unwrap_or(0).to_string(),
        );
        builder.field(group, "offset", format!("0x{offset:x}"));
        builder.field(
            group,
            "flagA4",
            rdr::u32_at(bytes, at + 0xa4).unwrap_or(0).to_string(),
        );
        builder.field(
            group,
            "flags",
            format!("0x{:x}", rdr::u32_at(bytes, at + 0xac).unwrap_or(0)),
        );
        builder.field(
            group,
            "type",
            rdr::u32_at(bytes, at + 0xb0).unwrap_or(0).to_string(),
        );

        if offset == 0 {
            continue;
        }
        if offset as usize >= bytes.len() {
            builder.warn(format!("entry[{index}] payload out of range: {offset:#x}"));
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
        "payload offsets are ascending",
        ascending,
        None,
    );

    let min_offset = all_offsets
        .iter()
        .filter(|value| **value > 0)
        .min()
        .copied();
    if let Some(min) = min_offset {
        builder.check(
            "rsc.payloadStart",
            "the smallest payload offset equals the end of the entry area",
            min == table_end as u64,
            Some(format!("{min:#x} vs {table_end:#x}")),
        );
    }

    builder.finish()
}
