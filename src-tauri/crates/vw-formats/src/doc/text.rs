//! 文本槽解码：Shift-JIS（cp932）优先，UTF-16LE 用于 fms。

use encoding_rs::SHIFT_JIS;

/// 定长槽：在第一个 NUL 处截断，按 Shift-JIS 解码；失败时做有损回退。
pub fn sjis_slot(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&byte| byte == 0).unwrap_or(bytes.len());
    let (text, _, _) = SHIFT_JIS.decode(&bytes[..end]);
    text.into_owned()
}

/// UTF-8 优先、Shift-JIS 回退（fms / rsc 的字符串池）。
pub fn utf8_or_sjis(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&byte| byte == 0).unwrap_or(bytes.len());
    let raw = &bytes[..end];
    match std::str::from_utf8(raw) {
        Ok(text) => text.to_string(),
        Err(_) => {
            let (text, _, _) = SHIFT_JIS.decode(raw);
            text.into_owned()
        }
    }
}

/// UTF-16LE 解码（调用方保证偶数长度）。
pub fn utf16le(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    String::from_utf16_lossy(&units)
}
