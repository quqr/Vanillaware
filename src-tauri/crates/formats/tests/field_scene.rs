use formats::content::project;
use formats::field;
use formats::raster::{paint_quad, Page, Raster};

type Quad = [(f32, f32); 4];
type Panel = (u16, u16, Quad, Quad);

fn mpd_bytes(panels: &[Panel]) -> Vec<u8> {
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
    assert_eq!(layout.counts.records, 0);
    assert_eq!(layout.masks.len(), 1);
    assert_eq!(layout.masks[0].rect, [0, 0, 1, 0]);
    assert_eq!(layout.masks[0].marked, 2);
    assert_eq!(layout.masks[0].cells, 2);
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
    assert!(titles.contains(&"Layer sets (one battlefield stage each)"));
}

struct MaskSpec {
    world: [f32; 3],
    rect: [i32; 4],
    sample: u16,
    flag: u16,
    marked: bool,
}

fn ufb_masks(masks: &[MaskSpec]) -> Vec<u8> {
    let mut bytes = vec![0u8; 0x88];
    bytes[0..4].copy_from_slice(b"UCFB");
    bytes[0x54..0x58].copy_from_slice(&(masks.len() as u32).to_le_bytes());

    for (index, mask) in masks.iter().enumerate() {
        let width = (mask.rect[2] - mask.rect[0] + 1) as usize;
        let height = (mask.rect[3] - mask.rect[1] + 1) as usize;
        let mut block = vec![0u8; 0x3c];
        block[0..4].copy_from_slice(&mask.world[0].to_le_bytes());
        block[8..12].copy_from_slice(&mask.world[2].to_le_bytes());
        block[0x0c..0x10]
            .copy_from_slice(&(mask.world[0] + (width as f32 - 1.0) * 32.0).to_le_bytes());
        block[0x14..0x18]
            .copy_from_slice(&(mask.world[2] + (height as f32 - 1.0) * 32.0).to_le_bytes());
        block[0x24..0x28].copy_from_slice(&mask.rect[0].to_le_bytes());
        block[0x28..0x2c].copy_from_slice(&mask.rect[1].to_le_bytes());
        block[0x2c..0x30].copy_from_slice(&mask.rect[2].to_le_bytes());
        block[0x30..0x34].copy_from_slice(&mask.rect[3].to_le_bytes());
        block[0x34..0x36].copy_from_slice(&mask.sample.to_le_bytes());
        block[0x38..0x3a].copy_from_slice(&(index as u16).to_le_bytes());
        block[0x3a..0x3c].copy_from_slice(&mask.flag.to_le_bytes());
        bytes.extend_from_slice(&block);
        for _ in 0..width * height {
            let value = if mask.marked {
                0x6641_0000u32
            } else {
                0x6640_0000
            };
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes
}

#[test]
fn rasterise_maps_world_units_onto_the_ucb_grid() {
    assert_eq!(field::grid_x(field::WORLD_X0), 0.0);
    assert_eq!(field::grid_x(field::WORLD_X0 + 320.0), 10.0);
    assert_eq!(field::grid_y(field::WORLD_Z0 + 64.0), 2.0);

    let bytes = ufb_masks(&[
        MaskSpec {
            world: [field::WORLD_X0, 0.0, field::WORLD_Z0],
            rect: [0, 0, 1, 0],
            sample: 5,
            flag: 0x4600,
            marked: true,
        },
        MaskSpec {
            world: [field::WORLD_X0, 0.0, field::WORLD_Z0],
            rect: [0, 0, 1, 0],
            sample: 9,
            flag: 0x4700,
            marked: true,
        },
    ]);
    let layout = field::parse(&bytes).expect("layout");
    let grid = field::rasterise(&layout, &bytes);
    assert_eq!(
        grid.painted, 4,
        "the underlay paints first and is then overwritten"
    );
    assert_eq!(grid.underlay, 1);
    assert_eq!(grid.outside, 0);
    assert_eq!(grid.at(0, 0), 0);
    assert_eq!(grid.at(1, 0), 0);
    assert_eq!(grid.at(2, 0), 0);
}

#[test]
fn rasterise_lets_the_highest_sample_win() {
    let bytes = ufb_masks(&[
        MaskSpec {
            world: [field::WORLD_X0, 0.0, field::WORLD_Z0],
            rect: [0, 0, 1, 0],
            sample: 5,
            flag: 0x4600,
            marked: true,
        },
        MaskSpec {
            world: [field::WORLD_X0, 0.0, field::WORLD_Z0],
            rect: [0, 0, 1, 0],
            sample: 9,
            flag: 0x4600,
            marked: true,
        },
    ]);
    let layout = field::parse(&bytes).expect("layout");
    let grid = field::rasterise(&layout, &bytes);
    assert_eq!(grid.painted, 4);
    assert_eq!(grid.at(0, 0), 1);
    assert_eq!(grid.at(1, 0), 1);
}

#[test]
fn rasterise_counts_cells_that_fall_outside_the_grid() {
    let bytes = ufb_masks(&[MaskSpec {
        world: [field::WORLD_X0 - 32.0, 0.0, field::WORLD_Z0],
        rect: [0, 0, 2, 0],
        sample: 1,
        flag: 0x4600,
        marked: true,
    }]);
    let layout = field::parse(&bytes).expect("layout");
    let grid = field::rasterise(&layout, &bytes);
    assert_eq!(grid.outside, 1);
    assert_eq!(grid.painted, 2);
    assert_eq!(grid.at(0, 0), 0);
    assert_eq!(grid.at(1, 0), 0);
}

#[test]
fn tint_reads_the_layer_colour_as_rgba_in_file_order() {
    // the file stores R,G,B,A, so `0x8cffffff` is white at alpha 140 -- not a cyan tint
    assert_eq!(field::tint(0x8cff_ffff), [255, 255, 255, 140]);
    assert_eq!(field::tint(0xeaff_fff4), [244, 255, 255, 234]);
    assert_eq!(field::tint(0x00ff_ffff), [255, 255, 255, 0]);
}

#[test]
fn stages_group_under_their_battlefield() {
    let left = ufb_pair("FieldBG/AC00_00P.MPD", "FieldBG\\AC00_00P.ftx");
    let right = ufb_pair("FieldBG/AC00_02P.MPD", "FieldBG\\AC00_02P.ftx");
    let other = ufb_pair("FieldBG/AC10_00P.MPD", "FieldBG\\AC10_00P.ftx");
    let mut bytes = vec![0u8; 0x88];
    bytes[0..4].copy_from_slice(b"UCFB");
    bytes[0x60..0x64].copy_from_slice(&3u32.to_le_bytes());
    bytes.extend_from_slice(&left);
    bytes.extend_from_slice(&right);
    bytes.extend_from_slice(&other);

    let layout = field::parse(&bytes).expect("layout");
    let mut names: Vec<String> = layout.pairs.iter().filter_map(field::stage).collect();
    names.sort();
    assert_eq!(names, vec!["AC00_00P", "AC00_02P", "AC10_00P"]);

    let maps = field::maps(&layout);
    let stage: Vec<&str> = maps.iter().map(|entry| entry.name.as_str()).collect();
    assert!(stage.contains(&"AC00_00P"));
    assert!(stage.contains(&"AC00_02P"));
    assert!(stage.contains(&"AC10_00P"));
    let ac00: Vec<&str> = maps
        .iter()
        .filter(|entry| entry.group == "AC00")
        .map(|entry| entry.name.as_str())
        .collect();
    assert_eq!(ac00.len(), 2);

    let first = &layout.pairs[0];
    assert!(field::matches("AC00", first));
    assert!(field::matches("ac00_00p", first));
    assert!(!field::matches("AC00_02P", first));
    assert!(!field::matches("AC10", first));
}

fn ufb_pair(model: &str, texture: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for value in [model, texture] {
        out.extend_from_slice(value.as_bytes());
        out.push(0);
    }
    out
}

fn close(got: (f32, f32), want: (f32, f32), what: &str) {
    assert!(
        (got.0 - want.0).abs() < 0.01 && (got.1 - want.1).abs() < 0.01,
        "{what}: got {got:?}, want {want:?}"
    );
}

#[test]
fn a_panel_quad_is_the_page_rectangle_halved_scaled_and_rotated() {
    let panel = field::Panel {
        page: 0,
        corners: [
            (-200.0, -100.0),
            (200.0, -100.0),
            (200.0, 100.0),
            (-200.0, 100.0),
        ],
        uv: [(0.0, 0.0), (0.5, 0.0), (0.5, 0.5), (0.0, 0.5)],
    };

    let plain = field::Layer {
        scale_x: 4096,
        scale_y: 4096,
        ..Default::default()
    };
    let quad = field::quad_corners(&plain, &panel);
    close(quad[0], (-100.0, 50.0), "page units are twice texels");
    close(quad[2], (100.0, -50.0), "opposite corner");

    let wide = field::Layer {
        scale_x: 8192,
        ..plain
    };
    let quad = field::quad_corners(&wide, &panel);
    close(quad[0], (-200.0, 50.0), "scale x only stretches x");
    close(quad[2], (200.0, -50.0), "scale x keeps y");

    let tall = field::Layer {
        scale_y: 2048,
        ..plain
    };
    let quad = field::quad_corners(&tall, &panel);
    close(quad[0], (-100.0, 25.0), "scale y only stretches y");

    let quarter_turn = field::Layer {
        roll: 16384,
        ..plain
    };
    let quad = field::quad_corners(&quarter_turn, &panel);
    close(quad[0], (50.0, 100.0), "a quarter turn swaps the axes");
    close(
        quad[2],
        (-50.0, -100.0),
        "a quarter turn swaps both corners",
    );

    let half_turn = field::Layer {
        roll: -32768,
        ..plain
    };
    let quad = field::quad_corners(&half_turn, &panel);
    close(quad[0], (100.0, -50.0), "half a turn flips both axes");
    close(quad[2], (-100.0, 50.0), "half a turn flips both corners");

    let face_on = field::Layer {
        pitch: 16384,
        ..plain
    };
    let quad = field::quad_corners(&face_on, &panel);
    assert!(
        quad.iter().all(|corner| corner.1.abs() < 1e-3),
        "a quarter pitch turns the quad edge on"
    );

    let tilted = field::Layer {
        pitch: 8192,
        ..plain
    };
    let quad = field::quad_corners(&tilted, &panel);
    close(
        quad[2],
        (100.0, -35.35534),
        "a 45 degree pitch keeps cos(45) of the height",
    );
    assert!(
        (quad[0].0 + 100.0).abs() < 1e-3,
        "pitch leaves the horizontal extent alone"
    );
}

fn ufb_named(group: i16, samples: &[&str], group_names: &[&str]) -> Vec<u8> {
    let mut bytes = vec![0u8; 0x88];
    bytes[0..4].copy_from_slice(b"UCFB");
    bytes[0x54..0x58].copy_from_slice(&1u32.to_le_bytes());
    bytes[0x64..0x68].copy_from_slice(&(samples.len() as u32).to_le_bytes());
    bytes[0x68..0x6c].copy_from_slice(&(group_names.len() as u32).to_le_bytes());

    let mut block = vec![0u8; 0x3c];
    block[0..4].copy_from_slice(&field::WORLD_X0.to_le_bytes());
    block[8..12].copy_from_slice(&field::WORLD_Z0.to_le_bytes());
    block[0x0c..0x10].copy_from_slice(&(field::WORLD_X0 + 32.0).to_le_bytes());
    block[0x14..0x18].copy_from_slice(&(field::WORLD_Z0 + 32.0).to_le_bytes());
    block[0x2c..0x30].copy_from_slice(&1i32.to_le_bytes());
    block[0x30..0x34].copy_from_slice(&1i32.to_le_bytes());
    block[0x36..0x38].copy_from_slice(&group.to_le_bytes());
    bytes.extend_from_slice(&block);
    for _ in 0..4 {
        bytes.extend_from_slice(&0x6641_0000u32.to_le_bytes());
    }
    for name in samples.iter().chain(group_names.iter()) {
        bytes.extend_from_slice(name.as_bytes());
        bytes.push(0);
    }
    bytes
}

#[test]
fn masks_name_their_area_and_layers_name_their_prop() {
    let bytes = ufb_named(1, &["Gate_C08"], &["AbyssMist", "Elf_Barrier"]);
    let layout = field::parse(&bytes).expect("layout");
    assert_eq!(layout.area_name(&layout.masks[0]), Some("Elf_Barrier"));
    assert_eq!(
        layout.prop_name(&field::Layer {
            sample: 0,
            ..Default::default()
        }),
        Some("Gate_C08")
    );
    assert_eq!(
        layout.prop_name(&field::Layer {
            sample: field::SAMPLE_NONE,
            ..Default::default()
        }),
        None
    );

    let unnamed = ufb_named(-1, &[], &[]);
    let layout = field::parse(&unnamed).expect("layout");
    assert_eq!(layout.area_name(&layout.masks[0]), None);
}
