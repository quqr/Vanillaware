use formats::field;

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let record_index: usize = std::env::args().nth(2).unwrap().parse().unwrap();
    let start: usize = std::env::args().nth(3).and_then(|v| usize::from_str_radix(&v, 16).ok()).unwrap_or(0);
    let count: usize = std::env::args().nth(4).and_then(|v| v.parse().ok()).unwrap_or(40);
    let bytes = std::fs::read(&path).unwrap();
    let layout = field::parse(&bytes).unwrap();
    let record = &layout.records[record_index];
    println!("record {} at {:#x} ext_at {:#x} groups_at {:#x}", record.index, record.at, record.ext_at, record.groups_at);
    let at = record.groups_at + start;
    for index in 0..count {
        let base = at + index * 0x18;
        let v = field::vertices(&bytes, base, 1)[0];
        println!("{index:3} +{:#05x}: ({:10.2},{:10.2}) zero {:08x} color {:08x} uv ({:8.4},{:8.4})", index * 0x18, v.x, v.y, u32::from_le_bytes(bytes[base + 8..base + 12].try_into().unwrap()), v.color, v.u, v.v);
    }
}
