use std::path::Path;

use formats::field;
use formats::raster::Page;

fn main() {
    let source = std::env::args().nth(1).unwrap();
    let path = Path::new(&source);
    let record_index: usize = std::env::args().nth(2).unwrap().parse().unwrap();
    let bytes = std::fs::read(path).unwrap();
    let layout = field::parse(&bytes).unwrap();
    let record = &layout.records[record_index];
    let dir = path.parent().unwrap();
    println!("record {} ext {}", record.index, record.ext_count);

    let mut uvs = Vec::new();
    for index in 0..record.ext_count {
        let verts = field::vertices(&bytes, record.ext_at + index * field::EXT, 6);
        let u = (verts[1].u + verts[3].u) * 0.5;
        let v = (verts[1].v + verts[3].v) * 0.5;
        uvs.push((u, v));
    }

    for (slot, pair) in layout.pairs.iter().enumerate() {
        let name = pair.texture.rsplit(['/', '\\']).next().unwrap();
        let Ok(data) = std::fs::read(dir.join(name)) else { continue };
        let Ok(pages) = formats::ftx::decode_bytes(&data) else { continue };
        let Some(first) = pages.first() else { continue };
        let rgba = first.rgba();
        let Some(page) = Page::new(first.width, first.height, &rgba) else { continue };
        let mut opaque = 0;
        let mut colors = Vec::new();
        for (u, v) in &uvs {
            let texel = page.sample(*u, *v);
            if texel[3] > 0 {
                opaque += 1;
                colors.push(texel);
            }
        }
        let avg = if colors.is_empty() {
            [0u8; 4]
        } else {
            let mut sum = [0u32; 4];
            for c in &colors {
                for i in 0..4 {
                    sum[i] += c[i] as u32;
                }
            }
            [
                (sum[0] / colors.len() as u32) as u8,
                (sum[1] / colors.len() as u32) as u8,
                (sum[2] / colors.len() as u32) as u8,
                (sum[3] / colors.len() as u32) as u8,
            ]
        };
        println!("{slot:2} {name:<20} {opaque}/{} opaque, avg {:?}", uvs.len(), avg);
    }
}
