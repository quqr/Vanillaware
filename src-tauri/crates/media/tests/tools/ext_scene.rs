use std::collections::HashMap;
use std::path::Path;

use formats::field;
use formats::raster::{paint_quad, Page, Raster};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ufb = Path::new(&args[1]);
    let out = args[2].clone();
    let x0: f32 = args[3].parse().unwrap();
    let width: f32 = args[4].parse().unwrap();
    let height: f32 = args[5].parse().unwrap();
    let base_y: f32 = args.get(6).and_then(|v| v.parse().ok()).unwrap_or(64.0);
    let texture = args.get(7).cloned().unwrap_or_else(|| "AC00_00P.ftx".to_string());

    let bytes = std::fs::read(ufb).unwrap();
    let layout = field::parse(&bytes).unwrap();
    let dir = ufb.parent().unwrap();
    let scale = 1400.0 / width.max(1.0);
    let (w, h) = ((width * scale) as u32, (height * scale) as u32);
    let mut raster = Raster::filled(w, h, [26, 30, 40, 255]);

    let mut pages: HashMap<String, Vec<(u32, u32, Vec<u8>)>> = HashMap::new();
    let mut drawn = 0;
    let mut records = 0;
    for record in &layout.records {
        if record.ext_count == 0 {
            continue;
        }
        if record.max[0] < x0 || record.min[0] > x0 + width {
            continue;
        }
        records += 1;
        let cols = ((record.max[0] - record.min[0]).abs() / 128.0).round().max(1.0) as usize;
        let rows = ((record.max[2] - record.min[2]).abs() / 128.0).round().max(1.0) as usize;
        let list = pages.entry(texture.clone()).or_insert_with(|| {
            let data = std::fs::read(dir.join(&texture)).unwrap();
            formats::ftx::decode_bytes(&data)
                .unwrap()
                .into_iter()
                .map(|page| (page.width, page.height, page.rgba()))
                .collect()
        });
        let Some((pw, ph, rgba)) = list.first() else { continue };
        let Some(page) = Page::new(*pw, *ph, rgba) else { continue };
        for index in 0..record.ext_count.min(cols * rows) {
            let verts = field::vertices(&bytes, record.ext_at + index * field::EXT, 6);
            let cell_x = record.min[0] + 64.0 + (index % cols) as f32 * 128.0;
            let cell_z = record.min[2] + 64.0 + (index / cols) as f32 * 128.0;
            let _ = cell_z;
            let mut dst = [(0.0f32, 0.0f32); 4];
            for (slot, corner) in [1usize, 2, 3, 4].iter().enumerate() {
                let v = &verts[*corner];
                let world_x = cell_x + v.x;
                let world_y = base_y + v.y;
                dst[slot] = ((world_x - x0) * scale, (height * 0.5 - world_y) * scale);
            }
            let uv = [
                (verts[1].u, verts[1].v),
                (verts[2].u, verts[2].v),
                (verts[3].u, verts[3].v),
                (verts[4].u, verts[4].v),
            ];
            paint_quad(&mut raster, dst, uv, &page, [255, 255, 255, 255]);
            drawn += 1;
        }
    }
    println!("{records} records, {drawn} ext quads");
    let image = image::RgbaImage::from_raw(w, h, raster.rgba).unwrap();
    image.save(&out).unwrap();
}
