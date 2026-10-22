//! FSB 单位出场包：HEAD → RESL → CTLG → SCNL 四块。
//!
//! 块级严格闭合；SCNL 只解析条目/组/关键帧头，不展开矩阵与关键帧数组。

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;
use super::text;

const FSB_VERSION: u32 = 0x15;
const MAX_ITEMS: u32 = 4096;

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
    end: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8], pos: usize, end: usize) -> Self {
        Self { bytes, pos, end }
    }

    fn u32(&mut self) -> Option<u32> {
        let value = rdr::u32_at(self.bytes, self.pos)?;
        self.pos += 4;
        (self.pos <= self.end).then_some(value)
    }

    fn i32(&mut self) -> Option<i32> {
        self.u32().map(|value| value as i32)
    }

    fn f32(&mut self) -> Option<f32> {
        let value = rdr::f32_at(self.bytes, self.pos)?;
        self.pos += 4;
        (self.pos <= self.end).then_some(value)
    }

    fn skip(&mut self, count: usize) -> Option<()> {
        self.pos = self.pos.checked_add(count)?;
        (self.pos <= self.end).then_some(())
    }

    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(count)?;
        let slice = self.bytes.get(self.pos..end)?;
        (end <= self.end).then_some(slice)?;
        self.pos = end;
        Some(slice)
    }
}

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("FSB");

    builder.field(root, "size", bytes.len().to_string());

    let chunks = builder.array(root, Some("chunks"), "Chunks");
    let mut pos = 0usize;
    let mut magic_seq: Vec<String> = Vec::new();

    while pos + 8 <= bytes.len() {
        let magic = &bytes[pos..pos + 4];
        let size = rdr::u32_at(bytes, pos + 4).unwrap_or(0) as usize;
        if size < 8 || pos + size > bytes.len() {
            builder.warn(format!("块 {} size={size} 非法", rdr::ascii(magic)));
            break;
        }

        let name = rdr::ascii(magic);
        magic_seq.push(name.clone());
        let group = builder.group(chunks, Some(&format!("@{pos:x}")), format!("{name} @{pos:#x}"));
        builder.field(group, "size", size.to_string());
        builder.bytes(group, Some("range"), "块区间", pos as u64, size as u64);

        let body = pos + 8;
        let end = pos + size;
        match magic {
            b"HEAD" => {
                let version = rdr::u32_at(bytes, body).unwrap_or(0);
                let fps = rdr::f32_at(bytes, body + 4).unwrap_or(0.0);
                builder.field(group, "version", format!("0x{version:02x}"));
                builder.field(group, "fps", format!("{fps:.3}"));
                builder.check(
                    "fsb.version",
                    "HEAD version == 0x15",
                    version == FSB_VERSION,
                    Some(format!("0x{version:02x}")),
                );
            }
            b"RESL" => parse_resl(&mut builder, group, bytes, body, end),
            b"CTLG" => parse_ctlg(&mut builder, group, bytes, body, end),
            b"SCNL" => parse_scnl(&mut builder, group, bytes, body, end),
            _ => {
                builder.mark(group, DocMark::Warn);
            }
        }

        pos = end;
    }

    let sequence_ok = magic_seq == ["HEAD", "RESL", "CTLG", "SCNL"];
    builder.check(
        "fsb.sequence",
        "四块顺序 HEAD→RESL→CTLG→SCNL",
        sequence_ok,
        Some(magic_seq.join(" → ")),
    );
    builder.check(
        "fsb.closed",
        "块序列精确闭合到 EOF",
        pos == bytes.len(),
        Some(format!("{pos:#x} vs {:#x}", bytes.len())),
    );

    builder.finish()
}

fn parse_resl(builder: &mut Builder, parent: u32, bytes: &[u8], start: usize, end: usize) {
    let mut cur = Cursor::new(bytes, start, end);
    let Some(count) = cur.u32() else { return };
    builder.field(parent, "count", count.to_string());

    let names = builder.array(parent, Some("names"), format!("Names ({count})"));
    for index in 0..count.min(MAX_ITEMS) {
        let Some(len) = cur.u32() else { break };
        let copy = (len as usize).min(0x100);
        let Some(raw) = cur.take(copy) else { break };
        if cur.skip((len as usize).saturating_sub(copy)).is_none() {
            break;
        }
        let key = format!("[{index}]");
        builder.text(names, &key, format!("name[{index}]"), text::utf8_or_sjis(raw), None);
    }
    if count > MAX_ITEMS {
        builder.truncate();
    }
}

fn parse_ctlg(builder: &mut Builder, parent: u32, bytes: &[u8], start: usize, end: usize) {
    let mut cur = Cursor::new(bytes, start, end);
    let Some(count) = cur.u32() else { return };
    builder.field(parent, "count", count.to_string());

    let records = builder.array(parent, Some("records"), format!("Records ({count})"));
    for index in 0..count.min(MAX_ITEMS) {
        let Some(record_type) = cur.u32() else { break };
        let key = format!("[{index}]");
        let node = builder.group(records, Some(&key), format!("record[{index}]"));
        builder.field(node, "recordType", format!("0x{record_type:x}"));

        let ok = match record_type {
            4 => read_two(&mut cur, builder, node),
            6 | 0xc | 0x13 | 0x1f => read_three(&mut cur, builder, node),
            0x15 => read_vals(&mut cur, builder, node, 13),
            0x16 | 0x17 => read_aabb(&mut cur, builder, node),
            0x1c => read_two(&mut cur, builder, node),
            0x1d => read_vals(&mut cur, builder, node, 12),
            0x20 => read_flag(&mut cur, builder, node),
            _ => {
                builder.warn(format!("CTLG 未知记录类型 0x{record_type:x}"));
                false
            }
        };
        if !ok {
            builder.mark(node, DocMark::Warn);
            break;
        }
    }
    if count > MAX_ITEMS {
        builder.truncate();
    }
}

fn read_two(cur: &mut Cursor<'_>, builder: &mut Builder, node: u32) -> bool {
    let (Some(a), Some(b)) = (cur.u32(), cur.u32()) else {
        return false;
    };
    builder.field(node, "a", a.to_string());
    builder.field(node, "b", b.to_string());
    true
}

fn read_three(cur: &mut Cursor<'_>, builder: &mut Builder, node: u32) -> bool {
    let (Some(a), Some(b), Some(c)) = (cur.u32(), cur.u32(), cur.u32()) else {
        return false;
    };
    builder.field(node, "a", a.to_string());
    builder.field(node, "b", b.to_string());
    builder.field(node, "c", c.to_string());
    true
}

fn read_vals(cur: &mut Cursor<'_>, builder: &mut Builder, node: u32, count: usize) -> bool {
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        let Some(value) = cur.u32() else { return false };
        values.push(value.to_string());
    }
    builder.field(node, "values", values.join(", "));
    true
}

fn read_aabb(cur: &mut Cursor<'_>, builder: &mut Builder, node: u32) -> bool {
    let Some(value) = cur.u32() else { return false };
    builder.field(node, "u", value.to_string());
    let mut aabb = Vec::new();
    for _ in 0..6 {
        let Some(value) = cur.f32() else { return false };
        aabb.push(format!("{value:.4}"));
    }
    builder.field(node, "aabb", aabb.join(", "));
    true
}

fn read_flag(cur: &mut Cursor<'_>, builder: &mut Builder, node: u32) -> bool {
    let (Some(flag), Some(a), Some(b)) = (cur.i32(), cur.u32(), cur.u32()) else {
        return false;
    };
    builder.field(node, "flag", flag.to_string());
    builder.field(node, "a", a.to_string());
    builder.field(node, "b", b.to_string());
    true
}

fn parse_scnl(builder: &mut Builder, parent: u32, bytes: &[u8], start: usize, end: usize) {
    let mut cur = Cursor::new(bytes, start, end);
    let Some(count) = cur.u32() else { return };
    builder.field(parent, "count", count.to_string());

    let entries = builder.array(parent, Some("entries"), format!("Entries ({count})"));
    for index in 0..count.min(MAX_ITEMS) {
        let Some(len) = cur.u32() else { break };
        let copy = (len as usize).min(0x40);
        let Some(raw) = cur.take(copy) else { break };
        if cur.skip((len as usize).saturating_sub(copy)).is_none() {
            break;
        }
        let Some(gcount) = cur.i32() else { break };
        let (Some(u70), Some(u74)) = (cur.u32(), cur.u32()) else {
            break;
        };
        if cur.skip(6 * 4).is_none() {
            break;
        }

        let key = format!("[{index}]");
        let name = text::utf8_or_sjis(raw);
        let node = builder.group(entries, Some(&key), if name.is_empty() { format!("entry[{index}]") } else { name });
        builder.field(node, "gcount", gcount.to_string());
        builder.field(node, "u70", u70.to_string());
        builder.field(node, "u74", u74.to_string());

        let mut ok = true;
        for group_index in 0..gcount.max(0) as u32 {
            let (Some(u20), Some(nkf), Some(flag), Some(third)) =
                (cur.u32(), cur.i32(), cur.u32(), cur.u32())
            else {
                ok = false;
                break;
            };
            let (Some(u34), Some(u38)) = (cur.u32(), cur.u32()) else {
                ok = false;
                break;
            };
            if cur.skip(24).is_none() {
                ok = false;
                break;
            }

            let gkey = format!("[{group_index}]");
            let group = builder.group(node, Some(&gkey), format!("group[{group_index}]"));
            builder.field(group, "u20", u20.to_string());
            builder.field(group, "keyframes", nkf.to_string());
            builder.field(group, "flag", format!("0x{flag:x}"));
            builder.field(group, "third", format!("0x{third:x}"));
            builder.field(group, "u34", u34.to_string());
            builder.field(group, "u38", u38.to_string());

            for kf_index in 0..nkf.max(0) as u32 {
                let base = cur.pos;
                let (Some(target), Some(u08), Some(frames), Some(record_id)) = (
                    rdr::u32_at(bytes, base),
                    rdr::u32_at(bytes, base + 4),
                    rdr::u32_at(bytes, base + 8),
                    rdr::i32_at(bytes, base + 12),
                ) else {
                    ok = false;
                    break;
                };
                let n = rdr::i32_at(bytes, base + 389).unwrap_or(0).max(0) as usize;
                if cur.skip(393 + n * 4).is_none() {
                    ok = false;
                    break;
                }

                let kkey = format!("[{kf_index}]");
                let kf = builder.group(group, Some(&kkey), format!("keyframe[{kf_index}]"));
                builder.field(kf, "target", target.to_string());
                builder.field(kf, "u08", u08.to_string());
                builder.field(kf, "frames", frames.to_string());
                builder.field(kf, "recordId", record_id.to_string());
                builder.field(kf, "skipTable", n.to_string());
            }
            if !ok {
                break;
            }
        }
        if !ok {
            builder.mark(node, DocMark::Warn);
            break;
        }
    }
    if count > MAX_ITEMS {
        builder.truncate();
    }
}
