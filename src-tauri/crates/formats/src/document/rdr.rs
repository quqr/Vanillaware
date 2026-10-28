pub fn f32_at(bytes: &[u8], at: usize) -> Option<f32> {
    u32_at(bytes, at).map(f32::from_bits)
}

pub fn u64_at(bytes: &[u8], at: usize) -> Option<u64> {
    let slice = bytes.get(at..at + 8)?;
    Some(u64::from_le_bytes([
        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
    ]))
}

pub fn u8_at(bytes: &[u8], at: usize) -> Option<u8> {
    bytes.get(at).copied()
}

pub fn i8_at(bytes: &[u8], at: usize) -> Option<i8> {
    bytes.get(at).map(|&byte| byte as i8)
}

pub fn i32_at(bytes: &[u8], at: usize) -> Option<i32> {
    u32_at(bytes, at).map(|value| value as i32)
}

pub fn i16_at(bytes: &[u8], at: usize) -> Option<i16> {
    u16_at(bytes, at).map(|value| value as i16)
}

pub fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    let slice = bytes.get(at..at + 2)?;
    Some(u16::from_le_bytes([slice[0], slice[1]]))
}

pub fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    let slice = bytes.get(at..at + 4)?;
    Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

pub fn u16_be_at(bytes: &[u8], at: usize) -> Option<u16> {
    let slice = bytes.get(at..at + 2)?;
    Some(u16::from_be_bytes([slice[0], slice[1]]))
}

pub fn u32_be_at(bytes: &[u8], at: usize) -> Option<u32> {
    let slice = bytes.get(at..at + 4)?;
    Some(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

pub fn f32_be_at(bytes: &[u8], at: usize) -> Option<f32> {
    u32_be_at(bytes, at).map(f32::from_bits)
}

pub fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn ascii(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&byte| {
            if (0x20..=0x7e).contains(&byte) {
                byte as char
            } else {
                '·'
            }
        })
        .collect()
}

pub fn utf8_string(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

pub fn c_string(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len());
    ascii(&bytes[..end])
}
