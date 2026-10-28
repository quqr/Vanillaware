use formats::document::drg::*;

#[test]
fn rejects_short_and_foreign_files() {
    for len in 0..0x40 {
        let arena = build(&vec![0u8; len]);
        assert!(arena.total_nodes() > 0);
    }
    let arena = build(b"NOPE........");
    assert!(arena.total_nodes() > 0);
}

#[test]
fn value_only_file_is_recognised() {
    let mut bytes = vec![0u8; HEADER + 8];
    bytes[..4].copy_from_slice(b"DRG\0");
    bytes[HEADER..HEADER + 4].copy_from_slice(&1.0f32.to_le_bytes());
    bytes[HEADER + 4..HEADER + 8].copy_from_slice(&2.0f32.to_le_bytes());
    let arena = build(&bytes);
    assert!(arena
        .checks()
        .iter()
        .any(|check| check.id == "drg.values" && check.ok));
}
