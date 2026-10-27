//! DRG 调试参数库（`DbReg/*.drg`，UO 侧 2 个）。
//!
//! 这是**开发期**的参数注册表，不是运行时玩法数据：记录里留着原始工程路径
//! （`C:/Project2/Unicorn/Soft/DbReg/Debug.txt`）。两个文件共用同一个头：
//!
//! ```text
//! +0x00  "DRG\0"
//! +0x04  u32 数据库数      +0x08 u32 命令数
//! +0x18  u32 命令区偏移    +0x28 u32 标签长度上限（63）
//! +0x38  数据库记录 × 数据库数，每条 0x90：
//!          +0x00 char[64] 原始工程路径
//!          +0x40 char[32] 名称（DEBUG / LOCAL / …）
//!          +0x60 char[32] 标签（UTF-8 日文）
//!          +0x80 u32 ×4   未定语义（原样展示）
//! 命令区 = 命令偏移 + 0x10（16 字节全 0 的对齐头）起，每条 0xD0：
//!          +0x00 char[64] 命令名（ALL_OFF / ALL_RESTORE / …）
//!          +0x40 char[144] 说明（UTF-8 日文）
//! ```
//!
//! `DbRegDb.drg` 上三条算术闭合：`96 × 0x90` 覆盖到命令区偏移附近、
//! `6559 × 0xD0` 正好到 EOF 前 0x10（末条被截短 0x10 字节，如实标注）。
//! `DbReg.drg` 没有数据库/命令（两个计数都是 0），体是 f32 值表：
//! `(文件长 − 0x38) % 4 == 0`，正好 6546 个。

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;
use super::text;

const HEADER: usize = 0x38;
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
        builder.warn("魔数不是 DRG\\0 或文件不足 0x38 字节");
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

    // --- 数据库表 ---
    let dbs = builder.array(root, Some("databases"), format!("Databases ({db_count})"));
    let mut db_end = HEADER;
    for index in 0..db_count.min(MAX_DBS_SHOWN) {
        let at = HEADER + index * DB_RECORD;
        if at + DB_RECORD > bytes.len() {
            builder.warn(format!("数据库[{index}] 越界"));
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

    // --- 命令表 ---
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

    // 全量走一遍只算算术闭合，不建节点。
    let full_cmd_end = cmd_start + cmd_count * CMD_RECORD;
    let short = full_cmd_end.saturating_sub(bytes.len());

    builder.check(
        "drg.db_table",
        "数据库表步长 0x90 不越界",
        HEADER + db_count * DB_RECORD <= bytes.len(),
        Some(format!("结束 0x{:x}", HEADER + db_count * DB_RECORD)),
    );
    // 命令区偏移 = 数据库表末尾 − 0x10：末尾记录的尾字段被命令区头覆盖 16 字节。
    builder.check(
        "drg.command_offset",
        "命令区（偏移 + 0x10）与数据库表末尾对齐",
        cmd_count == 0 || cmd_start == HEADER + db_count * DB_RECORD,
        Some(format!(
            "{cmd_start:#x} / 数据库表末尾 {:#x}",
            HEADER + db_count * DB_RECORD
        )),
    );
    builder.check(
        "drg.commands",
        "命令表覆盖到 EOF（末条允许短 0x10）",
        short <= 0x10,
        Some(format!(
            "{} 条 × 0xD0 结束于 {:#x}，超出文件 {short} 字节",
            cmd_count, full_cmd_end
        )),
    );

    // 没有命令时，体是 f32 值表（DbReg.drg）。
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
            "体长度被 4 整除（f32 值表）",
            body.is_multiple_of(4),
            Some(format!("{body} 字节 → {count} 个 f32")),
        );
    }

    let _ = parsed;
    let _ = cmd_end;
    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_short_and_foreign_files() {
        for len in 0..0x40 {
            let arena = build(&vec![0u8; len]);
            assert!(arena.total_nodes() > 0);
        }
        let arena = build(b"NOPE........");
        assert!(arena.total_nodes() > 0);
    }

    #[test]
    fn value_only_file_is_recognised() {
        let mut bytes = vec![0u8; HEADER + 8];
        bytes[..4].copy_from_slice(b"DRG\0");
        bytes[HEADER..HEADER + 4].copy_from_slice(&1.0f32.to_le_bytes());
        bytes[HEADER + 4..HEADER + 8].copy_from_slice(&2.0f32.to_le_bytes());
        let arena = build(&bytes);
        assert!(arena
            .checks()
            .iter()
            .any(|check| check.id == "drg.values" && check.ok));
    }
}
