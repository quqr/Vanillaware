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
    let mut classes: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
    let mut presence_ok = 0usize;
    let mut outside = 0usize;
    let mut tail_ok = 0usize;
    let mut areas: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
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
        let word = rdr::u32_at(bytes, at + 8).unwrap_or(0);
        let inside = word != 0;
        if (word & 0x003f_0000 != 0) == inside {
            presence_ok += 1;
        }
        if inside {
            classes.insert(word >> 22);
            areas.insert(word & 0xffff);
        } else {
            outside += 1;
        }
        let color = rdr::u32_at(bytes, at + 0x0c).unwrap_or(0);
        let zero = (0..4).all(|slot| rdr::u32_at(bytes, at + 0x10 + slot * 4).unwrap_or(1) == 0);
        if color == 0xff00_0000 && zero {
            tail_ok += 1;
        }
    }
    let types: Vec<u32> = classes.into_iter().collect();

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
    builder.check(
        "ucb.presence",
        "bits 16..21 are set exactly when the cell is inside the map",
        presence_ok == count,
        Some(format!(
            "{presence_ok}/{count}, {outside} cells are all zero"
        )),
    );
    builder.check(
        "ucb.tail",
        "the color word is 0xff000000 and the last 16 bytes are zero",
        tail_ok == count,
        Some(format!("{tail_ok}/{count}")),
    );
    builder.check(
        "ucb.ids",
        "area ids stay inside the 1647 masks of Field.ufb",
        areas.iter().next_back().copied().unwrap_or(0) < 1647,
        Some(format!(
            "{} distinct ids, max {:?}",
            areas.len(),
            areas.iter().next_back()
        )),
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
