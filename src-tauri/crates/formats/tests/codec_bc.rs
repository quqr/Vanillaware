use formats::codec::bc::*;

#[test]
fn bc3_block_size() {
    let out = decode_bc3(&[0u8; 16]);
    assert_eq!(out.len(), 16 * 4);
}

#[test]
fn bc4_block_size() {
    let out = decode_bc4(&[0u8; 8]);
    assert_eq!(out.len(), 16);
}
