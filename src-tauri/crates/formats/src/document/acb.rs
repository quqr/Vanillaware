//! ```text
//! ```

use super::arena::{Builder, DocArena, DocMark};
use super::awb;
use super::utf;

pub fn build(bytes: &[u8], title: &str) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root(title);
    builder.field(root, "size", bytes.len().to_string());

    let tables = utf::append(&mut builder, root, bytes);

    let bank_at = find(bytes, b"AFS2");
    match bank_at {
        Some(at) => {
            let group = builder.group(root, Some("audioBank"), "Audio Bank (AFS2)");
            builder.field(group, "offset", format!("0x{at:x}"));
            match awb::parse(bytes, at) {
                Some(bank) => {
                    builder.field(group, "entries", bank.entries.len().to_string());
                    builder.field(group, "bytes", bytes.len().saturating_sub(at).to_string());
                    builder.check(
                        "acb.bank",
                        "AFS2 starts after the table area",
                        bank_at.unwrap_or(0) > 0,
                        Some(format!("@{at:#x}")),
                    );
                }
                None => {
                    builder.warn(format!("AFS2 @0x{at:x} header does not close"));
                    builder.mark(group, DocMark::Warn);
                }
            }
        }
        None => {
            builder.field(root, "audioBank", "- (pure cue sheet)");
        }
    }

    builder.check(
        "acb.tables",
        "at least one @UTF table",
        tables > 0,
        Some(format!("{tables} tables")),
    );

    builder.finish()
}

fn find(bytes: &[u8], needle: &[u8]) -> Option<usize> {
    if bytes.len() < needle.len() {
        return None;
    }
    bytes
        .windows(needle.len())
        .position(|window| window == needle)
}
