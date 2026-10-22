//! 文档树用的最小读取器：只做带边界检查的小端整数与展示格式化。
//!
//! 不引入 byteorder / nom；格式解析的边界错误由调用方转成 `warn` / `check`。

pub fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    let slice = bytes.get(at..at + 2)?;
    Some(u16::from_le_bytes([slice[0], slice[1]]))
}

pub fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    let slice = bytes.get(at..at + 4)?;
    Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
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

/// 以 NUL 截断的可打印字符串（Shift-JIS 尚未解码，保留原始字节）。
pub fn c_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&byte| byte == 0).unwrap_or(bytes.len());
    ascii(&bytes[..end])
}
