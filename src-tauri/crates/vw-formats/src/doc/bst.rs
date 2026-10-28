//! BST —— 13 Sentinels 的战斗关卡数据（`EX DATA/BattleStageData/`，43 个）。
//!
//! ```text
//! +0x00 char[4] "BBST"
//! +0x04 12 字节 0
//! +0x10 u32     条目数（语料 116～118）
//! +0x14 u8      名字区相对 0x10 的偏移，恒为 0x0b
//! +0x15 u8      恒为 1
//! +0x16 u8      名字区长度（恒为 0x28）
//! +0x17 u8      变体号（0x0e / 0x10）
//! +0x18 3 字节填充
//! +0x1b ...     名字：`BT_STAGE00.fmd` / `BT_STAGE00.ftx`，各自 NUL 结尾 + 4 字节字段
//! ...           条目数据（未解）+ 大写标识符（如 `ENE_RANDOMWAVEGEN`）
//! ```
//!
//! **只解到这里**：名字之后的条目结构尚未闭合（不是 `.osb` 族的节点树，试过多种
//! 步长与偏移都对不上），所以文档树只报头部、名字与大写标识符，条目数据按字节保留。

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;

const MAGIC: &[u8] = b"BBST";
/// 名字区起点相对 0x10 的偏移字段所在处。
const NAME_OFFSET_AT: usize = 0x14;
const NAME_AREA_AT: usize = 0x16;
const VARIANT_AT: usize = 0x17;
const HEADER: usize = 0x1b;
const MAX_IDENTIFIERS: usize = 64;
const MIN_IDENT: usize = 6;

/// 关卡里引用的资源名。
///
/// 名字区起点由 `+0x14` 给（恒为 0x0b，即 0x1b），但名字之间夹的字段宽度随名字长度变
/// （4～5 字节），`+0x16` 的「名字区长度」只覆盖到第一个名字，所以不按步长切：
/// 只在前 512 字节里扫「可打印串 + 已知扩展名」，命中的就是资源。
pub fn resources(bytes: &[u8]) -> Vec<String> {
    let Some(offset) = bytes.get(NAME_OFFSET_AT).copied() else {
        return Vec::new();
    };
    let start = 0x10 + offset as usize;
    if start >= bytes.len() {
        return Vec::new();
    }
    let end = (start + 512).min(bytes.len());

    let mut out = Vec::new();
    let mut cursor = start;
    while cursor < end && out.len() < 8 {
        let byte = bytes[cursor];
        if byte.is_ascii_alphanumeric() {
            let mut stop = cursor;
            while stop < end
                && (bytes[stop].is_ascii_alphanumeric()
                    || matches!(bytes[stop], b'_' | b'/' | b'\\' | b'.' | b'-'))
            {
                stop += 1;
            }
            let text = String::from_utf8_lossy(&bytes[cursor..stop]).to_string();
            if text.ends_with(".fmd") || text.ends_with(".ftx") || text.ends_with(".mbs") {
                out.push(text);
            }
            cursor = stop.max(cursor + 1);
        } else {
            cursor += 1;
        }
    }
    out
}

/// 大写标识符（`ENE_RANDOMWAVEGEN` 这类枚举名）。
pub fn identifiers(bytes: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = 0usize;
    let limit = bytes.len().min(0x4000);
    while cursor < limit && out.len() < MAX_IDENTIFIERS {
        let byte = bytes[cursor];
        if byte.is_ascii_uppercase() || byte == b'_' {
            let mut end = cursor;
            while end < limit
                && (bytes[end].is_ascii_uppercase()
                    || bytes[end].is_ascii_digit()
                    || bytes[end] == b'_')
            {
                end += 1;
            }
            if end - cursor >= MIN_IDENT {
                out.push(String::from_utf8_lossy(&bytes[cursor..end]).to_string());
            }
            cursor = end.max(cursor + 1);
        } else {
            cursor += 1;
        }
    }
    out
}

/// 文档树。
pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("BST");
    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < HEADER {
        builder.warn("文件不足 0x1b 字节");
        builder.mark(root, DocMark::Error);
        return builder.finish();
    }

    let magic_ok = &bytes[..4] == MAGIC;
    let count = rdr::u32_at(bytes, 0x10).unwrap_or(0);
    let name_offset = bytes[NAME_OFFSET_AT];
    let name_area = bytes[NAME_AREA_AT];
    let variant = bytes[VARIANT_AT];

    let header = builder.group(root, Some("header"), "Header");
    builder.field(header, "magic", rdr::ascii(&bytes[..4]));
    builder.field(header, "records", count.to_string());
    builder.field(header, "nameOffset", format!("0x{name_offset:x}"));
    builder.field(header, "nameArea", name_area.to_string());
    builder.field(header, "variant", format!("0x{variant:x}"));

    builder.check(
        "bst.magic",
        "魔数为 BBST",
        magic_ok,
        Some(rdr::hex(&bytes[..4])),
    );

    let names = resources(bytes);
    builder.check(
        "bst.names",
        "名字区能解出 .fmd / .ftx",
        names.len() >= 2,
        Some(format!("{} 个", names.len())),
    );

    let list = builder.array(
        root,
        Some("resources"),
        format!("Resources ({})", names.len()),
    );
    for (index, name) in names.iter().enumerate() {
        builder.field(list, &format!("r{index}"), name.clone());
    }

    let idents = identifiers(bytes);
    let ids = builder.array(
        root,
        Some("identifiers"),
        format!("Identifiers ({})", idents.len()),
    );
    for (index, ident) in idents.iter().enumerate() {
        builder.field(ids, &format!("i{index}"), ident.clone());
    }

    let data_at = 0x10 + name_offset as usize + name_area as usize;
    builder.bytes(
        root,
        Some("body"),
        format!("Body ({} B)", bytes.len().saturating_sub(data_at)),
        data_at as u64,
        bytes.len().saturating_sub(data_at) as u64,
    );
    builder.field(root, "bodyOffset", format!("0x{data_at:x}"));

    builder.finish()
}
