//! ```text
//! ```

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;

pub const RECORD: usize = 0x20;
pub const GROUP_SIZE: usize = 2199;
const MAX_SHOWN: usize = 32;

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("UCB");
    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < RECORD {
        builder.warn("file is shorter than one record (0x20 bytes)");
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
        "the file length is divisible by 0x20",
        aligned,
        Some(format!("{} % 0x20 = {}", bytes.len(), bytes.len() % RECORD)),
    );
    builder.check(
        "ucb.groups",
        "the record count is divisible by the group size 2199",
        exact,
        Some(format!("{count} % {GROUP_SIZE} = {}", count % GROUP_SIZE)),
    );

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
        "index within a group equals sequence % 2199",
        index_ok == count,
        Some(format!("{index_ok}/{count}")),
    );
    builder.check(
        "ucb.group",
        "group equals sequence / 2199",
        group_ok == count,
        Some(format!("{group_ok}/{count}")),
    );

    let summary = builder.group(root, Some("summary"), "Summary");
    builder.field(summary, "typePatterns", types.len().to_string());
    for (index, kind) in types.iter().take(8).enumerate() {
        builder.field(summary, &format!("type[{index}]"), format!("0x{kind:08x}"));
    }

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
