//! NND 世界节点网络（`NodeNetworkData/`，UO 语料 302 个）。
//!
//! ```text
//! +0x00  u32  version    恒为 2
//! +0x04  u32  wordCount  载荷的 u32 个数；实测恒为 3 的倍数
//! +0x08  u32  field2     语义未定，原样展示
//! +0x0c  u32  field3
//! +0x10  u32  field4
//! +0x14  u32  field5
//! +0x18  ...  载荷：wordCount 个 u32，即 wordCount/3 组、每组 12 字节
//! ...         尾部（可选）：前导字 → 64 字节名字槽 × N → 表体
//! ```
//!
//! **载荷**三字一组的语义没有闭合：`w0` 绝大多数是 0/1，但少量记录出现
//! 700～800 的取值且第 2 字解不出合理浮点，说明记录边界在这批样本上有变体。
//! 所以这里只给 `w0/w1/w1f/w2` 四个位置名，不臆造字段含义。
//!
//! **尾部**是两段：先若干 u32 前导字，再 `N × 64` 字节的 UTF-8 名字槽
//! （`Gate_D01`、`Z_1通常_廃墟碑CB11` 这类地名），最后是表体。表体有两种形态，
//! 按「账要正好平」自动判别：
//!
//! * **连接对**：`头字（4～24 B）+ count × 8 B`，头里必有一个字等于 count；
//! * **值数组**：整段按 u32 排列（出现 8 字节对不齐时走这条）。
//!
//! 两条都只做**字节归属**：每个字节都要落到「前导字 / 名字槽 / 表体」之一，
//! 落到哪里就在 check 里写明，不把 8 字节对硬说成边。

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;

const HEADER: usize = 0x18;
const UNIT: usize = 12;
const MAX_UNITS: usize = 4096;
const NAME_SLOT: usize = 64;
const MAX_NAMES: usize = 32;
const MAX_SCAN_NAMES: usize = 256;
const MAX_LINKS_SHOWN: usize = 64;
const MAX_VALUES_SHOWN: usize = 64;
const MAX_PREFIX_WORDS: usize = 32;

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("NND");

    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < HEADER {
        builder.warn("文件不足 0x18 字节");
        builder.mark(root, DocMark::Error);
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

    // --- 全量统计：不必翻几万组也能看清载荷形态 ---
    let stats = summarize(bytes, HEADER, units, body_end);
    let summary = builder.group(root, Some("summary"), "Summary");
    builder.field(summary, "w0 == 0", stats.w0_zero.to_string());
    builder.field(summary, "w0 == 1", stats.w0_one.to_string());
    builder.field(summary, "w0 > 1", stats.w0_other.to_string());
    builder.field(
        summary,
        "w1 float",
        format!("{:.3} .. {:.3}", stats.f_min, stats.f_max),
    );
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
        builder.field(
            group,
            "w1f",
            rdr::f32_at(bytes, at + 4).unwrap_or(0.0).to_string(),
        );
        builder.field(group, "w2", w2.to_string());
    }
    if units > MAX_UNITS {
        builder.truncate();
    }

    // --- 尾部 ---
    if body_end < bytes.len() {
        let tail = &bytes[body_end..];
        let group = builder.group(root, Some("tail"), format!("Tail ({} B)", tail.len()));

        match split_tail(tail) {
            Some(split) => {
                let head = builder.group(
                    group,
                    Some("head"),
                    format!("Header Words ({})", split.prefix.len()),
                );
                for (index, value) in split.prefix.iter().take(MAX_PREFIX_WORDS).enumerate() {
                    builder.field(head, &format!("w{index}"), value.to_string());
                }
                if split.prefix.len() > MAX_PREFIX_WORDS {
                    builder.truncate();
                }

                let names = builder.array(
                    group,
                    Some("names"),
                    format!("Names ({})", split.names.len()),
                );
                for (index, text) in split.names.iter().take(MAX_NAMES).enumerate() {
                    builder.text(
                        names,
                        &index.to_string(),
                        format!("[{index}]"),
                        text.clone(),
                        None,
                    );
                }

                let links = split.links;
                match &split.body {
                    TailBody::Links {
                        header,
                        pairs,
                        bytes: body_bytes,
                    } => {
                        if !header.is_empty() {
                            let node = builder.group(
                                group,
                                Some("pairHeader"),
                                format!("Pair Header ({} words)", header.len()),
                            );
                            for (index, value) in header.iter().enumerate() {
                                builder.field(node, &format!("w{index}"), value.to_string());
                            }
                        }
                        let list = builder.array(
                            group,
                            Some("links"),
                            format!("Links ({})", links_total(&split)),
                        );
                        for (index, (left, right)) in pairs.iter().enumerate() {
                            let node = builder.group(
                                list,
                                Some(&index.to_string()),
                                format!("[{index}] 0x{left:08x} → 0x{right:08x}"),
                            );
                            builder.field(node, "from", format!("0x{left:08x}"));
                            builder.field(node, "to", format!("0x{right:08x}"));
                        }
                        if links_total(&split) > pairs.len() {
                            builder.field(group, "linksShown", pairs.len().to_string());
                            builder.truncate();
                        }
                        builder.check(
                            "nnd.tail",
                            "尾部按「前导字 / 名字槽 / 表体」归段且恰好到 EOF",
                            split.consumed == tail.len(),
                            Some(format!(
                                "{} 字 + {} 名 × {NAME_SLOT} + {} 字 + {} 对 × 8 = {} / {} B",
                                split.prefix.len(),
                                split.names.len(),
                                header.len(),
                                body_bytes / 8,
                                split.consumed,
                                tail.len()
                            )),
                        );
                    }
                    TailBody::Values {
                        words,
                        residual,
                        bytes: body_bytes,
                    } => {
                        let list = builder.array(
                            group,
                            Some("values"),
                            format!("Values ({} u32)", words.len()),
                        );
                        for (index, value) in words.iter().take(MAX_VALUES_SHOWN).enumerate() {
                            builder.field(
                                list,
                                &index.to_string(),
                                format!("0x{value:08x} ({value})"),
                            );
                        }
                        if words.len() > MAX_VALUES_SHOWN {
                            builder.truncate();
                        }
                        if *residual > 0 {
                            builder.field(group, "residual", format!("{residual} B"));
                        }
                        builder.check(
                            "nnd.tail",
                            "尾部按「前导字 / 名字槽 / 表体」归段且恰好到 EOF",
                            split.consumed == tail.len(),
                            Some(format!(
                                "{} 字 + {} 名 × {NAME_SLOT} + {} B 值数组 = {} / {} B",
                                split.prefix.len(),
                                split.names.len(),
                                body_bytes,
                                split.consumed,
                                tail.len()
                            )),
                        );
                    }
                }

                // 表体形态只是描述，不是校验：语料里两种形态都合法。
                builder.field(
                    group,
                    "shape",
                    if links {
                        format!("{} 对 8 字节连接对", links_total(&split))
                    } else {
                        "u32 值数组".to_string()
                    },
                );
            }
            None => {
                builder.warn("尾部找不到名字槽，只做原始转储");
                builder.mark(group, DocMark::Warn);
                builder.check(
                    "nnd.tail",
                    "尾部按「前导字 / 名字槽 / 表体」归段且恰好到 EOF",
                    false,
                    Some(format!("{} B 未归段", tail.len())),
                );
                let words = (tail.len() / 4).min(MAX_VALUES_SHOWN);
                let mut raw = String::new();
                for index in 0..words {
                    let value = rdr::u32_at(tail, index * 4).unwrap_or(0);
                    raw.push_str(&format!("{value:08x} "));
                }
                builder.field(group, "words", raw.trim_end().to_string());
                builder.bytes(
                    group,
                    Some("bytes"),
                    "raw",
                    body_end as u64,
                    tail.len() as u64,
                );
            }
        }
    } else {
        builder.check("nnd.tail", "无尾部字节", true, None);
    }

    builder.finish()
}

enum TailBody {
    Links {
        header: Vec<u32>,
        pairs: Vec<(u32, u32)>,
        bytes: usize,
    },
    Values {
        words: Vec<u32>,
        residual: usize,
        bytes: usize,
    },
}

struct TailSplit {
    prefix: Vec<u32>,
    names: Vec<String>,
    body: TailBody,
    links: bool,
    consumed: usize,
}

fn links_total(split: &TailSplit) -> usize {
    match &split.body {
        TailBody::Links { bytes, .. } => bytes / 8,
        _ => 0,
    }
}

/// 名字槽：64 字节、NUL 结尾、NUL 之后全 0、可解成 UTF-8 且无控制字符。
fn name_slot(bytes: &[u8]) -> Option<String> {
    let slot = bytes.get(..NAME_SLOT)?;
    let end = slot.iter().position(|&byte| byte == 0)?;
    if end == 0 {
        return None;
    }
    if slot[end..].iter().any(|&byte| byte != 0) {
        return None;
    }
    let text = std::str::from_utf8(&slot[..end]).ok()?;
    if text.chars().any(|ch| (ch as u32) < 0x20) {
        return None;
    }
    Some(text.to_string())
}

/// 找最长的连续名字槽（在尾部前 8 KB 里试 4 字节对齐的起点）。
///
/// 前导区有时是几百个 `1` 的索引数组，名字块并不紧贴尾部起点，所以要扫一段窗口。
fn find_names(tail: &[u8]) -> (usize, Vec<String>) {
    let mut best_at = 0usize;
    let mut best: Vec<String> = Vec::new();
    let limit = tail.len().min(8 * 1024) / 4 * 4;
    for start in (0..limit).step_by(4) {
        let mut names = Vec::new();
        let mut cursor = start;
        while let Some(name) = name_slot(&tail[cursor..]) {
            names.push(name);
            cursor += NAME_SLOT;
        }
        if names.len() > best.len() {
            best_at = start;
            best = names;
            if best.len() >= MAX_SCAN_NAMES {
                break;
            }
        }
    }
    (best_at, best)
}

fn split_tail(tail: &[u8]) -> Option<TailSplit> {
    let (name_at, names) = find_names(tail);
    if names.is_empty() || !name_at.is_multiple_of(4) {
        return None;
    }
    let after_names = name_at + names.len() * NAME_SLOT;
    let rest = tail.len().checked_sub(after_names)?;

    let prefix: Vec<u32> = (0..name_at / 4)
        .map(|index| rdr::u32_at(tail, index * 4).unwrap_or(0))
        .collect();

    // 先试「连接对」：头字数 0～6，且头里有一个字等于对数。
    for words in 0..=6usize {
        let header_bytes = words * 4;
        if rest < header_bytes {
            continue;
        }
        let pairs_bytes = rest - header_bytes;
        if !pairs_bytes.is_multiple_of(8) {
            continue;
        }
        let count = pairs_bytes / 8;
        let header: Vec<u32> = (0..words)
            .map(|index| rdr::u32_at(tail, after_names + index * 4).unwrap_or(0))
            .collect();
        if words > 0 && !header.iter().any(|&value| value as usize == count) {
            continue;
        }
        let mut pairs = Vec::with_capacity(count.min(MAX_LINKS_SHOWN));
        for index in 0..count.min(MAX_LINKS_SHOWN) {
            let at = after_names + header_bytes + index * 8;
            let left = rdr::u32_at(tail, at)?;
            let right = rdr::u32_at(tail, at + 4)?;
            pairs.push((left, right));
        }
        return Some(TailSplit {
            prefix,
            names,
            body: TailBody::Links {
                header,
                pairs,
                bytes: pairs_bytes,
            },
            links: true,
            consumed: after_names + header_bytes + pairs_bytes,
        });
    }

    // 再退到「值数组」：整段按 u32 归属，余数字节单独记账。
    let words: Vec<u32> = (0..rest / 4)
        .map(|index| rdr::u32_at(tail, after_names + index * 4).unwrap_or(0))
        .collect();
    let residual = rest % 4;
    Some(TailSplit {
        prefix,
        names,
        body: TailBody::Values {
            words,
            residual,
            bytes: rest,
        },
        links: false,
        consumed: after_names + rest,
    })
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

fn hex_and_dec(value: u32) -> String {
    format!("{value} (0x{value:x})")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(words: usize) -> Vec<u8> {
        // 头 0x18 + words 个 u32
        let mut bytes = vec![0u8; HEADER + words * 4];
        bytes[0..4].copy_from_slice(&2u32.to_le_bytes());
        bytes[4..8].copy_from_slice(&(words as u32).to_le_bytes());
        bytes
    }

    #[test]
    fn reads_payload_units() {
        let mut bytes = payload(6);
        bytes[0x18..0x1c].copy_from_slice(&0u32.to_le_bytes());
        bytes[0x1c..0x20].copy_from_slice(&2.5f32.to_le_bytes());
        bytes[0x20..0x24].copy_from_slice(&832236u32.to_le_bytes());
        let arena = build(&bytes);
        assert!(arena
            .checks()
            .iter()
            .all(|check| check.id != "nnd.units" || check.ok));
        assert!(arena
            .checks()
            .iter()
            .any(|check| check.id == "nnd.tail" && check.ok));
        let units = arena
            .page(1, arena.root(), 0, 100)
            .unwrap()
            .items
            .iter()
            .find(|node| node.name.starts_with("Units"))
            .map(|node| node.id)
            .expect("Units 分组");
        let first = arena.page(1, units, 0, 10).unwrap();
        assert_eq!(first.total, 2);
    }

    #[test]
    fn splits_tail_into_names_and_links() {
        let mut bytes = payload(0);
        // 尾部：3 个前导字 + 2 个名字槽 + 头 3 字 + 2 对
        let tail_words = [3u32, 1, 2];
        for value in tail_words {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for name in ["Gate_D01", "Town_C24"] {
            let mut slot = vec![0u8; NAME_SLOT];
            slot[..name.len()].copy_from_slice(name.as_bytes());
            bytes.extend_from_slice(&slot);
        }
        for value in [2u32, 0, 2] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for (left, right) in [(0x19a685u32, 0x19842cu32), (0x19af16, 0x19842c)] {
            bytes.extend_from_slice(&left.to_le_bytes());
            bytes.extend_from_slice(&right.to_le_bytes());
        }
        let arena = build(&bytes);
        let checks: Vec<_> = arena
            .checks()
            .iter()
            .map(|check| (check.id.as_str(), check.ok))
            .collect();
        assert!(checks.contains(&("nnd.tail", true)));
        // 表体形态是描述字段：这一份应当被判成 8 字节连接对。
        assert_eq!(arena.count_field_value("shape", "2 对 8 字节连接对"), 1);
    }

    #[test]
    fn survives_truncation() {
        for len in 0..0x60 {
            let arena = build(&vec![0u8; len]);
            assert!(arena.total_nodes() > 0);
        }
    }
}
