use vw_formats::doc::vs2::*;

#[test]
fn parses_two_entries() {
    let mut bytes = vec![0u8; 16 + 2 * 12];
    bytes[0..4].copy_from_slice(&2u32.to_le_bytes());
    bytes[0x10..0x14].copy_from_slice(&100u32.to_le_bytes());
    bytes[0x14..0x18].copy_from_slice(&200u32.to_le_bytes());
    bytes[0x18..0x1c].copy_from_slice(&0xFFu32.to_le_bytes());
    let arena = build(&bytes);
    assert!(arena
        .checks()
        .iter()
        .any(|check| check.id == "vs2.size" && check.ok));
    assert!(arena
        .checks()
        .iter()
        .any(|check| check.id == "vs2.ids" && check.ok));
}

#[test]
fn rejects_short_file() {
    for len in 0..16 {
        let arena = build(&vec![0u8; len]);
        assert!(arena.total_nodes() > 0);
    }
}
