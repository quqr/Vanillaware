use formats::document::usm::*;

fn minimal_usm() -> Vec<u8> {
    let header = 0x40usize;
    let mut bytes = vec![0u8; header + 0x40 + 0x20];
    bytes[..4].copy_from_slice(b"CRID");
    bytes[4..8].copy_from_slice(&((header - 8) as u32).to_be_bytes());
    bytes[8..12].copy_from_slice(&0x0018_0690u32.to_be_bytes());
    bytes[header..header + 4].copy_from_slice(b"@SFV");
    bytes[header + 4..header + 8].copy_from_slice(&0x40u32.to_be_bytes());
    let next = header + 0x40;
    bytes[next..next + 4].copy_from_slice(b"@END");
    bytes[next + 4..next + 8].copy_from_slice(&0x20u32.to_be_bytes());
    bytes
}

#[test]
fn walks_chunk_chain() {
    let bytes = minimal_usm();
    let header = parse(&bytes).expect("usm");
    assert_eq!(header.chunks.len(), 2);
    assert_eq!(header.chunks[0].kind, "@SFV");
    assert_eq!(header.chunks[1].kind, "@END");
    assert_eq!(header.end, bytes.len());
}

#[test]
fn rejects_truncation() {
    let bytes = minimal_usm();
    for len in 0..bytes.len() {
        let _ = parse(&bytes[..len]);
    }
}
