use std::collections::HashMap;

use formats::field;

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let limit: usize = std::env::args()
        .nth(2)
        .and_then(|v| v.parse().ok())
        .unwrap_or(8);
    let bytes = std::fs::read(&path).unwrap();
    let layout = field::parse(&bytes).unwrap();

    let mut users: HashMap<(usize, usize), usize> = HashMap::new();
    let mut total = 0usize;
    let mut shown = 0usize;
    let mut box_match = 0usize;
    let mut scale_one = 0usize;
    let mut color_uniform = 0usize;
    let mut uv_pages: HashMap<u32, usize> = HashMap::new();
    let mut sizes: HashMap<(i32, i32), usize> = HashMap::new();
    let mut degenerate = 0usize;

    for record in &layout.records {
        for layer in &record.layers {
            let ext = layer.ext as usize;
            if ext >= record.ext_count {
                continue;
            }
            total += 1;
            *users.entry((record.index, ext)).or_insert(0) += 1;
            let verts = field::vertices(&bytes, record.ext_at + ext * field::EXT, 6);
            let mut low = (f32::MAX, f32::MAX);
            let mut high = (f32::MIN, f32::MIN);
            for v in &verts {
                low = (low.0.min(v.x), low.1.min(v.y));
                high = (high.0.max(v.x), high.1.max(v.y));
            }
            let (w, h) = ((high.0 - low.0).round() as i32, (high.1 - low.1).round() as i32);
            *sizes.entry((w, h)).or_insert(0) += 1;
            if layer.scale_x == 4096 && layer.scale_y == 4096 {
                scale_one += 1;
            }
            let box_w = (layer.max[0] - layer.min[0]).abs();
            let box_h = (layer.max[1] - layer.min[1]).abs();
            if (box_w - w as f32).abs() < 1.0 && (box_h - h as f32).abs() < 1.0 {
                box_match += 1;
            }
            let color = verts[0].color;
            if verts.iter().all(|v| v.color == color) {
                color_uniform += 1;
            }
            for v in &verts {
                *uv_pages
                    .entry(((v.u * 1024.0) as u32) << 16 | (v.v * 1024.0) as u32)
                    .or_insert(0) += 1;
            }
            let area = {
                let a = (verts[1].x - verts[0].x) * (verts[2].y - verts[0].y)
                    - (verts[1].y - verts[0].y) * (verts[2].x - verts[0].x);
                a.abs()
            };
            if area < 1.0 {
                degenerate += 1;
            }
            if shown < limit {
                println!(
                    "rec {} layer {:?} pair {} ext {} | box {:.1}x{:.1} scale ({},{}) roll {} pitch {} | verts {:.1}..{:.1} x {:.1}..{:.1} | tri area {:.0} | uv ({:.3},{:.3})..({:.3},{:.3}) | color {:08x} uniform {}",
                    record.index,
                    layer.panel,
                    layer.pair,
                    ext,
                    box_w,
                    box_h,
                    layer.scale_x,
                    layer.scale_y,
                    layer.roll,
                    layer.pitch,
                    low.0,
                    high.0,
                    low.1,
                    high.1,
                    area,
                    verts.iter().map(|v| v.u).fold(f32::MAX, f32::min),
                    verts.iter().map(|v| v.v).fold(f32::MAX, f32::min),
                    verts.iter().map(|v| v.u).fold(f32::MIN, f32::max),
                    verts.iter().map(|v| v.v).fold(f32::MIN, f32::max),
                    color,
                    verts.iter().all(|v| v.color == color),
                );
                shown += 1;
            }
        }
    }

    let shared = users.values().filter(|count| **count > 1).count();
    println!("\n{total} layers with an extension block, {shared} blocks shared by more than one layer");
    println!("box == vertex bbox {box_match}/{total}, scale 4096 {scale_one}, uniform colour {color_uniform}, degenerate {degenerate}");
    let mut size_list: Vec<_> = sizes.into_iter().collect();
    size_list.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    println!("sizes {:?}", &size_list[..size_list.len().min(8)]);
    println!("uv cells {}", uv_pages.len());
}
