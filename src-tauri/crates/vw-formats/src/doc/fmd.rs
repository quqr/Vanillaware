//! FMD 模型容器：HEAD/JINT/MTRL/TXTR/MESH/ANIM/POSE 块序列。
//!
//! 块级严格按 `block_start + size` 推进；MESH 只解析网格头、子段/叶计数与顶点流锚点，
//! 不复制顶点数组。依据 `fmd_format.py`。

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;
use super::text;

const HEAD_VERSION: u32 = 0x19;
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

    fn f32s(&mut self, count: usize) -> Option<()> {
        self.pos = self.pos.checked_add(count * 4)?;
        (self.pos <= self.end).then_some(())
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

    fn align16(&mut self) -> Option<()> {
        self.pos = if self.pos.is_multiple_of(16) {
            self.pos
        } else {
            (self.pos - self.pos % 16) + 16
        };
        (self.pos <= self.end).then_some(())
    }
}

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("FMD");

    builder.field(root, "size", bytes.len().to_string());

    let chunks = builder.array(root, Some("chunks"), "Chunks");
    let mut pos = 0usize;
    let mut version_ok = true;

    while pos + 8 <= bytes.len() {
        let tag = &bytes[pos..pos + 4];
        let size = rdr::u32_at(bytes, pos + 4).unwrap_or(0) as usize;
        if size < 8 || pos + size > bytes.len() {
            builder.warn(format!("块 {} size={size} 非法", rdr::ascii(tag)));
            break;
        }

        let body_start = pos + 8;
        let end = pos + size;
        let tag_name = rdr::ascii(tag);
        let group = builder.group(chunks, Some(&format!("@{pos:x}")), format!("{tag_name} @{pos:#x}"));
        builder.field(group, "tag", tag_name.clone());
        builder.field(group, "size", size.to_string());
        builder.bytes(group, Some("range"), "块区间", pos as u64, size as u64);

        match tag {
            b"HEAD" => {
                let version = rdr::u32_at(bytes, body_start).unwrap_or(0);
                builder.field(group, "version", format!("0x{version:02x}"));
                version_ok = version == HEAD_VERSION;
                builder.check(
                    "fmd.version",
                    "HEAD version == 0x19",
                    version_ok,
                    Some(format!("0x{version:02x}")),
                );
            }
            b"JINT" => parse_jint(&mut builder, group, bytes, body_start, end),
            b"MTRL" => parse_mtrl(&mut builder, group, bytes, body_start, end),
            b"TXTR" => parse_txtr(&mut builder, group, bytes, body_start, end),
            b"MESH" => parse_mesh(&mut builder, group, bytes, body_start, end),
            b"ANIM" | b"POSE" => {
                let count = rdr::u32_at(bytes, body_start).unwrap_or(0);
                builder.field(group, "count", count.to_string());
            }
            _ => {
                builder.mark(group, DocMark::Note);
            }
        }

        pos = end;
    }

    builder.check(
        "fmd.closed",
        "块序列精确闭合到 EOF",
        pos == bytes.len(),
        Some(format!("{pos:#x} vs {:#x}", bytes.len())),
    );
    builder.check("fmd.versionGate", "版本闸门通过", version_ok, None);

    builder.finish()
}

fn parse_jint(builder: &mut Builder, parent: u32, bytes: &[u8], start: usize, end: usize) {
    let mut cur = Cursor::new(bytes, start, end);
    let Some(count) = cur.u32() else { return };
    let Some(root_index) = cur.i32() else { return };

    builder.field(parent, "nodeCount", count.to_string());
    builder.field(parent, "rootNode", root_index.to_string());

    let joints = builder.array(parent, Some("joints"), format!("Joints ({count})"));
    for index in 0..count.min(MAX_ITEMS) {
        let Some(n_a) = cur.u32() else { break };
        if n_a > 0x4000 || cur.skip(n_a as usize * 4).is_none() {
            builder.warn(format!("JINT 节点 {index} 首字段非法"));
            break;
        }
        let Some(name_len) = cur.u32() else { break };
        let Some(name_raw) = cur.take(name_len as usize) else {
            builder.warn(format!("JINT 节点 {index} 名字越界"));
            break;
        };
        let Some(parent_index) = cur.i32() else { break };
        let Some(_unk) = cur.i32() else { break };
        let Some(child_count) = cur.u32() else { break };
        if child_count > 0x4000 || cur.skip(child_count as usize * 4).is_none() {
            builder.warn(format!("JINT 节点 {index} 子表非法"));
            break;
        }
        if cur.skip(4 * 16 * 4).is_none() {
            builder.warn(format!("JINT 节点 {index} 矩阵越界"));
            break;
        }

        let key = format!("[{index}]");
        let name = text::utf8_or_sjis(name_raw);
        let node = builder.group(joints, Some(&key), if name.is_empty() { format!("joint[{index}]") } else { name });
        builder.field(node, "parent", parent_index.to_string());
        builder.field(node, "children", child_count.to_string());
        builder.field(node, "nameLength", name_len.to_string());
    }
    if count > MAX_ITEMS {
        builder.truncate();
    }
}

fn parse_mtrl(builder: &mut Builder, parent: u32, bytes: &[u8], start: usize, end: usize) {
    let mut cur = Cursor::new(bytes, start, end);
    let Some(count) = cur.u32() else { return };
    builder.field(parent, "count", count.to_string());

    let materials = builder.array(parent, Some("materials"), format!("Materials ({count})"));
    for index in 0..count.min(MAX_ITEMS) {
        let Some(name_len) = cur.u32() else { break };
        let Some(name_raw) = cur.take(name_len as usize) else { break };
        let Some(tex_ref) = cur.i32() else { break };
        let Some(rgba) = cur.take(4) else { break };
        let Some(_a_flag) = cur.u32() else { break };
        let Some(int_count) = cur.u32() else { break };
        if int_count > 0x400 || cur.skip(int_count as usize * 4).is_none() {
            builder.warn(format!("MTRL[{index}] 内联表越界"));
            break;
        }
        let Some(skip_count) = cur.i32() else { break };
        if skip_count > 0 && cur.skip(skip_count as usize * 4).is_none() {
            builder.warn(format!("MTRL[{index}] 跳过区越界"));
            break;
        }

        let key = format!("[{index}]");
        let name = text::utf8_or_sjis(name_raw);
        let node = builder.group(materials, Some(&key), if name.is_empty() { format!("material[{index}]") } else { name });
        builder.field(node, "texRef", tex_ref.to_string());
        builder.field(node, "rgba", rdr::hex(rgba));
    }
    if count > MAX_ITEMS {
        builder.truncate();
    }
}

fn parse_txtr(builder: &mut Builder, parent: u32, bytes: &[u8], start: usize, end: usize) {
    let mut cur = Cursor::new(bytes, start, end);
    let Some(count) = cur.u32() else { return };
    builder.field(parent, "count", count.to_string());

    let textures = builder.array(parent, Some("textures"), format!("Textures ({count})"));
    for index in 0..count.min(MAX_ITEMS) {
        let Some(name_len) = cur.u32() else { break };
        let Some(raw) = cur.take(name_len as usize) else { break };
        let key = format!("[{index}]");
        builder.text(textures, &key, format!("texture[{index}]"), text::utf8_or_sjis(raw), None);
    }
    if count > MAX_ITEMS {
        builder.truncate();
    }
}

fn parse_mesh(builder: &mut Builder, parent: u32, bytes: &[u8], start: usize, end: usize) {
    let mut cur = Cursor::new(bytes, start, end);
    let Some(mesh_count) = cur.u32() else { return };
    builder.field(parent, "meshCount", mesh_count.to_string());

    let meshes = builder.array(parent, Some("meshes"), format!("Meshes ({mesh_count})"));
    for index in 0..mesh_count.min(MAX_ITEMS) {
        if !probe_header(bytes, cur.pos, end) {
            let Some(next) = resync(bytes, cur.pos, end) else {
                builder.warn(format!("mesh[{index}] 无法重同步到下一个网格头"));
                break;
            };
            cur.pos = next;
        }

        let Some(name_len) = cur.u32() else { break };
        if name_len == 0 || name_len > 0x400 {
            builder.warn(format!("mesh[{index}] nameLen={name_len} 非法"));
            break;
        }
        let Some(name_raw) = cur.take(name_len as usize) else { break };
        let Some(a) = cur.i32() else { break };
        let Some(parent_joint) = cur.i32() else { break };
        let Some(skinned) = cur.i32() else { break };
        let Some(sub_count) = cur.i32() else { break };
        let Some(vtx_total) = cur.i32() else { break };
        if cur.f32s(11).is_none() {
            builder.warn(format!("mesh[{index}] bounds 越界"));
            break;
        }
        if !(1..=0x400).contains(&sub_count) {
            builder.warn(format!("mesh[{index}] subCount={sub_count} 非法"));
            break;
        }

        let Some((after_subs, tri_total)) = skip_subs(bytes, cur.pos, end, sub_count) else {
            match resync(bytes, cur.pos, end) {
                Some(next) => {
                    builder.warn(format!("mesh[{index}] 子段/叶越界，重同步到 {next:#x}"));
                    cur.pos = next;
                    continue;
                }
                None => {
                    builder.warn(format!("mesh[{index}] 子段/叶越界且无法重同步"));
                    break;
                }
            }
        };
        cur.pos = after_subs;

        let mut vertex_ok = true;
        let mut vtx_count = 0u32;
        if vtx_total != 0 || tri_total != 0 {
            if cur.align16().is_none() {
                vertex_ok = false;
            } else {
                match cur.u32() {
                    Some(value) => vtx_count = value,
                    None => vertex_ok = false,
                }
                if vertex_ok && vtx_count != vtx_total as u32 {
                    vertex_ok = false;
                }
                if vertex_ok {
                    let bytes_len = vtx_count as usize * 12;
                    vertex_ok = cur.skip(bytes_len).is_some();
                }
                for (stride, rule) in [
                    (12usize, AttrRule::Fixed),
                    (12, AttrRule::Fixed),
                    (12, AttrRule::Fixed),
                    (4, AttrRule::Fixed),
                    (8, AttrRule::Fixed),
                    (2, AttrRule::U16),
                    (4, AttrRule::U32I),
                ]
                .iter()
                .skip(1)
                {
                    if !vertex_ok || cur.align16().is_none() {
                        vertex_ok = false;
                        break;
                    }
                    let Some(n) = cur.u32() else {
                        vertex_ok = false;
                        break;
                    };
                    let payload = match rule {
                        AttrRule::Fixed => n as usize * stride,
                        AttrRule::U16 => {
                            if n == 0 { 0 } else { ((n as usize * 2 - 2) & !7) + 8 }
                        }
                        AttrRule::U32I => {
                            if n == 0 { 0 } else { ((n as usize - 1) & !3) + 4 }
                        }
                    };
                    if cur.skip(payload).is_none() {
                        vertex_ok = false;
                        break;
                    }
                }
                // Python: p = align16(p + 4 + pay) 对最后一条流也成立。
                if vertex_ok && cur.align16().is_none() {
                    vertex_ok = false;
                }
            }
        }
        if !vertex_ok {
            builder.warn(format!("mesh[{index}] 顶点流定位失败"));
            break;
        }

        let key = format!("[{index}]");
        let name = text::utf8_or_sjis(name_raw);
        let node = builder.group(meshes, Some(&key), if name.is_empty() { format!("mesh[{index}]") } else { name });
        builder.field(node, "A", a.to_string());
        builder.field(node, "parentJoint", parent_joint.to_string());
        builder.field(node, "skinned", skinned.to_string());
        builder.field(node, "subCount", sub_count.to_string());
        builder.field(node, "vtxTotal", vtx_total.to_string());
        builder.field(node, "vtxCount", vtx_count.to_string());
        builder.field(node, "triCount", tri_total.to_string());
        builder.field(node, "nonEmpty", (vtx_total != 0).to_string());
    }
    if mesh_count > MAX_ITEMS {
        builder.truncate();
    }
}

fn align16(pos: usize) -> usize {
    if pos.is_multiple_of(16) {
        pos
    } else {
        (pos - pos % 16) + 16
    }
}

/// 顺序跳过 `sub_count` 条子段及其叶，返回落点与累计三角数。
fn skip_subs(bytes: &[u8], mut pos: usize, end: usize, sub_count: i32) -> Option<(usize, u64)> {
    if sub_count < 0 {
        return None;
    }
    let mut tri_total = 0u64;
    for _ in 0..sub_count {
        if pos + 52 > end {
            return None;
        }
        let leaf_count = rdr::u32_at(bytes, pos + 48)?;
        pos += 52;
        if leaf_count > 0x4000 {
            return None;
        }
        for _ in 0..leaf_count {
            if pos + 44 + 8 > end {
                return None;
            }
            pos += 44;
            let n = rdr::u32_at(bytes, pos)?;
            let _skip = rdr::u32_at(bytes, pos + 4)?;
            if n > 0x1000 {
                return None;
            }
            let leaf_size = 4 * n as usize + 24;
            if pos + leaf_size > end {
                return None;
            }
            tri_total += rdr::u32_at(bytes, pos + leaf_size - 12)? as u64 & 0xffff;
            pos += leaf_size;
        }
    }
    Some((pos, tri_total))
}

/// 网格头预检：名字合法 + 五个 i32 合理 + 子段/叶可顺序读出。
fn probe_header(bytes: &[u8], q: usize, end: usize) -> bool {
    if q + 4 > end {
        return false;
    }
    let Some(name_len) = rdr::u32_at(bytes, q) else {
        return false;
    };
    if !(1..=0x40).contains(&name_len) {
        return false;
    }
    let Some(raw) = bytes.get(q + 4..q + 4 + name_len as usize) else {
        return false;
    };
    let name = &raw[..raw.iter().position(|&byte| byte == 0).unwrap_or(raw.len())];
    if name.is_empty() || name.iter().any(|&byte| byte < 0x20) {
        return false;
    }

    let p = q + 4 + name_len as usize;
    if p + 20 + 44 + 52 > end {
        return false;
    }
    let (Some(parent), Some(sub_count), Some(vtx_total)) =
        (rdr::i32_at(bytes, p + 4), rdr::i32_at(bytes, p + 12), rdr::i32_at(bytes, p + 16))
    else {
        return false;
    };
    if !(1..=0x400).contains(&sub_count)
        || !(0..50_000_000).contains(&vtx_total)
        || !(-1..0x100000).contains(&parent)
    {
        return false;
    }
    skip_subs(bytes, p + 64, end, sub_count).is_some()
}

fn resync(bytes: &[u8], from: usize, end: usize) -> Option<usize> {
    let limit = end.min(from.saturating_add(0x200000));
    let mut q = align16(from + 1);
    while q < limit {
        if probe_header(bytes, q, end) {
            return Some(q);
        }
        q = align16(q + 1);
    }
    None
}

#[derive(Clone, Copy)]
enum AttrRule {
    Fixed,
    U16,
    U32I,
}
