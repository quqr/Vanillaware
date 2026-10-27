//! 文本槽解码：Shift-JIS（cp932）优先，UTF-16LE 用于 fms。

use encoding_rs::SHIFT_JIS;

/// 定长槽：在第一个 NUL 处截断，按 Shift-JIS 解码；失败时做有损回退。
pub fn sjis_slot(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len());
    let (text, _, _) = SHIFT_JIS.decode(&bytes[..end]);
    text.into_owned()
}

/// UTF-8 优先、Shift-JIS 回退（fms / rsc 的字符串池）。
pub fn utf8_or_sjis(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len());
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

const MAX_LINES_SHOWN: usize = 256;

/// 明文资源的文档视图（`.inc` 这类开发期枚举表）。
///
/// 不做结构化假装：只判编码、切行、把前 256 行作为文本节点挂出来。
pub fn build_plain(bytes: &[u8], title: &str) -> super::arena::DocArena {
    use super::arena::Builder;

    let mut builder = Builder::new();
    let root = builder.root(title);
    builder.field(root, "size", bytes.len().to_string());

    let has_bom = bytes.starts_with(&[0xef, 0xbb, 0xbf]);
    let body = if has_bom { &bytes[3..] } else { bytes };
    let utf8 = std::str::from_utf8(body).is_ok();
    let text = if utf8 {
        String::from_utf8_lossy(body).into_owned()
    } else {
        let (decoded, _, _) = SHIFT_JIS.decode(body);
        decoded.into_owned()
    };

    let crlf = text.contains("\r\n");
    builder.field(root, "encoding", if utf8 { "UTF-8" } else { "Shift-JIS" });
    builder.field(root, "bom", has_bom.to_string());
    builder.field(root, "lineEnding", if crlf { "CRLF" } else { "LF" });

    let mut lines: Vec<&str> = text.split('\n').collect();
    if lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    builder.field(root, "lines", lines.len().to_string());
    builder.check(
        "text.lines",
        "按换行切出的行数 > 0",
        !lines.is_empty(),
        Some(format!("{} 行", lines.len())),
    );

    let group = builder.array(
        root,
        Some("lines"),
        format!("Lines ({}，仅前 {MAX_LINES_SHOWN})", lines.len()),
    );
    for (index, line) in lines.iter().take(MAX_LINES_SHOWN).enumerate() {
        let value = line.trim_end_matches('\r');
        builder.text(
            group,
            &index.to_string(),
            format!("{:>4}", index + 1),
            value.to_string(),
            None,
        );
    }
    if lines.len() > MAX_LINES_SHOWN {
        builder.truncate();
    }

    builder.finish()
}
