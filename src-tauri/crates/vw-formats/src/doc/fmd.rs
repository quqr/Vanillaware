//! FMD 模型容器：HEAD/JINT/MTRL/TXTR/MESH/ANIM/POSE 块序列。
//!
//! 块级严格按 `block_start + size` 推进；MESH 只解析网格头、子段/叶计数与顶点流锚点，
//! 不复制顶点数组。依据 `fmd_format.py`。

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;
use super::text;
use crate::viz::{identity, VizMeshDto, VizNodeDto, VizSceneDto, VizSubRangeDto};

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

/// 可视化首版：提取所有非空网格的 positions + 按 sub 生成的顺序 indices。
///
/// 不做材质/UV/贴图；只验证几何、包围盒与三角拓扑。
pub fn mesh_scene(bytes: &[u8]) -> Result<VizSceneDto, String> {
    let mut nodes = vec![VizNodeDto {
        id: 0,
        parent: None,
        name: "FMD".into(),
        kind: "group".into(),
        transform: identity(),
        mesh: None,
        mesh_ref: None,
        reference: None,
    }];
    let mut scene_bounds = [f32::INFINITY, f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];
    let mut truncated = false;

    let mut pos = 0usize;
    while pos + 8 <= bytes.len() {
        let tag = &bytes[pos..pos + 4];
        let size = rdr::u32_at(bytes, pos + 4).unwrap_or(0) as usize;
        if size < 8 || pos + size > bytes.len() {
            break;
        }
        if tag == b"MESH" {
            collect_meshes(bytes, pos + 8, pos + size, &mut nodes, &mut scene_bounds, &mut truncated)?;
        }
        pos += size;
    }

    let bounds = if nodes.len() > 1 && scene_bounds[0].is_finite() {
        scene_bounds
    } else {
        [0.0; 6]
    };
    Ok(VizSceneDto {
        bounds,
        nodes,
        references: Vec::new(),
        truncated,
    })
}

fn collect_meshes(
    bytes: &[u8],
    start: usize,
    end: usize,
    nodes: &mut Vec<VizNodeDto>,
    scene_bounds: &mut [f32; 6],
    truncated: &mut bool,
) -> Result<(), String> {
    let mut cur = Cursor::new(bytes, start, end);
    let Some(mesh_count) = cur.u32() else {
        return Ok(());
    };

    for index in 0..mesh_count.min(MAX_ITEMS) {
        if !probe_header(bytes, cur.pos, end) {
            let Some(next) = resync(bytes, cur.pos, end) else {
                break;
            };
            cur.pos = next;
        }

        let Some(name_len) = cur.u32() else { break };
        if name_len == 0 || name_len > 0x400 {
            break;
        }
        let Some(name_raw) = cur.take(name_len as usize) else {
            break;
        };
        let (Some(_a), Some(_parent), Some(_skinned), Some(sub_count), Some(vtx_total)) =
            (cur.i32(), cur.i32(), cur.i32(), cur.i32(), cur.i32())
        else {
            break;
        };
        if cur.f32s(11).is_none() || !(1..=0x400).contains(&sub_count) {
            break;
        }

        let Some((after_subs, ranges)) = skip_subs(bytes, cur.pos, end, sub_count) else {
            let Some(next) = resync(bytes, cur.pos, end) else {
                break;
            };
            cur.pos = next;
            continue;
        };
        cur.pos = after_subs;
        let tri_total: u64 = ranges.iter().map(|range| range.tri as u64).sum();

        let mut positions = Vec::new();
        let mut vtx_count = 0u32;
        if vtx_total != 0 || tri_total != 0 {
            if cur.align16().is_none() {
                break;
            }
            let Some(count) = cur.u32() else { break };
            vtx_count = count;
            let byte_len = count as usize * 12;
            let Some(raw) = bytes.get(cur.pos..cur.pos + byte_len) else {
                break;
            };
            positions = raw
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();
            if cur.skip(byte_len).is_none() {
                break;
            }

            let mut stream_ok = true;
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
                if !stream_ok || cur.align16().is_none() {
                    stream_ok = false;
                    break;
                }
                let Some(n) = cur.u32() else {
                    stream_ok = false;
                    break;
                };
                let payload = match rule {
                    AttrRule::Fixed => n as usize * stride,
                    AttrRule::U16 => {
                        if n == 0 {
                            0
                        } else {
                            ((n as usize * 2 - 2) & !7) + 8
                        }
                    }
                    AttrRule::U32I => {
                        if n == 0 {
                            0
                        } else {
                            ((n as usize - 1) & !3) + 4
                        }
                    }
                };
                if cur.skip(payload).is_none() {
                    stream_ok = false;
                    break;
                }
            }
            if !stream_ok || cur.align16().is_none() {
                break;
            }
        }

        if positions.is_empty() {
            continue;
        }

        let mut indices = Vec::new();
        let mut sub_ranges = Vec::new();
        for range in ranges {
            let count = range.tri * 3;
            if count == 0 || range.start + count > vtx_count {
                continue;
            }
            sub_ranges.push(VizSubRangeDto {
                start: range.start,
                count,
                material: range.material,
            });
            for vertex in range.start..range.start + count {
                indices.push(vertex);
            }
        }
        if indices.is_empty() {
            continue;
        }

        let mut bounds = [f32::INFINITY, f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];
        for point in positions.chunks_exact(3) {
            bounds[0] = bounds[0].min(point[0]);
            bounds[1] = bounds[1].min(point[1]);
            bounds[2] = bounds[2].min(point[2]);
            bounds[3] = bounds[3].max(point[0]);
            bounds[4] = bounds[4].max(point[1]);
            bounds[5] = bounds[5].max(point[2]);
        }
        if bounds[0].is_finite() {
            scene_bounds[0] = scene_bounds[0].min(bounds[0]);
            scene_bounds[1] = scene_bounds[1].min(bounds[1]);
            scene_bounds[2] = scene_bounds[2].min(bounds[2]);
            scene_bounds[3] = scene_bounds[3].max(bounds[3]);
            scene_bounds[4] = scene_bounds[4].max(bounds[4]);
            scene_bounds[5] = scene_bounds[5].max(bounds[5]);
        }

        let name = text::utf8_or_sjis(name_raw);
        nodes.push(VizNodeDto {
            id: nodes.len() as u32,
            parent: Some(0),
            name: if name.is_empty() { format!("mesh[{index}]") } else { name },
            kind: "mesh".into(),
            transform: identity(),
            mesh: Some(VizMeshDto {
                positions,
                indices,
                bounds,
                sub_ranges,
            }),
            mesh_ref: None,
            reference: None,
        });

        // 首版 IPC 上限：超过后停止继续提取，前端明确显示「已截断」。
        const MAX_VERTICES: usize = 600_000;
        const MAX_INDICES: usize = 2_000_000;
        let vertices: usize = nodes.iter().filter_map(|node| node.mesh.as_ref()).map(|mesh| mesh.positions.len() / 3).sum();
        let indices: usize = nodes.iter().filter_map(|node| node.mesh.as_ref()).map(|mesh| mesh.indices.len()).sum();
        if vertices > MAX_VERTICES || indices > MAX_INDICES {
            *truncated = true;
            break;
        }
    }
    Ok(())
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

        let Some((after_subs, ranges)) = skip_subs(bytes, cur.pos, end, sub_count) else {
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
        let tri_total: u64 = ranges.iter().map(|range| range.tri as u64).sum();

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

/// 一个绘制段：顶点起始、三角数、材质下标。
#[derive(Clone, Copy, Debug)]
struct SubRange {
    start: u32,
    tri: u32,
    material: i32,
}

/// 顺序读取 `sub_count` 条子段及其叶，返回落点与每个绘制段。
fn skip_subs(bytes: &[u8], mut pos: usize, end: usize, sub_count: i32) -> Option<(usize, Vec<SubRange>)> {
    if sub_count < 0 {
        return None;
    }
    let mut ranges = Vec::new();
    for _ in 0..sub_count {
        if pos + 52 > end {
            return None;
        }
        let material = rdr::i32_at(bytes, pos)?;
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
            let tri = rdr::u32_at(bytes, pos + leaf_size - 12)? & 0xffff;
            let start = rdr::u32_at(bytes, pos + leaf_size - 8)?;
            if tri > 0 {
                ranges.push(SubRange { start, tri, material });
            }
            pos += leaf_size;
        }
    }
    Some((pos, ranges))
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
