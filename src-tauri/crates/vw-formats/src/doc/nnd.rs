//! NND 世界节点网络（`NodeNetworkData/`，UO 语料 302 个）。
//!
//! 结构（在 302/302 个语料文件上逐项验证，见 [`build`] 里的 check）：
//!
//! ```text
//! +0x00  u32  version    恒为 2
//! +0x04  u32  wordCount  载荷的 u32 个数；实测恒为 3 的倍数
//! +0x08  u32  field2     语义未知，原样展示
//! +0x0c  u32  field3
//! +0x10  u32  field4
//! +0x14  u32  field5
//! +0x18  ...  载荷：wordCount 个 u32，即 wordCount/3 组、每组 12 字节
//! ...         尾部：长度随文件而变（0～数十 KB），部分文件含 64 字节 UTF-8 名字
//! ```
//!
//! 已确定的只有**容器**：`version == 2`、`wordCount % 3 == 0`、载荷落在文件内。
//! 每组三个字的含义尚未确定——约 0.2% 的组里第 2 字解不出合理浮点，
//! 说明分组虽成立但字段语义未闭合，因此一律用 `w0/w1/w2` 命名，不臆造语义。
//!
//! 文件名本身是四个坐标（`-10400_-6976_-8992_-5984.nnd`），另有 `FLY_` 前缀的
//! 飞行路径变体（151/302）。尾部出现的 `Gate_D01`、`Z_1通常_廃墟碑CB11` 等
//! 是地名，但名字表的边界（前导 u32 个数随条数变化）尚未闭合，故只做字符串扫描，
//! 不声称解析了表结构。

use super::arena::{Builder, DocArena};
use super::rdr;

const HEADER: usize = 0x18;
const UNIT: usize = 12;
const MAX_UNITS: usize = 4096;
const NAME_SLOT: usize = 64;
const MAX_NAMES: usize = 32;

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("NND");

    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < HEADER {
        builder.warn("文件不足 0x18 字节");
        builder.mark(root, super::arena::DocMark::Error);
        return builder.finish();
    }

    let version = rdr::u32_at(bytes, 0).unwrap_or(0);
    let word_count = rdr::u32_at(bytes, 4).unwrap_or(0);
    let field2 = rdr::u32_at(bytes, 8).unwrap_or(0);
    let field3 = rdr::u32_at(bytes, 0x0c).unwrap_or(0);
    let field4 = rdr::u32_at(bytes, 0x10).unwrap_or(0);
    let field5 = rdr::u32_at(bytes, 0x14).unwrap_or(0);

    let header = builder.group(root, Some("header"), "Header");
    builder.field(header, "version", version.to_string());
    builder.field(header, "wordCount", word_count.to_string());
    builder.field(header, "units", (word_count / 3).to_string());
    builder.field(header, "field2", hex_and_dec(field2));
    builder.field(header, "field3", hex_and_dec(field3));
    builder.field(header, "field4", hex_and_dec(field4));
    builder.field(header, "field5", hex_and_dec(field5));

    builder.check(
        "nnd.version",
        "version == 2",
        version == 2,
        Some(format!("读到 {version}")),
    );
    builder.check(
        "nnd.units",
        "wordCount % 3 == 0（载荷按 3 字一组）",
        word_count.is_multiple_of(3),
        Some(format!("wordCount = {word_count}")),
    );

    let payload_end = HEADER as u64 + word_count as u64 * 4;
    builder.check(
        "nnd.payload",
        "载荷不越界",
        payload_end <= bytes.len() as u64,
        Some(format!("结束于 {payload_end:#x} / 文件 {:#x}", bytes.len())),
    );

    let units = (word_count / 3) as usize;
    let body_end = (payload_end as usize).min(bytes.len());

    // --- 全量统计：让人不必翻几万组也能看清载荷形态 ---
    let stats = summarize(bytes, HEADER, units, body_end);
    let summary = builder.group(root, Some("summary"), "Summary");
    builder.field(summary, "w0 == 0", stats.w0_zero.to_string());
    builder.field(summary, "w0 == 1", stats.w0_one.to_string());
    builder.field(summary, "w0 > 1", stats.w0_other.to_string());
    builder.field(summary, "w1 float", format!("{:.3} .. {:.3}", stats.f_min, stats.f_max));
    builder.field(
        summary,
        "w1 非规格化",
        format!("{} / {}", stats.f_denormal, stats.checked),
    );
    builder.field(summary, "w2", format!("{} .. {}", stats.c_min, stats.c_max));

    // --- 载荷：只展开前 MAX_UNITS 组 ---
    let array = builder.array(root, Some("units"), format!("Units ({units})"));
    for index in 0..units.min(MAX_UNITS) {
        let at = HEADER + index * UNIT;
        if at + UNIT > bytes.len() {
            break;
        }
        let w0 = rdr::u32_at(bytes, at).unwrap_or(0);
        let w1 = rdr::u32_at(bytes, at + 4).unwrap_or(0);
        let w2 = rdr::u32_at(bytes, at + 8).unwrap_or(0);
        let group = builder.group(array, Some(&index.to_string()), format!("[{index}]"));
        builder.field(group, "w0", w0.to_string());
        builder.field(group, "w1", format!("0x{w1:08x}"));
        builder.field(group, "w1f", rdr::f32_at(bytes, at + 4).unwrap_or(0.0).to_string());
        builder.field(group, "w2", w2.to_string());
    }
    if units > MAX_UNITS {
        builder.truncate();
    }

    // --- 尾部：原始字 + 可读名字 ---
    if body_end < bytes.len() {
        let tail = &bytes[body_end..];
        let group = builder.group(root, Some("tail"), format!("Tail ({} B)", tail.len()));
        let words = (tail.len() / 4).min(32);
        let mut raw = String::new();
        for index in 0..words {
            let value = rdr::u32_at(tail, index * 4).unwrap_or(0);
            raw.push_str(&format!("{value:08x} "));
        }
        let raw_text: String = if raw.is_empty() {
            "—".to_string()
        } else {
            raw.trim_end().to_string()
        };
        builder.field(group, "words", raw_text);

        let names = scan_names(tail);
        if names.is_empty() {
            builder.field(group, "names", "—");
        } else {
            let array = builder.array(group, Some("names"), format!("Names ({})", names.len()));
            for (offset, text) in names.iter().take(MAX_NAMES) {
                let slot = builder.group(array, Some(&format!("{offset:x}")), format!("@0x{offset:x}"));
                builder.field(slot, "offset", format!("0x{offset:x}"));
                builder.field(slot, "text", text.clone());
            }
            if names.len() > MAX_NAMES {
                builder.truncate();
            }
        }
    } else {
        builder.check("nnd.tail", "无尾部字节", true, None);
    }

    builder.finish()
}

struct Stats {
    checked: u64,
    w0_zero: u64,
    w0_one: u64,
    w0_other: u64,
    f_min: f32,
    f_max: f32,
    f_denormal: u64,
    c_min: u32,
    c_max: u32,
}

fn summarize(bytes: &[u8], base: usize, units: usize, end: usize) -> Stats {
    let mut stats = Stats {
        checked: 0,
        w0_zero: 0,
        w0_one: 0,
        w0_other: 0,
        f_min: f32::INFINITY,
        f_max: f32::NEG_INFINITY,
        f_denormal: 0,
        c_min: u32::MAX,
        c_max: 0,
    };
    for index in 0..units {
        let at = base + index * UNIT;
        if at + UNIT > end {
            break;
        }
        let (Some(w0), Some(w1), Some(w2)) = (
            rdr::u32_at(bytes, at),
            rdr::u32_at(bytes, at + 4),
            rdr::u32_at(bytes, at + 8),
        ) else {
            break;
        };
        stats.checked += 1;
        match w0 {
            0 => stats.w0_zero += 1,
            1 => stats.w0_one += 1,
            _ => stats.w0_other += 1,
        }
        let f = f32::from_bits(w1);
        if f.is_finite() {
            if f < stats.f_min {
                stats.f_min = f;
            }
            if f > stats.f_max {
                stats.f_max = f;
            }
            // 次正规数说明这一组的三字分组在这条记录上并不成立
            if f != 0.0 && f.abs() < f32::MIN_POSITIVE {
                stats.f_denormal += 1;
            }
        } else {
            stats.f_denormal += 1;
        }
        stats.c_min = stats.c_min.min(w2);
        stats.c_max = stats.c_max.max(w2);
    }
    if !stats.f_min.is_finite() {
        stats.f_min = 0.0;
    }
    if !stats.f_max.is_finite() {
        stats.f_max = 0.0;
    }
    if stats.c_min == u32::MAX {
        stats.c_min = 0;
    }
    stats
}

/// 在尾部扫可读名字：NUL 结尾、长度 ≥2、含 ASCII 字母的 UTF-8 片段。
///
/// 这是**扫描**而不是解析表结构——名字槽的边界随条数变化，尚未闭合。
fn scan_names(tail: &[u8]) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut index = 0usize;
    while index < tail.len() {
        let byte = tail[index];
        if byte == 0 {
            if index.saturating_sub(start) >= 2 && index - start <= NAME_SLOT {
                let slice = &tail[start..index];
                if slice.iter().all(|b| *b >= 0x20 || *b >= 0x80) {
                    if let Ok(text) = std::str::from_utf8(slice) {
                        if text.chars().any(|c| c.is_ascii_alphabetic()) {
                            out.push((start, text.to_string()));
                        }
                    }
                }
            }
            start = index + 1;
        }
        index += 1;
    }
    out
}

fn hex_and_dec(value: u32) -> String {
    format!("{value} (0x{value:x})")
}
