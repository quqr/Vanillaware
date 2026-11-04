use std::path::Path;

use formats::field;
use formats::raster::{paint_quad, Page, Raster};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ufb = Path::new(&args[1]);
    let record_index: usize = args[2].parse().unwrap();
    let texture = args[3].clone();
    let out = args[4].clone();
    let cell: f32 = args.get(5).and_then(|v| v.parse().ok()).unwrap_or(128.0);

    let bytes = std::fs::read(ufb).unwrap();
    let layout = field::parse(&bytes).unwrap();
    let record = &layout.records[record_index];
    let dir = ufb.parent().unwrap();
    let tex = std::fs::read(dir.join(&texture)).unwrap();
    let pages: Vec<(u32, u32, Vec<u8>)> = formats::ftx::decode_bytes(&tex)
        .unwrap()
        .into_iter()
        .map(|page| (page.width, page.height, page.rgba()))
        .collect();
    let (pw, ph, rgba) = &pages[0];
    let page = Page::new(*pw, *ph, rgba).unwrap();

    let cols = ((record.max[0] - record.min[0]).abs() / cell)
        .round()
        .max(1.0) as usize;
    let rows = ((record.max[2] - record.min[2]).abs() / cell)
        .round()
        .max(1.0) as usize;
    println!(
        "record {} ext {} grid {}x{} cell {}",
        record.index, record.ext_count, cols, rows, cell
    );

    let scale = 2.0f32;
    let width = (cols as f32 * cell * scale) as u32;
    let height = (rows as f32 * cell * scale) as u32;
    let mut raster = Raster::filled(width, height, [16, 16, 24, 255]);

    for index in 0..record.ext_count.min(cols * rows) {
        let verts = field::vertices(&bytes, record.ext_at + index * field::EXT, 6);
        let col = index % cols;
        let row = index / cols;
        let base_x = col as f32 * cell;
        let base_y = row as f32 * cell;
        let map = |x: f32, y: f32| {
            (
                (base_x + cell * 0.5 + x) * scale,
                (base_y + cell * 0.5 + y) * scale,
            )
        };
        let (p1, p2, p3, p4) = (
            map(verts[1].x, verts[1].y),
            map(verts[2].x, verts[2].y),
            map(verts[3].x, verts[3].y),
            map(verts[4].x, verts[4].y),
        );
        let dst = [p1, p2, p3, p4];
        let uv = [
            (verts[1].u, verts[1].v),
            (verts[2].u, verts[2].v),
            (verts[3].u, verts[3].v),
            (verts[4].u, verts[4].v),
        ];
        paint_quad(&mut raster, dst, uv, &page, [255, 255, 255, 255]);
    }

    let image = image::RgbaImage::from_raw(width, height, raster.rgba).unwrap();
    image.save(&out).unwrap();
}
