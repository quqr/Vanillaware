use std::path::Path;

fn main() {
    for path in std::env::args().skip(1) {
        let target = Path::new(&path);
        let name = target
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.clone());
        let _unused = match std::fs::read(target) {
            Ok(bytes) => bytes,
            Err(error) => {
                println!("{name}: read failed {error}");
                continue;
            }
        };
        match media::content::open_path(target) {
            Some(content) => {
                let image = content
                    .image
                    .as_ref()
                    .map(|value| format!("{} B", value.len()))
                    .unwrap_or_else(|| "—".to_string());
                println!(
                    "{name}: {} / {} tables / bitmap {image} / lines {} / text {}",
                    content.title,
                    content.sections.len(),
                    content
                        .series
                        .as_ref()
                        .map(|series| series.points.len().to_string())
                        .unwrap_or_else(|| "—".to_string()),
                    content
                        .text
                        .as_ref()
                        .map(|lines| format!("{} lines", lines.len()))
                        .unwrap_or_else(|| "—".to_string())
                );
                if let Some(note) = content.image_note.as_ref() {
                    println!("   ! {note}");
                }
                if let Some(note) = content.note.as_ref() {
                    println!("   * {note}");
                }
                for section in content.sections.iter().take(3) {
                    let head: Vec<String> = section
                        .rows
                        .first()
                        .map(|row| row.iter().take(4).cloned().collect())
                        .unwrap_or_default();
                    println!(
                        "   - {} [{}] {} lines / {} total / first {:?}",
                        section.title,
                        section.columns.join("|"),
                        section.rows.len(),
                        section.total,
                        head
                    );
                }
                if let Some(dir) = std::env::var_os("DUMP_DIR") {
                    if let Some(data) = content.image.as_ref() {
                        if let Some((_, payload)) = data.split_once(",") {
                            let file = Path::new(&dir).join(format!("{name}.png"));
                            if let Some(bytes) = decode_base64(payload) {
                                std::fs::write(&file, bytes).ok();
                                println!("   - bitmap written to {}", file.display());
                            }
                        }
                    }
                }
                if let Some(lines) = content.text.as_ref() {
                    for line in lines.iter().take(2) {
                        println!("   > {line}");
                    }
                }
            }
            None => println!("{name}: no content view"),
        }
    }
}

fn decode_base64(text: &str) -> Option<Vec<u8>> {
    fn value(byte: u8) -> Option<u8> {
        match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut buffer = 0u32;
    let mut bits = 0u32;
    for byte in text.bytes() {
        if byte == b'=' {
            break;
        }
        let Some(part) = value(byte) else {
            continue;
        };
        buffer = (buffer << 6) | part as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Some(out)
}
