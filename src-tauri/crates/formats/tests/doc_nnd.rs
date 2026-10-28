use formats::document::nnd::*;

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
