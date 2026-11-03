use std::collections::HashMap;

use formats::field;

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let layout = field::parse(&bytes).unwrap();
    let mut delta: HashMap<i32, usize> = HashMap::new();
    let mut center_zero = 0usize;
    let mut closing = 0usize;
    let mut blocks = 0usize;
    let mut colors: HashMap<bool, usize> = HashMap::new();
    for record in &layout.records {
        if record.ext_count == 0 {
            continue;
        }
        let cols = ((record.max[0] - record.min[0]).abs() / 128.0).round().max(1.0) as i32;
        let rows = ((record.max[2] - record.min[2]).abs() / 128.0).round().max(1.0) as i32;
        *delta.entry(record.ext_count as i32 - cols * rows).or_insert(0) += 1;
        for index in 0..record.ext_count {
            let verts = field::vertices(&bytes, record.ext_at + index * field::EXT, 6);
            blocks += 1;
            if verts[0].x == 0.0 && verts[0].y == 0.0 {
                center_zero += 1;
            }
            if verts[5].x == verts[1].x
                && verts[5].y == verts[1].y
                && verts[5].u == verts[1].u
                && verts[5].v == verts[1].v
            {
                closing += 1;
            }
            let uniform = verts.iter().all(|v| v.color == verts[0].color);
            *colors.entry(uniform).or_insert(0) += 1;
        }
    }
    let mut list: Vec<_> = delta.into_iter().collect();
    list.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    println!("delta count - cols*rows: {:?}", &list[..list.len().min(10)]);
    println!("{blocks} blocks, centre at origin {center_zero}, v5 repeats v1 {closing}, colours {colors:?}");
}
