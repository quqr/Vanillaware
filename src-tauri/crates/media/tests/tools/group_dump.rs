use formats::field;

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let record_index: usize = std::env::args().nth(2).unwrap().parse().unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let layout = field::parse(&bytes).unwrap();
    let record = &layout.records[record_index];
    println!(
        "record {} box ({:.0},{:.0})..({:.0},{:.0}) layers {} ext {}",
        record.index,
        record.min[0], record.min[2], record.max[0], record.max[2],
        record.layers.len(),
        record.ext_count
    );
    let mut cursor = record.groups_at;
    for block in &record.groups {
        println!("group {} slot {} count {}", block.id, block.slot, block.count);
        for entry in 0..block.count as usize {
            let at = cursor + field::GROUP_HEAD + entry * field::GROUP_ENTRY;
            let verts = field::vertices(&bytes, at, 10);
            let (mut lx, mut ly, mut hx, mut hy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
            for v in &verts {
                lx = lx.min(v.x); ly = ly.min(v.y);
                hx = hx.max(v.x); hy = hy.max(v.y);
            }
            println!(
                "  entry {entry}: {:.0}..{:.0} x {:.0}..{:.0} | v0 ({:.1},{:.1}) uv ({:.3},{:.3}) color {:08x} | v1 ({:.1},{:.1}) uv ({:.3},{:.3})",
                lx, hx, ly, hy,
                verts[0].x, verts[0].y, verts[0].u, verts[0].v, verts[0].color,
                verts[1].x, verts[1].y, verts[1].u, verts[1].v
            );
        }
        cursor += field::GROUP_HEAD + block.count as usize * field::GROUP_ENTRY;
    }
}
