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
    let low: f32 = args.get(5).and_then(|v| v.parse().ok()).unwrap_or(-600.0);
    let high: f32 = args.get(6).and_then(|v| v.parse().ok()).unwrap_or(600.0);
    let max_side: u32 = args.get(7).and_then(|v| v.parse().ok()).unwrap_or(1400);
    let stage = args.get(8).cloned();
    let legacy = args.get(9).map(|value| value == "legacy").unwrap_or(false);

    let bytes = std::fs::read(ufb).unwrap();
    let layout = field::parse(&bytes).unwrap();
    let dir = ufb.parent().unwrap();
    let scale = max_side as f32 / width.max(1.0);
    let (w, h) = (
        (width * scale) as u32,
        ((high - low) * scale).max(1.0) as u32,
    );
    let mut raster = Raster::filled(w, h, [8, 10, 14, 255]);
    let mut pages: HashMap<usize, Vec<(u32, u32, Vec<u8>)>> = HashMap::new();
    let mut carry: Option<usize> = None;
    let mut drawn = 0usize;
    for record in &layout.records {
        let mut tally: HashMap<usize, usize> = HashMap::new();
        for layer in &record.layers {
            if layer.pair < 0 {
                continue;
            }
            let slot = layer.pair as usize;
            if layout
                .pairs
                .get(slot)
                .map(field::Pair::is_panel)
                .unwrap_or(false)
            {
                *tally.entry(slot).or_insert(0) += 1;
            }
        }
        if let Some((slot, _)) = tally.into_iter().max_by_key(|(_, count)| *count) {
            carry = Some(slot);
        }
        if record.max[0] < x0 || record.min[0] > x0 + width {
            continue;
        }
        let Some(slot) = carry else { continue };
        let Some(pair) = layout.pairs.get(slot) else {
            continue;
        };
        if let Some(want) = &stage {
            if !field::matches(want, pair) {
                continue;
            }
        }
        let list = pages.entry(slot).or_insert_with(|| {
            field::read_pair(dir, pair)
                .and_then(|(_, texture)| formats::ftx::decode_bytes(&texture).ok())
                .map(|pages| {
                    pages
                        .into_iter()
                        .map(|page| (page.width, page.height, page.rgba()))
                        .collect()
                })
                .unwrap_or_default()
        });
        let Some((pw, ph, rgba)) = list.first() else {
            continue;
        };
        let Some(page) = Page::new(*pw, *ph, rgba) else {
            continue;
        };
        if legacy {
            for (index, block) in field::ext_blocks(&bytes, record).iter().enumerate().rev() {
                let (world_x, _, _) = field::ext_center(record, index);
                let mut dst = [(0.0f32, 0.0f32); 4];
                let mut uv = [(0.0f32, 0.0f32); 4];
                for corner in 0..4 {
                    let vertex = block.corners[corner];
                    dst[corner] = ((world_x + vertex.x - x0) * scale, (high - vertex.y) * scale);
                    uv[corner] = (vertex.u, vertex.v);
                }
                paint_quad(&mut raster, dst, uv, &page, field::tint(block.center.color));
                drawn += 1;
            }
            continue;
        }
        for block in &record.groups {
            for quad in field::ground_quads(&bytes, block) {
                let mut dst = [(0.0f32, 0.0f32); 4];
                for corner in 0..4 {
                    dst[corner] = (
                        (quad.at.0 + quad.corners[corner].0 - x0) * scale,
                        (high - quad.corners[corner].1) * scale,
                    );
                }
                paint_quad(&mut raster, dst, quad.uv, &page, field::tint(quad.color));
                drawn += 1;
            }
        }
    }
    println!("{drawn} ground pieces");
    let image = image::RgbaImage::from_raw(w, h, raster.rgba).unwrap();
    image.save(&out).unwrap();
}
