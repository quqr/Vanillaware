use serde::Serialize;

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;
use super::text;

const HEADER: usize = 0x20;
const TRAILER: usize = 0x10;
const MAX_STRINGS: u32 = 4096;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FmsTextDto {
    pub encoding: u8,
    pub count: u32,
    pub strings: Vec<String>,
}

pub fn text_table(bytes: &[u8]) -> FmsTextDto {
    if bytes.len() < HEADER {
        return FmsTextDto {
            encoding: 0,
            count: 0,
            strings: Vec::new(),
        };
    }
    let count = rdr::u32_at(bytes, 0x14).unwrap_or(0);
    let encoding = bytes[0x18];
    let mut cursor = HEADER + count as usize * 8;
    let mut strings = Vec::new();

    for _ in 0..count {
        if cursor >= bytes.len() {
            break;
        }
        if encoding == 1 {
            let mut probe = cursor;
            while probe + 1 < bytes.len() {
                if bytes[probe] == 0 && bytes[probe + 1] == 0 {
                    break;
                }
                probe += 2;
            }
            let end = probe.min(bytes.len());
            strings.push(text::utf16le(&bytes[cursor..end]));
            cursor = (end + 2).min(bytes.len());
        } else {
            let end = bytes[cursor..]
                .iter()
                .position(|&byte| byte == 0)
                .map(|offset| cursor + offset)
                .unwrap_or(bytes.len());
            strings.push(text::utf8_or_sjis(&bytes[cursor..end]));
            cursor = (end + 1).min(bytes.len());
        }
    }

    FmsTextDto {
        encoding,
        count,
        strings,
    }
}

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("FMSB");
    parse(&mut builder, root, bytes, 0, "fms");
    builder.finish()
}

pub fn parse(builder: &mut Builder, parent: u32, bytes: &[u8], base: u64, prefix: &str) {
    builder.field(parent, "size", bytes.len().to_string());

    if bytes.len() < HEADER {
        builder.warn(format!("{prefix}: file is shorter than 0x20 bytes"));
        builder.mark(parent, DocMark::Error);
        return;
    }

    let magic = rdr::ascii(&bytes[..4]);
    let payload = rdr::u32_at(bytes, 4).unwrap_or(0);
    let header_size = rdr::u32_at(bytes, 8).unwrap_or(0);
    let count = rdr::u32_at(bytes, 0x14).unwrap_or(0);
    let encoding = bytes[0x18];

    builder.field(parent, "magic", magic.clone());
    builder.field(parent, "payloadSize", payload.to_string());
    builder.field(parent, "headerSize", format!("0x{header_size:x}"));
    builder.field(parent, "stringCount", count.to_string());
    builder.field(
        parent,
        "encoding",
        format!("{encoding} ({})", encoding_name(encoding)),
    );

    builder.check(
        &format!("{prefix}.magic"),
        "magic is FMSB",
        magic == "FMSB",
        Some(format!("read {magic}")),
    );
    builder.check(
        &format!("{prefix}.header"),
        "u32@0x08 == 0x20",
        header_size == HEADER as u32,
        Some(format!("{header_size:#x}")),
    );
    builder.check(
        &format!("{prefix}.reserved"),
        "u32 at 0x0C/0x10 is 0",
        rdr::u32_at(bytes, 0x0c) == Some(0) && rdr::u32_at(bytes, 0x10) == Some(0),
        None,
    );

    let table_end = HEADER + count as usize * 8;
    let table_ok = table_end <= bytes.len();
    builder.check(
        &format!("{prefix}.table"),
        "the entry table lies inside the file",
        table_ok,
        Some(format!("tableEnd={table_end:#x} size={:#x}", bytes.len())),
    );
    if !table_ok {
        builder.mark(parent, DocMark::Error);
        return;
    }

    let strings = builder.array(parent, Some("strings"), format!("Strings ({count})"));
    let shown = count.min(MAX_STRINGS);
    if count > shown {
        builder.truncate();
    }

    let mut cursor = table_end;
    for index in 0..count {
        if cursor >= bytes.len() {
            builder.warn(format!("{prefix}: string {index} out of range"));
            break;
        }

        let start = cursor;
        let value = if encoding == 1 {
            let mut probe = cursor;
            while probe + 1 < bytes.len() {
                if bytes[probe] == 0 && bytes[probe + 1] == 0 {
                    break;
                }
                probe += 2;
            }
            let end = probe.min(bytes.len());
            cursor = (end + 2).min(bytes.len());
            (index < shown).then(|| text::utf16le(&bytes[start..end]))
        } else {
            let end = bytes[start..]
                .iter()
                .position(|&byte| byte == 0)
                .map(|offset| start + offset)
                .unwrap_or(bytes.len());
            cursor = (end + 1).min(bytes.len());
            (index < shown).then(|| text::utf8_or_sjis(&bytes[start..end]))
        };

        if let Some(value) = value {
            let key = format!("s{index}");
            builder.text(
                strings,
                &key,
                format!("string[{index}]"),
                value,
                Some(base + start as u64),
            );
        }
    }

    let aligned = cursor.div_ceil(16) * 16;
    let trailer_ok =
        aligned + TRAILER <= bytes.len() && bytes.get(aligned..aligned + 4) == Some(b"FEOC");
    builder.check(
        &format!("{prefix}.trailer"),
        "the 16-byte FEOC trailer closes exactly",
        trailer_ok,
        Some(format!("aligned={aligned:#x} size={:#x}", bytes.len())),
    );
    if trailer_ok {
        builder.check(
            &format!("{prefix}.payload"),
            "payloadSize == aligned − 0x20",
            payload as usize == aligned - HEADER,
            Some(format!("{payload:#x} vs {:#x}", aligned - HEADER)),
        );
        let tail = builder.group(parent, Some("trailer"), "Trailer");
        builder.field(tail, "magic", rdr::ascii(&bytes[aligned..aligned + 4]));
        builder.field(
            tail,
            "u32@4",
            rdr::u32_at(bytes, aligned + 4).unwrap_or(0).to_string(),
        );
        builder.field(
            tail,
            "u32@8",
            format!("0x{:x}", rdr::u32_at(bytes, aligned + 8).unwrap_or(0)),
        );
        builder.field(
            tail,
            "u32@12",
            rdr::u32_at(bytes, aligned + 12).unwrap_or(0).to_string(),
        );
    } else if aligned < bytes.len() {
        builder.mark(parent, DocMark::Warn);
    }
}

fn encoding_name(encoding: u8) -> &'static str {
    match encoding {
        0 | 3 => "UTF-8",
        1 => "UTF-16LE",
        _ => "unknown",
    }
}
