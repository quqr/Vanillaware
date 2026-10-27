//! UCB 大地图碰撞/网格单元（`FieldBG/Field.ucb`）。
//!
//! 定长记录流，**每条 0x20 字节**：
//!
//! ```text
//! +0x00 u32 区内索引（每个组内从 0 连续递增）
//! +0x04 u32 组号（0 起，组内条数恒定）
//! +0x08 u32 类型/模式常量（组 0 为 0x008105f1，其余组同）
//! +0x0c u32 掩码（恒为 0xff000000）
//! +0x10 u32 ×4 恒 0（预留）
//! ```
//!
//! 本语料的闭合判据：`1311 组 × 2199 条 × 0x20 = 92 252 448 = 文件长度`。
//! 三件事在这里校验：文件长度被 0x20 整除、总条数被组容量整除、
//! 索引与组号满足 `index % 2199` / `index / 2199`。

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;

const RECORD: usize = 0x20;
const GROUP_SIZE: usize = 2199;
const MAX_SHOWN: usize = 32;

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("UCB");
    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < RECORD {
        builder.warn("文件不足一条记录（0x20 字节）");
        builder.mark(root, DocMark::Error);
        return builder.finish();
    }

    let aligned = bytes.len().is_multiple_of(RECORD);
    let count = bytes.len() / RECORD;
    let groups = count / GROUP_SIZE;
    let exact = count.is_multiple_of(GROUP_SIZE);

    let header = builder.group(root, Some("header"), "Header");
    builder.field(header, "records", count.to_string());
    builder.field(header, "recordSize", RECORD.to_string());
    builder.field(header, "groupSize", GROUP_SIZE.to_string());
    builder.field(header, "groups", groups.to_string());

    builder.check(
        "ucb.align",
        "文件长度被 0x20 整除",
        aligned,
        Some(format!("{} % 0x20 = {}", bytes.len(), bytes.len() % RECORD)),
    );
    builder.check(
        "ucb.groups",
        "总条数被组容量 2199 整除",
        exact,
        Some(format!("{count} % {GROUP_SIZE} = {}", count % GROUP_SIZE)),
    );

    // 索引/组号一致性（全量扫，代价是每 32 字节读两个 u32）。
    let mut index_ok = 0usize;
    let mut group_ok = 0usize;
    let mut types: Vec<u32> = Vec::new();
    for index in 0..count {
        let at = index * RECORD;
        let Some(local) = rdr::u32_at(bytes, at) else {
            break;
        };
        let Some(group) = rdr::u32_at(bytes, at + 4) else {
            break;
        };
        if local as usize == index % GROUP_SIZE {
            index_ok += 1;
        }
        if group as usize == index / GROUP_SIZE {
            group_ok += 1;
        }
        let kind = rdr::u32_at(bytes, at + 8).unwrap_or(0);
        if !types.contains(&kind) {
            types.push(kind);
        }
    }

    builder.check(
        "ucb.index",
        "区内索引 == 序号 % 2199",
        index_ok == count,
        Some(format!("{index_ok}/{count}")),
    );
    builder.check(
        "ucb.group",
        "组号 == 序号 / 2199",
        group_ok == count,
        Some(format!("{group_ok}/{count}")),
    );

    let summary = builder.group(root, Some("summary"), "Summary");
    builder.field(summary, "typePatterns", types.len().to_string());
    for (index, kind) in types.iter().take(8).enumerate() {
        builder.field(summary, &format!("type[{index}]"), format!("0x{kind:08x}"));
    }

    // 组表：只列组首记录，1311 组足够看出「一组 2199 条」的规律。
    let list = builder.array(root, Some("groups"), format!("Groups ({groups})"));
    for group in 0..groups.min(MAX_SHOWN) {
        let at = group * GROUP_SIZE * RECORD;
        let node = builder.group(list, Some(&group.to_string()), format!("[{group}]"));
        builder.field(node, "offset", format!("0x{at:x}"));
        builder.field(
            node,
            "type",
            format!("0x{:08x}", rdr::u32_at(bytes, at + 8).unwrap_or(0)),
        );
        builder.field(
            node,
            "mask",
            format!("0x{:08x}", rdr::u32_at(bytes, at + 0x0c).unwrap_or(0)),
        );
    }
    if groups > MAX_SHOWN {
        builder.field(root, "groupsShown", MAX_SHOWN.to_string());
        builder.truncate();
    }

    let list = builder.array(root, Some("records"), format!("Records[0..{MAX_SHOWN}]"));
    for index in 0..count.min(MAX_SHOWN) {
        let at = index * RECORD;
        let node = builder.group(list, Some(&index.to_string()), format!("[{index}]"));
        builder.field(
            node,
            "index",
            rdr::u32_at(bytes, at).unwrap_or(0).to_string(),
        );
        builder.field(
            node,
            "group",
            rdr::u32_at(bytes, at + 4).unwrap_or(0).to_string(),
        );
        builder.field(
            node,
            "type",
            format!("0x{:08x}", rdr::u32_at(bytes, at + 8).unwrap_or(0)),
        );
        builder.field(
            node,
            "mask",
            format!("0x{:08x}", rdr::u32_at(bytes, at + 0x0c).unwrap_or(0)),
        );
    }

    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_short_file() {
        for len in 0..0x40 {
            let arena = build(&vec![0u8; len]);
            assert!(arena.total_nodes() > 0);
        }
    }

    #[test]
    fn checks_alignment_and_grouping() {
        // 两组（满足 2199 整除）但组号/索引全 0，组号校验应失败。
        let bytes = vec![0u8; RECORD * GROUP_SIZE * 2];
        let arena = build(&bytes);
        let checks: Vec<_> = arena
            .checks()
            .iter()
            .map(|c| (c.id.as_str(), c.ok))
            .collect();
        assert!(checks.contains(&("ucb.align", true)));
        assert!(checks.contains(&("ucb.groups", true)));
        assert!(checks.contains(&("ucb.group", false)));
    }
}
