use formats::field;

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let record_index: usize = std::env::args().nth(2).unwrap().parse().unwrap();
    let ext_index: usize = std::env::args().nth(3).unwrap().parse().unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let layout = field::parse(&bytes).unwrap();
    let record = &layout.records[record_index];
    println!(
        "record {} at {:#x} size {:#x} layers {} ext {} groups {}",
        record.index, record.at, record.size, record.layers.len(), record.ext_count, record.groups.len()
    );
    for (slot, layer) in record.layers.iter().enumerate() {
        println!(
            "  layer {slot}: pair {} panel {} ext {} flags {:#06x} box ({:.0},{:.0})..({:.0},{:.0}) center ({:.0},{:.0},{:.0}) scale ({},{}) roll {} pitch {} band {} slot {}",
            layer.pair, layer.panel, layer.ext, layer.flags,
            layer.min[0], layer.min[1], layer.max[0], layer.max[1],
            layer.center_x, layer.center_y, layer.center_z,
            layer.scale_x, layer.scale_y, layer.roll, layer.pitch, layer.band, layer.slot
        );
    }
    for ext in 0..record.ext_count {
        let verts = field::vertices(&bytes, record.ext_at + ext * field::EXT, 6);
        println!("ext {ext}:");
        for (index, v) in verts.iter().enumerate() {
            println!("  v{index}: ({:7.2},{:7.2}) uv ({:.4},{:.4}) color {:08x}", v.x, v.y, v.u, v.v, v.color);
        }
    }
}
