use formats::document::ufb::*;

#[test]
fn rejects_short_file() {
    for len in 0..0x90 {
        let arena = build(&vec![0u8; len]);
        assert!(arena.total_nodes() > 0);
    }
}

#[test]
fn record_size_counts_layers_and_groups() {
    let mut bytes = vec![0u8; 0x400];
    bytes[0x20..0x22].copy_from_slice(&1u16.to_le_bytes());
    bytes[0x22] = 1;
    bytes[0x23] = 1;
    let after = 0x24 + 0x40 + 0x90;
    bytes[after + 3] = 2;
    let (size, layers, ext, groups) = record_size(&bytes, 0).expect("record");
    assert_eq!((layers, ext, groups), (1, 1, 1));
    assert_eq!(size, after + 2 * 0xF0 + 4);
}
