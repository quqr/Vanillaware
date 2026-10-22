//! MBS 族的结构视图：0x130 头 + 12 个分节的 sp/count/stride。
//!
//! 不解读记录语义；记录只给区间与样本，避免把几 MB 的 V55 塞进文档树。

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;
use crate::mbs::profile;

const HEADER_SIZE: u64 = 0x130;

pub fn build(bytes: &[u8], tag: &str) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("FMBS");

    builder.field(root, "size", bytes.len().to_string());
    builder.field(root, "tag", tag);

    if bytes.len() < HEADER_SIZE as usize {
        builder.warn(format!("文件不足 {HEADER_SIZE:#x} 字节"));
        builder.mark(root, DocMark::Error);
        return builder.finish();
    }

    let magic = rdr::ascii(&bytes[..4]);
    let body_size = rdr::u32_at(bytes, 4).unwrap_or(0);
    let version = rdr::u16_at(bytes, 0x14).unwrap_or(0);

    let header = builder.group(root, Some("header"), "Header");
    builder.field(header, "magic", magic.clone());
    builder.field(header, "bodySize", format!("{body_size} (0x{body_size:x})"));
    builder.field(header, "version", format!("0x{version:02x}"));

    builder.check(
        "mbs.magic",
        "魔数为 FMBS",
        magic == "FMBS",
        Some(format!("读到 {magic}")),
    );

    let expected = body_size as u64 + HEADER_SIZE;
    builder.check(
        "mbs.body",
        "bodySize + 0x130 == 文件长",
        expected == bytes.len() as u64,
        Some(format!("{expected:#x} vs {:#x}", bytes.len())),
    );

    let Some(profile) = profile::lookup(tag) else {
        builder.warn(format!("未知 mbs tag「{tag}」，只展示头部"));
        builder.mark(root, DocMark::Error);
        return builder.finish();
    };

    builder.check(
        "mbs.version",
        "版本号与 tag 期望一致",
        version == profile.version,
        Some(format!("读到 0x{version:02x}，期望 0x{:02x}", profile.version)),
    );

    let counts = match profile.read_counts(bytes) {
        Ok(counts) => counts,
        Err(error) => {
            builder.warn(format!("分节 count 读取失败：{error}"));
            [0u32; 12]
        }
    };

    let sections = builder.array(root, Some("sections"), "Sections (12)");
    for (index, section) in profile.sections.iter().enumerate() {
        let name = format!("s{index:x}");
        let group = builder.group(sections, Some(name.as_str()), format!("{name} · {tag}"));
        let sp = rdr::u32_at(bytes, section.sp_at as usize);
        let count = counts[index];
        let stride = section.stride;

        builder.field(
            group,
            "sp",
            sp.map(|value| format!("0x{value:08x}"))
                .unwrap_or_else(|| "—".into()),
        );
        builder.field(group, "count", count.to_string());
        builder.field(group, "stride", format!("0x{stride:x}"));

        let Some(sp) = sp else {
            builder.warn(format!("{name} 的 sp 越界"));
            builder.mark(group, DocMark::Warn);
            continue;
        };

        let bytes_len = count as u64 * stride as u64;
        let end = sp as u64 + bytes_len;
        if end > bytes.len() as u64 {
            builder.warn(format!("{name} 区间 {:#x}..{end:#x} 超出文件", sp));
            builder.mark(group, DocMark::Warn);
        } else if bytes_len > 0 {
            builder.bytes(
                group,
                Some("range"),
                "区间",
                sp as u64,
                bytes_len,
            );
        }

        let samples = count.min(3);
        for sample in 0..samples {
            let offset = sp as u64 + sample as u64 * stride as u64;
            if offset + stride as u64 > bytes.len() as u64 {
                break;
            }
            let key = format!("rec[{sample}]");
            builder.bytes(group, Some(key.as_str()), key.clone(), offset, stride as u64);
        }
    }

    builder.finish()
}
