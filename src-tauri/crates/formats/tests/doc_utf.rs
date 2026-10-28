use formats::document::utf::*;

fn minimal_table() -> Vec<u8> {
    let mut bytes = vec![0u8; 0x60];
    bytes[..4].copy_from_slice(MAGIC);
    bytes[4..8].copy_from_slice(&0x58u32.to_be_bytes());
    bytes[0x0a..0x0c].copy_from_slice(&0x30u16.to_be_bytes());
    bytes[0x0c..0x10].copy_from_slice(&0x38u32.to_be_bytes());
    bytes[0x10..0x14].copy_from_slice(&0x38u32.to_be_bytes());
    bytes[0x18..0x1a].copy_from_slice(&1u16.to_be_bytes());
    bytes[0x1a..0x1c].copy_from_slice(&4u16.to_be_bytes());
    bytes[0x1c..0x20].copy_from_slice(&1u32.to_be_bytes());
    bytes[0x20] = 0x54;
    bytes[0x21..0x25].copy_from_slice(&0u32.to_be_bytes());
    bytes[0x40..0x46].copy_from_slice(b"Value\0");
    bytes[0x38..0x3c].copy_from_slice(&42u32.to_be_bytes());
    bytes
}

#[test]
fn parses_minimal_table() {
    let bytes = minimal_table();
    let tables = scan_tables(&bytes);
    assert_eq!(tables.len(), 1);
    let table = &tables[0];
    assert_eq!(table.columns.len(), 1);
    assert_eq!(table.columns[0].name, "Value");
    assert_eq!(table.row_width, 4);
    assert_eq!(table.computed_width(), 4);
    assert_eq!(table.row_count, 1);
    let value = cell_text(&bytes, table, table.rows_at(), &table.columns[0]);
    assert_eq!(value, "42");
}

#[test]
fn rejects_truncated_table() {
    let bytes = minimal_table();
    for len in 0..bytes.len() {
        let _ = parse_table(&bytes[..len], 0);
    }
}

#[test]
fn build_reports_empty_table_error() {
    let bytes = [0u8; 32];
    assert!(scan_tables(&bytes).is_empty());
    let arena = build(&bytes, "ACB");
    assert!(arena.total_nodes() > 0);
}
