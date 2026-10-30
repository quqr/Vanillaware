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

fn ucb_bytes(cells: &[(u32, u32, u32)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (x, y, word) in cells {
        bytes.extend_from_slice(&x.to_le_bytes());
        bytes.extend_from_slice(&y.to_le_bytes());
        bytes.extend_from_slice(&word.to_le_bytes());
        bytes.extend_from_slice(&0xff00_0000u32.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 16]);
    }
    bytes
}

#[test]
fn ucb_draws_the_world_grid_by_area() {
    let cells = [
        (0u32, 0u32, 2u32),
        (1, 0, (7u32 << 22) | 2),
        (0, 1, (3u32 << 22) | 5),
        (1, 1, 0),
    ];
    let projection = project(&ucb_bytes(&cells), "ucb").expect("ucb must expose a content view");
    let color = projection.color.expect("ucb must draw a colour map");
    assert_eq!(color.width, 2);
    assert_eq!(color.height, 2);
    assert_eq!(color.rgba.len(), 16);
    assert!(color.legend.contains("2×2"));

    let grid = &projection.sections[0];
    assert_eq!(grid.rows[0][1], "4");
    assert_eq!(grid.rows[1][1], "2");
    assert_eq!(grid.rows[2][1], "2");

    let areas = projection
        .sections
        .iter()
        .find(|section| section.title.starts_with("Areas"))
        .expect("area table");
    assert_eq!(areas.total, 2);
    let classes = projection
        .sections
        .iter()
        .find(|section| section.title.starts_with("Terrain"))
        .expect("class table");
    assert_eq!(classes.total, 3);
}

#[test]
fn ufb_picks_a_mask_and_lists_resources() {
    let resources = [
        ("FieldBG/AC00_00P.MBS", "FieldBG\\AC00_00P.ftx"),
        ("FieldBG/AC01_00P.MPD", "FieldBG\\AC01_00P.ftx"),
    ];
    let width = 4usize;
    let height = 3usize;
    let mut bytes = vec![0u8; 0x88];
    bytes[0..4].copy_from_slice(b"UCFB");
    bytes[0x40..0x44].copy_from_slice(&0u32.to_le_bytes());
    bytes[0x54..0x58].copy_from_slice(&1u32.to_le_bytes());
    bytes[0x58..0x5c].copy_from_slice(&0u32.to_le_bytes());
    bytes[0x5c..0x60].copy_from_slice(&0u32.to_le_bytes());
    bytes[0x60..0x64].copy_from_slice(&(resources.len() as u32).to_le_bytes());

    let mut mask = vec![0u8; 0x3c];
    mask[0x24..0x28].copy_from_slice(&0i32.to_le_bytes());
    mask[0x28..0x2c].copy_from_slice(&0i32.to_le_bytes());
    mask[0x2c..0x30].copy_from_slice(&((width - 1) as i32).to_le_bytes());
    mask[0x30..0x34].copy_from_slice(&((height - 1) as i32).to_le_bytes());
    for index in 0..width * height {
        let value = if index % 2 == 0 {
            0x6641_0000u32
        } else {
            0x6640_0000
        };
        mask.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&mask);
    for (left, right) in resources {
        bytes.extend_from_slice(left.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(right.as_bytes());
        bytes.push(0);
    }

    let projection = project(&bytes, "ufb").expect("ufb must expose a content view");
    let color = projection.color.expect("ufb must draw a mask");
    assert_eq!(color.width, width as u32);
    assert_eq!(color.height, height as u32);

    let area = projection
        .sections
        .iter()
        .find(|section| section.title.starts_with("Area masks"))
        .expect("mask table");
    assert_eq!(area.total, 1);
    assert_eq!(area.rows[0][3], "50.0%");

    let listed = projection
        .sections
        .iter()
        .find(|section| section.title == "Resource pairs")
        .expect("resource table");
    assert_eq!(listed.rows.len(), 2);
    assert_eq!(listed.rows[0][0], "FieldBG/AC00_00P.MBS");
}
