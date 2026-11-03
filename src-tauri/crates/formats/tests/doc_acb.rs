use formats::document::acb::*;

#[test]
fn empty_file_is_reported_not_panicked() {
    let arena = build(&[0u8; 8], "ACB");
    assert!(arena.total_nodes() > 0);
    assert!(arena
        .checks()
        .iter()
        .any(|check| check.id == "acb.tables" && !check.ok));
}
