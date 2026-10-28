use formats::content::*;

#[test]
fn drg_lists_databases_and_commands() {
    let mut bytes = vec![0u8; 0x38 + 2 * 0x90];
    bytes[..4].copy_from_slice(b"DRG\0");
    bytes[4..8].copy_from_slice(&2u32.to_le_bytes());
    bytes[8..12].copy_from_slice(&0u32.to_le_bytes());
    bytes[0x38..0x38 + 5].copy_from_slice(b"DEBUG");
    let projection = project(&bytes, "drg").expect("drg must expose a content view");
    assert!(projection.sections[0].columns.contains(&"name".to_string()));
    assert_eq!(projection.sections[0].rows.len(), 2);
}

#[test]
fn vs2_expands_flag_bitmap() {
    let count = 3usize;
    let mut bytes = vec![0u8; 0x10 + count * 4 + count * 8];
    bytes[0..4].copy_from_slice(&(count as u32).to_le_bytes());
    for index in 0..count {
        let at = 0x10 + index * 4;
        bytes[at..at + 4].copy_from_slice(&(1000u32 + index as u32).to_le_bytes());
    }
    bytes[0x10 + count * 4..0x10 + count * 4 + 8].copy_from_slice(&1u64.to_le_bytes());
    let projection = project(&bytes, "vs2").expect("vs2 must expose a content view");
    let bitmap = projection.bitmap.expect("must produce a bitmap");
    assert_eq!(bitmap.width, count as u32);
    assert_eq!(bitmap.height, 64);
    assert_eq!(bitmap.values[63 * count], 255);
    assert_eq!(bitmap.values[63 * count + 1], 0);
}

#[test]
fn scb_renders_readable_source() {
    let mut bytes = Vec::new();
    let tokens = ["ROOT", "{", "@PAGE", "(", "TITLE", ")", "{", "}", "}"];
    bytes.extend_from_slice(&(tokens.len() as u32).to_le_bytes());
    for token in tokens {
        let raw = token.as_bytes();
        bytes.extend_from_slice(&((raw.len() + 1) as u32).to_le_bytes());
        bytes.extend_from_slice(raw);
        bytes.push(0);
    }
    let projection = project(&bytes, "scb").expect("scb must expose a content view");
    let lines = projection.text.expect("must be a text view");
    assert!(lines.iter().any(|line| line.contains("@PAGE")));
    assert_eq!(lines.first().map(String::as_str), Some("ROOT {"));
}
