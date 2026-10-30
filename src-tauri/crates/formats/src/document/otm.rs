use std::path::Path;

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;
use super::resolve::Resolver;
use super::text;
use crate::primitives::{FileGraphDto, GraphEdgeDto, GraphNodeDto};

const HEADER: usize = 0x78;
const A_RECORD: usize = 4;
const B_RECORD: usize = 4;
const C_RECORD: usize = 0x10;
const MAX_RECORDS: u32 = 4096;

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("OTBM");

    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < HEADER {
        builder.warn("file is shorter than 0x78 bytes");
        builder.mark(root, DocMark::Error);
        return builder.finish();
    }

    let magic = rdr::ascii(&bytes[..4]);
    let data_size = rdr::u32_at(bytes, 4).unwrap_or(0);
    let data_offset = rdr::u32_at(bytes, 8).unwrap_or(0);
    let mbs_name = text::utf8_or_sjis(&bytes[0x10..0x50]);
    let flags = bytes[0x50];
    let version = rdr::u16_at(bytes, 0x54).unwrap_or(0);
    let count_a = rdr::u16_at(bytes, 0x56).unwrap_or(0) as u32;
    let count_b = rdr::u16_at(bytes, 0x58).unwrap_or(0) as u32;
    let count_c = rdr::u16_at(bytes, 0x5a).unwrap_or(0) as u32;
    let off_a = rdr::u64_at(bytes, 0x60).unwrap_or(0);
    let off_b = rdr::u64_at(bytes, 0x68).unwrap_or(0);
    let off_c = rdr::u64_at(bytes, 0x70).unwrap_or(0);

    let header = builder.group(root, Some("header"), "Header");
    builder.field(header, "magic", magic.clone());
    builder.field(header, "dataSize", data_size.to_string());
    builder.field(header, "dataOffset", format!("0x{data_offset:x}"));
    builder.field(
        header,
        "mbsName",
        if mbs_name.is_empty() {
            "—".into()
        } else {
            mbs_name
        },
    );
    builder.field(header, "flags", format!("0x{flags:02x}"));
    builder.field(header, "version", version.to_string());
    builder.field(header, "countA", count_a.to_string());
    builder.field(header, "countB", count_b.to_string());
    builder.field(header, "countC", count_c.to_string());
    builder.field(header, "offA", format!("0x{off_a:x}"));
    builder.field(header, "offB", format!("0x{off_b:x}"));
    builder.field(header, "offC", format!("0x{off_c:x}"));

    builder.check(
        "otm.magic",
        "magic is OTBM",
        magic == "OTBM",
        Some(format!("read {magic}")),
    );
    builder.check(
        "otm.size",
        "dataSize equals the file length minus 0x78",
        data_size as usize == bytes.len() - HEADER,
        Some(format!("{data_size:#x} vs {:#x}", bytes.len() - HEADER)),
    );
    builder.check(
        "otm.offset",
        "dataOffset == 0x78",
        data_offset as usize == HEADER,
        Some(format!("{data_offset:#x}")),
    );

    let expect_b = off_a + count_a as u64 * A_RECORD as u64;
    let expect_c = off_b + count_b as u64 * B_RECORD as u64;
    builder.check(
        "otm.offB",
        "offB == offA + countA × 4",
        off_b == expect_b,
        Some(format!("{off_b:#x} vs {expect_b:#x}")),
    );
    builder.check(
        "otm.offC",
        "offC == offB + countB × 4",
        off_c == expect_c,
        Some(format!("{off_c:#x} vs {expect_c:#x}")),
    );

    let section_c_end = off_c + count_c as u64 * C_RECORD as u64;
    builder.check(
        "otm.sections",
        "the C section ends inside the file",
        section_c_end <= bytes.len() as u64,
        Some(format!("{section_c_end:#x} vs {:#x}", bytes.len())),
    );

    build_a(&mut builder, root, bytes, off_a, count_a);
    build_b(&mut builder, root, bytes, off_b, count_b);
    build_c(&mut builder, root, bytes, off_c, count_c);

    builder.finish()
}

fn build_a(builder: &mut Builder, root: u32, bytes: &[u8], offset: u64, count: u32) {
    let array = builder.array(root, Some("secA"), format!("A ({count})"));
    for index in 0..count.min(MAX_RECORDS) {
        let at = offset as usize + index as usize * A_RECORD;
        let Some(_) = bytes.get(at..at + A_RECORD) else {
            break;
        };
        let group = builder.group(array, Some(&index.to_string()), format!("A[{index}]"));
        builder.field(
            group,
            "bStart",
            rdr::u16_at(bytes, at).unwrap_or(0).to_string(),
        );
        builder.field(group, "count", bytes[at + 2].to_string());
        builder.field(group, "pad", bytes[at + 3].to_string());
    }
    if count > MAX_RECORDS {
        builder.truncate();
    }
}

fn build_b(builder: &mut Builder, root: u32, bytes: &[u8], offset: u64, count: u32) {
    let array = builder.array(root, Some("secB"), format!("B ({count})"));
    for index in 0..count.min(MAX_RECORDS) {
        let at = offset as usize + index as usize * B_RECORD;
        let Some(_) = bytes.get(at..at + B_RECORD) else {
            break;
        };
        let group = builder.group(array, Some(&index.to_string()), format!("B[{index}]"));
        builder.field(
            group,
            "cStart",
            rdr::u16_at(bytes, at).unwrap_or(0).to_string(),
        );
        builder.field(group, "count", bytes[at + 2].to_string());
        builder.field(group, "key", bytes[at + 3].to_string());
    }
    if count > MAX_RECORDS {
        builder.truncate();
    }
}

fn build_c(builder: &mut Builder, root: u32, bytes: &[u8], offset: u64, count: u32) {
    let array = builder.array(root, Some("secC"), format!("C ({count})"));
    for index in 0..count.min(MAX_RECORDS) {
        let at = offset as usize + index as usize * C_RECORD;
        let Some(_) = bytes.get(at..at + C_RECORD) else {
            break;
        };
        let group = builder.group(array, Some(&index.to_string()), format!("C[{index}]"));
        builder.field(
            group,
            "frame",
            rdr::u16_at(bytes, at).unwrap_or(0).to_string(),
        );
        builder.field(
            group,
            "type",
            rdr::i8_at(bytes, at + 2).unwrap_or(0).to_string(),
        );
        builder.field(
            group,
            "key",
            rdr::u8_at(bytes, at + 3).unwrap_or(0).to_string(),
        );
        builder.field(
            group,
            "id",
            rdr::u32_at(bytes, at + 4).unwrap_or(0).to_string(),
        );
        builder.field(
            group,
            "flags",
            format!("0x{:x}", rdr::u16_at(bytes, at + 8).unwrap_or(0)),
        );
        builder.field(
            group,
            "x",
            rdr::i16_at(bytes, at + 0x0a).unwrap_or(0).to_string(),
        );
        builder.field(
            group,
            "y",
            rdr::i16_at(bytes, at + 0x0c).unwrap_or(0).to_string(),
        );
        builder.field(
            group,
            "z",
            rdr::i16_at(bytes, at + 0x0e).unwrap_or(0).to_string(),
        );
    }
    if count > MAX_RECORDS {
        builder.truncate();
    }
}

pub fn graph(bytes: &[u8], path: &Path) -> FileGraphDto {
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let mut resolver = Resolver::new(base);

    let mut nodes = vec![GraphNodeDto {
        id: 0,
        label: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "OTM".into()),
        kind: "file".into(),
        detail: Some(format!("{} bytes", bytes.len())),
        path: Some(path.to_string_lossy().into_owned()),
        offset: None,
        status: None,
    }];
    let mut edges = Vec::new();

    if bytes.len() < HEADER {
        return FileGraphDto {
            nodes,
            edges,
            truncated: true,
        };
    }

    let name = text::utf8_or_sjis(&bytes[0x10..0x50]);
    if !name.is_empty() {
        let ext = Path::new(&name)
            .extension()
            .map(|value| value.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let role = match ext.as_str() {
            "mbs" | "mbp" => "model",
            "fsb" => "animation",
            "ftx" | "ftp" => "texture",
            "fms" => "text",
            "rsc" => "script",
            _ => "resource",
        };
        let (resolved, ok) = resolver.resolve(&name);
        let id = nodes.len() as u32;
        nodes.push(GraphNodeDto {
            id,
            label: name,
            kind: "external".into(),
            detail: Some(role.into()),
            path: Some(resolved.to_string_lossy().into_owned()),
            offset: Some(0x10),
            status: Some(if ok { "resolved" } else { "missing" }.into()),
        });
        edges.push(GraphEdgeDto {
            from: 0,
            to: id,
            label: "resource".into(),
        });
    }

    FileGraphDto {
        nodes,
        edges,
        truncated: false,
    }
}
