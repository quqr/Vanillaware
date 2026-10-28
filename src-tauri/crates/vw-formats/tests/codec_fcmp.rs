use vw_formats::codec::fcmp::*;

#[test]
fn rejects_non_fcmp() {
    assert!(!is_fcmp(b"FTEX"));
    assert!(decode(&[0u8; 32]).unwrap().is_empty());
}
