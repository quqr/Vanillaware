fn main() {
    let path = std::env::args().nth(1).unwrap();
    let file_offset = usize::from_str_radix(&std::env::args().nth(2).unwrap(), 16).unwrap();
    let length: usize = std::env::args().nth(3).and_then(|v| v.parse().ok()).unwrap_or(256);
    let bytes = std::fs::read(&path).unwrap();
    let mut at = file_offset;
    while at < file_offset + length {
        let mut line = format!("{at:#010x} ");
        for slot in 0..8 {
            let base = at + slot * 4;
            if base + 4 > bytes.len() { break; }
            let raw = u32::from_le_bytes(bytes[base..base + 4].try_into().unwrap());
            line.push_str(&format!("{raw:08x} "));
        }
        line.push_str(" | ");
        for slot in 0..8 {
            let base = at + slot * 4;
            if base + 4 > bytes.len() { break; }
            let value = f32::from_le_bytes(bytes[base..base + 4].try_into().unwrap());
            if value.abs() > 1e-4 && value.abs() < 1e7 {
                line.push_str(&format!("{value:10.2} "));
            } else {
                line.push_str(&format!("{:>10} ", "-"));
            }
        }
        println!("{line}");
        at += 32;
    }
}
