use formats::content::project;
use formats::field;
use formats::raster::{paint_quad, Page, Raster};

fn mpd_bytes(panels: &[(u16, u16, [(f32, f32); 4], [(f32, f32); 4])]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"MPD\0");
    bytes.extend_from_slice(&((panels.len() as u32) * 0xa0).to_le_bytes());
    bytes.extend_from_slice(&0x20u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&(panels.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    for (index, (page, rotation, corners, uv)) in panels.iter().enumerate() {
        bytes.extend_from_slice(&(index as u32).to_le_bytes());
        bytes.extend_from_slice(&page.to_le_bytes());
        bytes.extend_from_slice(&rotation.to_le_bytes());
        for corner in 0..4 {
            bytes.extend_from_slice(&corners[corner].0.to_le_bytes());
            bytes.extend_from_slice(&corners[corner].1.to_le_bytes());
            bytes.extend_from_slice(&0f32.to_le_bytes());
            bytes.extend_from_slice(&0xffff_ffffu32.to_le_bytes());
            bytes.extend_from_slice(&uv[corner].0.to_le_bytes());
            bytes.extend_from_slice(&uv[corner].1.to_le_bytes());
        }
        let mut name = [0u8; 0x38];
        name[..1].copy_from_slice(b"0");
        bytes.extend_from_slice(&name);
    }
    bytes
}

#[test]
fn panels_expose_the_atlas_page_and_uv() {
    let quads = [(
        3u16,
        1u16,
        [
            (-10.0f32, -20.0f32),
            (10.0, -20.0),
            (10.0, 20.0),
            (-10.0, 20.0),
        ],
        [(0.0f32, 0.0f32), (0.25, 0.0), (0.25, 0.5), (0.0, 0.5)],
    )];
    let bytes = mpd_bytes(&quads);
    let list = field::panels(&bytes);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].page, 3);
    assert_eq!(list[0].corners[2], (10.0, 20.0));
    assert_eq!(list[0].uv[2], (0.25, 0.5));
}

#[test]
fn panels_ignore_files_that_are_not_panels() {
    assert!(field::panels(b"MBS\0\x01\x02\x03\x04").is_empty());
}

#[test]
fn paint_quad_fills_the_target_with_the_tinted_texel() {
    let page_rgba = vec![
        200u8, 100, 50, 255, 200, 100, 50, 255, 200, 100, 50, 255, 200, 100, 50, 255,
    ];
    let page = Page::new(2, 2, &page_rgba).expect("page");
    let mut raster = Raster::new(4, 4);
    paint_quad(
        &mut raster,
        [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)],
        [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)],
        &page,
        [128, 255, 255, 255],
    );
    let center = ((2 * 4 + 2) * 4) as usize;
    assert_eq!(&raster.rgba[center..center + 3], &[100, 100, 50]);
    assert_eq!(raster.rgba[center + 3], 255);
}

#[test]
fn paint_quad_keeps_the_canvas_empty_outside_the_quad() {
    let page_rgba = vec![255u8; 4];
    let page = Page::new(1, 1, &page_rgba).expect("page");
    let mut raster = Raster::new(8, 8);
    paint_quad(
        &mut raster,
        [(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)],
        [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)],
        &page,
        [255, 255, 255, 255],
    );
    assert_eq!(raster.rgba[0], 255);
    let corner = ((7 * 8 + 7) * 4) as usize;
    assert_eq!(raster.rgba[corner + 3], 0);
}

fn ufb_bytes(area: [i32; 4], marked: bool, pair: (&str, &str)) -> Vec<u8> {
    let mut bytes = vec![0u8; 0x88];
    bytes[0..4].copy_from_slice(b"UCFB");
    bytes[0x40..0x44].copy_from_slice(&0u32.to_le_bytes());
    bytes[0x54..0x58].copy_from_slice(&1u32.to_le_bytes());
    bytes[0x60..0x64].copy_from_slice(&1u32.to_le_bytes());

    let width = (area[2] - area[0] + 1) as usize;
    let height = (area[3] - area[1] + 1) as usize;
    let mut block = vec![0u8; 0x3c];
    block[0x24..0x28].copy_from_slice(&area[0].to_le_bytes());
    block[0x28..0x2c].copy_from_slice(&area[1].to_le_bytes());
    block[0x2c..0x30].copy_from_slice(&area[2].to_le_bytes());
    block[0x30..0x34].copy_from_slice(&area[3].to_le_bytes());
    bytes.extend_from_slice(&block);
    for _ in 0..width * height {
        let value = if marked { 0x6641_0000u32 } else { 0x6640_0000 };
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for name in [pair.0, pair.1] {
        bytes.extend_from_slice(name.as_bytes());
        bytes.push(0);
    }
    bytes
}

#[test]
fn layout_reads_areas_and_resource_pairs() {
    let bytes = ufb_bytes([0, 0, 1, 0], true, ("FieldBG/A.MPD", "FieldBG\\A.ftx"));
    let layout = field::parse(&bytes).expect("layout");
    assert_eq!(layout.header.records, 0);
    assert_eq!(layout.areas.len(), 1);
    assert_eq!(layout.areas[0].rect, [0, 0, 1, 0]);
    assert_eq!(layout.areas[0].marked, 2);
    assert_eq!(layout.areas[0].cells, 2);
    assert_eq!(layout.pairs.len(), 1);
    assert_eq!(layout.pairs[0].model, "FieldBG/A.MPD");
    assert_eq!(layout.pairs[0].texture, "FieldBG\\A.ftx");
}

#[test]
fn view_without_assets_paints_the_area_mask() {
    let bytes = ufb_bytes([0, 0, 3, 3], true, ("FieldBG/A.MPD", "FieldBG\\A.ftx"));
    let projection = project(&bytes, "ufb").expect("ufb projection");
    let color = projection.color.expect("mask image");
    assert_eq!(color.width, 4);
    assert_eq!(color.height, 4);
    assert!(color.legend.contains("Area mask"));
    let titles: Vec<&str> = projection
        .sections
        .iter()
        .map(|section| section.title.as_str())
        .collect();
    assert!(titles.contains(&"Resource pairs"));
    assert!(titles.contains(&"Scene records (by layer count)"));
}
