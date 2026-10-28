use formats::document::*;

#[test]
fn binary_view_is_truncation_safe() {
    for len in 0..128 {
        let bytes = vec![0x41u8; len];
        let arena = parse_bytes(&bytes, "zzz", None).expect("binary");
        assert!(arena.total_nodes() > 0);
    }
}

#[test]
fn nnd_view_reads_header_units_and_names() {
    // 头 0x18 + 2 组（6 个字）+ 尾部一个 64 字节名字槽
    let mut bytes = vec![0u8; 0x18 + 24 + 68];
    bytes[0..4].copy_from_slice(&2u32.to_le_bytes());
    bytes[4..8].copy_from_slice(&6u32.to_le_bytes());
    bytes[8..12].copy_from_slice(&1003u32.to_le_bytes());
    bytes[0x18..0x1c].copy_from_slice(&0u32.to_le_bytes());
    bytes[0x1c..0x20].copy_from_slice(&2.5f32.to_le_bytes());
    bytes[0x20..0x24].copy_from_slice(&832236u32.to_le_bytes());
    let tail = 0x18 + 24;
    bytes[tail..tail + 4].copy_from_slice(&1u32.to_le_bytes());
    bytes[tail + 4..tail + 12].copy_from_slice(b"Gate_D01");

    let arena = parse_bytes(&bytes, "nnd", None).expect("nnd");
    let page = arena.page(1, arena.root(), 0, 100).unwrap();
    assert!(page.total >= 4, "应含 header / summary / units / tail");
    let checks = arena.checks();
    assert!(checks
        .iter()
        .all(|check| check.id != "nnd.version" || check.ok));
    assert!(checks
        .iter()
        .all(|check| check.id != "nnd.units" || check.ok));

    let units = page
        .items
        .iter()
        .find(|node| node.name.starts_with("Units ("))
        .map(|node| node.id)
        .expect("Units 分组");
    let first = arena.page(1, units, 0, 10).unwrap();
    assert_eq!(first.total, 2, "两组记录");
    let unit = first.items[0].id;
    let fields: Vec<String> = arena
        .page(1, unit, 0, 10)
        .unwrap()
        .items
        .iter()
        .map(|node| node.name.clone())
        .collect();
    for key in ["w0", "w1", "w1f", "w2"] {
        assert!(fields.iter().any(|name| name == key), "缺少字段 {key}");
    }
}

#[test]
fn nnd_view_survives_truncation() {
    for len in 0..0x40 {
        let bytes = vec![0u8; len];
        let _ = parse_bytes(&bytes, "nnd", None);
    }
}

#[test]
fn ftx_view_reads_header_and_pages() {
    let mut bytes = vec![0u8; 0x80];
    bytes[..4].copy_from_slice(b"FTEX");
    bytes[4..8].copy_from_slice(&0x20u32.to_le_bytes());
    bytes[8..12].copy_from_slice(&0x50u32.to_le_bytes());
    bytes[12..16].copy_from_slice(&1u32.to_le_bytes());
    bytes[0x20..0x24].copy_from_slice(b"a.nv");

    let arena = parse_bytes(&bytes, "ftx", None).expect("ftx");
    let page = arena.page(1, arena.root(), 0, 100).unwrap();
    assert!(page.total >= 2);
}

#[test]
fn mbs_view_reads_sections() {
    let mut bytes = vec![0u8; 0x130];
    bytes[..4].copy_from_slice(b"FMBS");
    bytes[4..8].copy_from_slice(&0u32.to_le_bytes());
    bytes[0x14..0x16].copy_from_slice(&0x77u16.to_le_bytes());

    let arena = parse_bytes(&bytes, "mbs", Some("swi_unic")).expect("mbs");
    let page = arena.page(1, arena.root(), 0, 100).unwrap();
    assert!(page.total >= 2);
}

#[test]
fn bbs_view_reads_counts_and_offsets() {
    let mut bytes = vec![0u8; 0x380];
    bytes[..4].copy_from_slice(b"UBBS");
    for index in 0..9 {
        bytes[0x338 + index * 8..0x340 + index * 8].copy_from_slice(&0x380u64.to_le_bytes());
    }

    let arena = parse_bytes(&bytes, "bbs", None).expect("bbs");
    let page = arena.page(1, arena.root(), 0, 100).unwrap();
    assert!(page.total >= 3);
}

#[test]
fn osb_view_walks_empty_scene() {
    let mut bytes = vec![0u8; 0x8a];
    bytes[..4].copy_from_slice(b"OSB\0");
    bytes[4..8].copy_from_slice(&1u32.to_le_bytes());
    bytes[0x14..0x18].copy_from_slice(&0x18u32.to_le_bytes());
    // 场景根：type 2, body 0x52, children 0, pad。
    bytes[0x28..0x2c].copy_from_slice(&2u32.to_le_bytes());
    bytes[0x2c..0x30].copy_from_slice(&0x52u32.to_le_bytes());

    let arena = parse_bytes(&bytes, "osb", None).expect("osb");
    assert!(arena.total_nodes() > 3);
}

/// 造一个最小 bundle：1 个资源名 + 1 个节点（caption + label + group）。
fn build_bundle(
    magic: &[u8],
    type_id: u32,
    caption: &str,
    label: &str,
    body_len: usize,
) -> Vec<u8> {
    let mut body = vec![0u8; body_len.max(0x52)];
    body[..caption.len()].copy_from_slice(caption.as_bytes());
    body[0x40..0x40 + label.len()].copy_from_slice(label.as_bytes());
    body[0x50..0x52].copy_from_slice(&7u16.to_le_bytes());

    // 偏移表占 0x18..0x1c，名字从 0x1c 开始（相对 0x10 的偏移就是 0x0c）。
    let name = b"Character/a.mbs\0";
    let pool_size = 0x0c + name.len();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(magic);
    bytes.extend_from_slice(&4u32.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 8]);
    bytes.extend_from_slice(&1u32.to_le_bytes()); // 资源数
    bytes.extend_from_slice(&(pool_size as u32).to_le_bytes());
    bytes.extend_from_slice(&0x0cu32.to_le_bytes()); // 相对 0x10 的偏移
    bytes.extend_from_slice(name);
    bytes.extend_from_slice(&type_id.to_le_bytes());
    bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&body);
    bytes
}

#[test]
fn oeb_bundle_closes_and_reads_slots() {
    // 槽位 body 固定 228 字节，0x52 起是 scale(2) + position(3)。
    let mut bytes = build_bundle(b"OEB\0", 28, "SLOT01", "JK01A", 228);
    let body = 0x10 + 0x0c + b"Character/a.mbs\0".len() + 0x10;
    let floats = [1.0f32, 1.0, 100.0, -200.0, -30.0];
    for (index, value) in floats.iter().enumerate() {
        let at = body + 0x52 + index * 4;
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    let arena = parse_bytes(&bytes, "oeb", None).expect("oeb");
    let checks = arena.checks();
    for id in ["bundle.magic", "bundle.closed"] {
        assert!(
            checks.iter().all(|check| check.id != id || check.ok),
            "{id} 不成立"
        );
    }

    let slots = oeb::slots(&bytes);
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].caption, "SLOT01");
    assert_eq!(slots[0].label, "JK01A");
    assert_eq!(slots[0].position[0], 100.0);
    assert_eq!(oeb::resources(&bytes), vec!["Character/a.mbs".to_string()]);
}

#[test]
fn orb_bundle_lists_cameras() {
    let bytes = build_bundle(b"ORB\0", 30, "STAGE01", "00", 0x52);
    assert!(parse_bytes(&bytes, "orb", None).is_ok());
    let cameras = orb::cameras(&bytes);
    assert_eq!(cameras.len(), 1);
    assert_eq!(cameras[0].stage, "STAGE01");
    assert_eq!(orb::resources(&bytes), vec!["Character/a.mbs".to_string()]);
}

#[test]
fn bst_reads_stage_header() {
    // 头部 0x16 之后是值流：两个字符串 + END。
    let mut bytes = vec![0u8; 0x16];
    bytes[..4].copy_from_slice(b"BBST");
    bytes[0x10..0x14].copy_from_slice(&118u32.to_le_bytes());
    bytes[0x14] = 0x0b;
    bytes[0x15] = 1;
    for name in ["BT_STAGE00.fmd", "BT_STAGE00.ftx"] {
        bytes.push(0x28);
        bytes.extend_from_slice(&(name.len() as u32).to_le_bytes());
        bytes.extend_from_slice(name.as_bytes());
        bytes.push(0);
    }
    bytes.push(0x34);

    let arena = parse_bytes(&bytes, "bst", None).expect("bst");
    let checks = arena.checks();
    for id in ["bst.magic", "bst.names", "bst.closed"] {
        assert!(
            checks.iter().all(|check| check.id != id || check.ok),
            "{id} 不成立"
        );
    }
    assert_eq!(
        bst::parse(&bytes).names,
        vec!["BT_STAGE00.fmd".to_string(), "BT_STAGE00.ftx".to_string()]
    );
}

#[test]
fn bundle_view_survives_truncation() {
    let full = build_bundle(b"OEB\0", 28, "x", "y", 0x52);
    for len in 0..full.len().min(0x60) {
        let _ = parse_bytes(&full[..len], "oeb", None);
    }
}

#[test]
fn unsupported_error_is_never_panicking() {
    let prefix = b"FTEX";
    for len in 0..prefix.len() {
        let _ = parse_bytes(&prefix[..len], "ftx", None);
    }
}
