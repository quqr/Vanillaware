use super::arena::{Builder, DocArena, DocMark};
use super::rdr;

pub fn build(bytes: &[u8], ext: &str) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root(format!("Binary .{ext}"));

    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < 4 {
        builder.warn("file is shorter than 4 bytes, no magic to read");
        builder.mark(root, DocMark::Error);
        return builder.finish();
    }

    builder.field(
        root,
        "magic",
        format!("{} · {}", rdr::ascii(&bytes[..4]), rdr::hex(&bytes[..4])),
    );

    let table = builder.array(root, Some("u32"), "u32[0..]");
    let count = (bytes.len() / 4).min(16);
    for index in 0..count {
        let Some(value) = rdr::u32_at(bytes, index * 4) else {
            builder.warn(format!("u32 table out of range at 0x{:x}", index * 4));
            break;
        };
        let key = format!("[{index}]");
        builder.field(table, &key, format!("0x{value:08X} ({value})"));
    }

    let strings = builder.array(root, Some("strings"), "strings[0..]");
    let mut start: Option<usize> = None;
    let mut found = 0usize;
    for (index, &byte) in bytes.iter().take(4096).enumerate() {
        let printable = (0x20..=0x7e).contains(&byte);
        if printable {
            start.get_or_insert(index);
            continue;
        }
        if let Some(begin) = start.take() {
            if index - begin >= 4 && found < 16 {
                let key = format!("s{found}");
                let name = format!("string[{found}]");
                let text = rdr::ascii(&bytes[begin..index]);
                builder.text(strings, &key, name, text, Some(begin as u64));
                found += 1;
            }
        }
    }
    if let Some(begin) = start {
        if bytes.len().saturating_sub(begin) >= 4 && found < 16 {
            let key = format!("s{found}");
            let name = format!("string[{found}]");
            let text = rdr::ascii(&bytes[begin..]);
            builder.text(strings, &key, name, text, Some(begin as u64));
        }
    }

    builder.finish()
}
