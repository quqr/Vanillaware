use std::collections::HashMap;
use std::path::Path;

use formats::raster::{paint_quad, Page, Raster};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = Path::new(&args[1]);
    let out = args[2].clone();
    let stage = args[3].clone();
    let x0: f32 = args[4].parse().unwrap();
    let width: f32 = args[5].parse().unwrap();
    let height: f32 = args[6].parse().unwrap();
    let y0: f32 = args[7].parse().unwrap();

    let bytes = std::fs::read(path).unwrap();
    let layout = formats::field::parse(&bytes).unwrap();
    let dir = path.parent().unwrap();
    let scale = 1200.0 / width.max(1.0);
    let (w, h) = ((width * scale) as u32, (height * scale) as u32);
    let mut raster = Raster::filled(w, h, [12, 14, 20, 255]);
    let mut props: HashMap<usize, Vec<Vec<formats::field::Piece>>> = HashMap::new();
    let mut pages: HashMap<usize, Vec<(u32, u32, Vec<u8>)>> = HashMap::new();
    let mut drawn = 0;
    for record in &layout.records {
        for layer in &record.layers {
            if layer.pair < 0 || layer.flags & 4 != 0 {
                continue;
            }
            let pair = &layout.pairs[layer.pair as usize];
            if pair.is_panel() || !formats::field::matches(&stage, pair) {
                continue;
            }
            if layer.max[0] < x0 || layer.min[0] > x0 + width {
                continue;
            }
            let key = layer.pair as usize;
            let list = props.entry(key).or_insert_with(|| {
                let name = match pair.model.rsplit(['/', '\\']).next() {
                    Some(v) => v,
                    None => return Vec::new(),
                };
                let bytes = match std::fs::read(dir.join(name)) {
                    Ok(v) => v,
                    Err(_) => return Vec::new(),
                };
                let v55 = match formats::mbs::parse_bytes(&bytes, "swi_unic") {
                    Ok(v) => v,
                    Err(_) => return Vec::new(),
                };
                let quad = match formats::mbs::convert(&v55) {
                    Ok(v) => v,
                    Err(_) => return Vec::new(),
                };
                formats::field::props(&quad)
            });
            let Some(pieces) = list.get(layer.panel as usize) else {
                continue;
            };
            let tex = pages.entry(key).or_insert_with(|| {
                let name = match pair.texture.rsplit(['/', '\\']).next() {
                    Some(v) => v,
                    None => return Vec::new(),
                };
                let bytes = match std::fs::read(dir.join(name)) {
                    Ok(v) => v,
                    Err(_) => return Vec::new(),
                };
                match formats::ftx::decode_bytes(&bytes) {
                    Ok(list) => list
                        .into_iter()
                        .map(|p| (p.width, p.height, p.rgba()))
                        .collect(),
                    Err(_) => Vec::new(),
                }
            });
            for piece in pieces {
                let Some((pw, ph, rgba)) = tex.get(piece.page) else {
                    continue;
                };
                let Some(page) = Page::new(*pw, *ph, rgba) else {
                    continue;
                };
                let corners = formats::field::piece_corners(layer, piece.dst);
                let mut dst = [(0.0f32, 0.0f32); 4];
                for corner in 0..4 {
                    let wx = layer.center_x + corners[corner].0;
                    let wy = layer.center_y + corners[corner].1;
                    dst[corner] = ((wx - x0) * scale, (y0 + height - wy) * scale);
                }
                paint_quad(&mut raster, dst, piece.uv, &page, piece.color);
            }
            // outline the layer box
            let box_dst = [
                (
                    (layer.min[0] - x0) * scale,
                    (y0 + height - layer.min[1]) * scale,
                ),
                (
                    (layer.max[0] - x0) * scale,
                    (y0 + height - layer.min[1]) * scale,
                ),
                (
                    (layer.max[0] - x0) * scale,
                    (y0 + height - layer.max[1]) * scale,
                ),
                (
                    (layer.min[0] - x0) * scale,
                    (y0 + height - layer.max[1]) * scale,
                ),
            ];
            for slot in 0..4 {
                let (ax, ay) = box_dst[slot];
                let (bx, by) = box_dst[(slot + 1) % 4];
                let steps = ((bx - ax).abs().max((by - ay).abs())) as i32;
                for step in 0..=steps.max(1) {
                    let t = step as f32 / steps.max(1) as f32;
                    let px = ax + (bx - ax) * t;
                    let py = ay + (by - ay) * t;
                    if px >= 0.0 && py >= 0.0 {
                        raster.blend(px as u32, py as u32, [255, 60, 60, 255]);
                    }
                }
            }
            drawn += 1;
        }
    }
    println!("{drawn} model layers");
    let image = image::RgbaImage::from_raw(w, h, raster.rgba).unwrap();
    image.save(&out).unwrap();
}
