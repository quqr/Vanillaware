use formats::document::awb::*;

fn two_entry_afs() -> Vec<u8> {
    let mut bytes = vec![0u8; 0x10 + 8 * 2 + 4 + 10];
    bytes[..4].copy_from_slice(b"AFS2");
    bytes[4..8].copy_from_slice(&0x0004_0402u32.to_le_bytes());
    bytes[8..12].copy_from_slice(&2u32.to_le_bytes());
    bytes[12..16].copy_from_slice(&0x20u32.to_le_bytes());
    // ids
    bytes[0x10..0x14].copy_from_slice(&0u32.to_le_bytes());
    bytes[0x14..0x18].copy_from_slice(&1u32.to_le_bytes());
    // offsets
    let data = 0x10 + 8 * 2 + 4;
    bytes[0x18..0x1c].copy_from_slice(&(data as u32).to_le_bytes());
    bytes[0x1c..0x20].copy_from_slice(&((data + 4) as u32).to_le_bytes());
    let total = bytes.len() as u32;
    bytes[0x20..0x24].copy_from_slice(&total.to_le_bytes());
    bytes
}

#[test]
fn parses_header_and_entries() {
    let bytes = two_entry_afs();
    let afs = parse(&bytes, 0).expect("afs2");
    assert_eq!(afs.entries.len(), 2);
    assert_eq!(afs.entries[0].size, 4);
    assert_eq!(afs.entries[1].size, 6);
    assert_eq!(afs.declared_total as usize, bytes.len());
}

#[test]
fn rejects_truncation() {
    let bytes = two_entry_afs();
    for len in 0..bytes.len() {
        let _ = parse(&bytes[..len], 0);
    }
}
