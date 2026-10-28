//! ```text
//! +0x00  "DRG\0"
//! ```

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;
use super::text;

pub const HEADER: usize = 0x38;
const DB_RECORD: usize = 0x90;
const CMD_RECORD: usize = 0xD0;
const MAX_DBS_SHOWN: usize = 128;
const MAX_CMDS_SHOWN: usize = 128;
const MAX_VALUES_SHOWN: usize = 128;

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("DRG");
    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < HEADER || bytes.get(..4) != Some(b"DRG\0") {
        builder.warn("magic is not DRG\\0, or the file is shorter than 0x38 bytes");
        builder.mark(root, DocMark::Error);
        return builder.finish();
    }

    let db_count = rdr::u32_at(bytes, 0x04).unwrap_or(0) as usize;
    let cmd_count = rdr::u32_at(bytes, 0x08).unwrap_or(0) as usize;
    let cmd_offset = rdr::u32_at(bytes, 0x18).unwrap_or(0) as usize;
    let max_label = rdr::u32_at(bytes, 0x28).unwrap_or(0);

    let header = builder.group(root, Some("header"), "Header");
    builder.field(header, "magic", "DRG\\0");
    builder.field(header, "databases", db_count.to_string());
    builder.field(header, "commands", cmd_count.to_string());
    builder.field(header, "commandOffset", format!("0x{cmd_offset:x}"));
    builder.field(header, "maxLabel", max_label.to_string());

    let dbs = builder.array(root, Some("databases"), format!("Databases ({db_count})"));
    let mut db_end = HEADER;
    for index in 0..db_count.min(MAX_DBS_SHOWN) {
        let at = HEADER + index * DB_RECORD;
        if at + DB_RECORD > bytes.len() {
            builder.warn(format!("database[{index}] out of range"));
            break;
        }
        let path = text::utf8_or_sjis(&bytes[at..at + 64]);
        let name = text::utf8_or_sjis(&bytes[at + 0x40..at + 0x60]);
        let label = text::utf8_or_sjis(&bytes[at + 0x60..at + 0x80]);
        let node = builder.group(dbs, Some(&index.to_string()), format!("[{index}] {name}"));
        builder.text(node, "path", "path", path, Some(at as u64));
        builder.text(node, "name", "name", name, Some((at + 0x40) as u64));
        builder.text(node, "label", "label", label, Some((at + 0x60) as u64));
        for slot in 0..4 {
            let value = rdr::u32_at(bytes, at + 0x80 + slot * 4).unwrap_or(0);
            builder.field(node, &format!("f{slot}"), value.to_string());
        }
        db_end = at + DB_RECORD;
    }
    let _ = db_end;
    if db_count > MAX_DBS_SHOWN {
        builder.truncate();
    }

    let cmds = builder.array(root, Some("commands"), format!("Commands ({cmd_count})"));
    let cmd_start = cmd_offset.saturating_add(0x10);
    let mut cmd_end = cmd_start;
    let mut parsed = 0usize;
    for index in 0..cmd_count.min(MAX_CMDS_SHOWN) {
        let at = cmd_start + index * CMD_RECORD;
        if at + 0x40 > bytes.len() {
            break;
        }
        let name = text::utf8_or_sjis(&bytes[at..(at + 64).min(bytes.len())]);
        let label_end = (at + CMD_RECORD).min(bytes.len());
        let label = text::utf8_or_sjis(&bytes[(at + 0x40).min(label_end)..label_end]);
        let node = builder.group(cmds, Some(&index.to_string()), format!("[{index}] {name}"));
        builder.text(node, "name", "name", name, Some(at as u64));
        builder.text(node, "label", "label", label, Some((at + 0x40) as u64));
        parsed += 1;
        cmd_end = at + CMD_RECORD;
    }
    if cmd_count > MAX_CMDS_SHOWN {
        builder.truncate();
    }

    let full_cmd_end = cmd_start + cmd_count * CMD_RECORD;
    let short = full_cmd_end.saturating_sub(bytes.len());

    builder.check(
        "drg.db_table",
        "database table stride 0x90 stays inside the file",
        HEADER + db_count * DB_RECORD <= bytes.len(),
        Some(format!("ends at 0x{:x}", HEADER + db_count * DB_RECORD)),
    );
    builder.check(
        "drg.command_offset",
        "command area (offset + 0x10) aligns with the database table end",
        cmd_count == 0 || cmd_start == HEADER + db_count * DB_RECORD,
        Some(format!(
            "{cmd_start:#x} / database table ends at {:#x}",
            HEADER + db_count * DB_RECORD
        )),
    );
    builder.check(
        "drg.commands",
        "command table reaches EOF (the last entry may be 0x10 short)",
        short <= 0x10,
        Some(format!(
            "{} × 0xD0 ends at {:#x}, {short} bytes past the file",
            cmd_count, full_cmd_end
        )),
    );

    if cmd_count == 0 && db_count == 0 {
        let body = bytes.len().saturating_sub(HEADER);
        let count = body / 4;
        let values = builder.array(root, Some("values"), format!("f32 ({count})"));
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for index in 0..count.min(MAX_VALUES_SHOWN) {
            let value = rdr::f32_at(bytes, HEADER + index * 4).unwrap_or(0.0);
            if value.is_finite() {
                min = min.min(value);
                max = max.max(value);
            }
            builder.field(values, &index.to_string(), format!("{value}"));
        }
        let summary = builder.group(root, Some("summary"), "Summary");
        builder.field(summary, "values", count.to_string());
        if min.is_finite() {
            builder.field(summary, "f32Range", format!("{min} .. {max}"));
        }
        builder.check(
            "drg.values",
            "body length is divisible by 4 (f32 value table)",
            body.is_multiple_of(4),
            Some(format!("{body} bytes -> {count} f32")),
        );
    }

    let _ = parsed;
    let _ = cmd_end;
    builder.finish()
}
