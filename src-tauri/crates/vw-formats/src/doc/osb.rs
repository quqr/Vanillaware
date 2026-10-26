//! OSB 场景容器的结构视图（13 Sentinels，94/94 闭合）。
//!
//! 依据 `osb_parse.py`：16B 头 + 路径池 + 递归节点树；节点 = 16B 头 + body。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::arena::{Builder, DocArena, DocMark};
use super::{fmd, rdr, text};
use crate::viz::{identity, VizNodeDto, VizReferenceDto, VizSceneDto};

const NODE_HEADER: usize = 0x10;
/// Caption(0x40) + Label(0x10) + Group(u16)。
const BODY_PREFIX: usize = 0x52;
/// model / anime 末尾的公共参数（88B）起点。
const MODEL_COMMON: usize = BODY_PREFIX + 0x44;
const ANIME_COMMON: usize = BODY_PREFIX + 0x11;
const MAX_DEPTH: usize = 64;
const MAX_CHILDREN: u32 = 4096;

const NODE_NAMES: &[(u32, &str)] = &[
    (2, "tree_root"),
    (5, "stage"),
    (6, "lane"),
    (7, "layer"),
    (8, "floor"),
    (9, "model"),
    (10, "particle"),
    (11, "anime"),
    (12, "camera_event"),
    (14, "mirror"),
    (15, "trigger"),
    (18, "shade"),
    (19, "light"),
    (20, "fog"),
    (21, "marker"),
    (22, "draw_area"),
    (23, "route"),
    (24, "postfx"),
];

/// 资源路径表（Scene 查看器与引用图共用）。
pub fn resources(bytes: &[u8]) -> Vec<String> {
    if bytes.len() < 0x18 {
        return Vec::new();
    }
    let count = rdr::u32_at(bytes, 0x10).unwrap_or(0);
    let pool_end = rdr::u32_at(bytes, 0x14).unwrap_or(0);
    let mut out = Vec::new();
    for index in 0..count.min(4096) {
        let Some(start_off) = rdr::u32_at(bytes, 0x18 + index as usize * 4) else {
            break;
        };
        let start = 0x10usize + start_off as usize;
        let end = if index + 1 < count {
            rdr::u32_at(bytes, 0x18 + (index as usize + 1) * 4)
                .map(|next| 0x10usize + next as usize)
                .unwrap_or(bytes.len())
        } else {
            0x10usize + pool_end as usize
        };
        let end = end.min(bytes.len());
        let Some(raw) = bytes.get(start..end) else {
            break;
        };
        out.push(text::sjis_slot(raw));
    }
    out
}

struct SceneBudget {
    vertices: usize,
    indices: usize,
    truncated: bool,
}

/// 同一 FMD 在场景里被多个 model 引用时只保留一份网格。
struct FmdInstance {
    mesh_ids: Vec<u32>,
    names: Vec<String>,
}

/// OSB 场景组装：节点树 + 资源引用；model 节点尝试把同目录 FMD 的网格挂进来。
///
/// 首版只做平移/缩放（不做旋转），FMD 灰模，纹理与动画引用只作为可跳转节点。
pub fn scene(bytes: &[u8], osb_path: &Path) -> VizSceneDto {
    let resources = resources(bytes);
    let base = osb_path.parent().unwrap_or_else(|| Path::new("."));
    let resolved: Vec<(String, PathBuf, bool)> = resources
        .iter()
        .map(|rel| {
            let path = resolve_resource(base, rel);
            let status = path.is_file();
            (rel.clone(), path, status)
        })
        .collect();

    let mut nodes = vec![VizNodeDto {
        id: 0,
        parent: None,
        name: osb_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "OSB".into()),
        kind: "group".into(),
        transform: identity(),
        mesh: None,
        mesh_ref: None,
        reference: None,
    }];
    let mut references = Vec::new();
    let mut fmd_cache: HashMap<PathBuf, FmdInstance> = HashMap::new();
    let mut bounds = [
        f32::INFINITY,
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    ];
    let mut budget = SceneBudget {
        vertices: 0,
        indices: 0,
        truncated: false,
    };

    if bytes.len() >= 0x18 {
        let pool_end = rdr::u32_at(bytes, 0x14).unwrap_or(0) as usize;
        let start = 0x10 + pool_end;
        collect_scene(
            bytes,
            start,
            bytes.len(),
            0,
            0,
            &resolved,
            &mut fmd_cache,
            &mut nodes,
            &mut references,
            &mut bounds,
            &mut budget,
        );
    }

    VizSceneDto {
        bounds: if bounds[0].is_finite() { bounds } else { [0.0; 6] },
        nodes,
        references,
        truncated: budget.truncated,
    }
}

fn resolve_resource(base: &Path, rel: &str) -> PathBuf {
    let mut candidates = vec![base.join(rel)];
    if let Some(parent) = base.parent() {
        candidates.push(parent.join(rel));
        if let Some(grand) = parent.parent() {
            candidates.push(grand.join(rel));
        }
    }
    candidates
        .iter()
        .find(|path| path.is_file())
        .cloned()
        .unwrap_or_else(|| base.join(rel))
}

#[allow(clippy::too_many_arguments)]
fn collect_scene(
    bytes: &[u8],
    offset: usize,
    end: usize,
    depth: usize,
    parent: u32,
    resources: &[(String, PathBuf, bool)],
    fmd_cache: &mut HashMap<PathBuf, FmdInstance>,
    nodes: &mut Vec<VizNodeDto>,
    references: &mut Vec<VizReferenceDto>,
    bounds: &mut [f32; 6],
    budget: &mut SceneBudget,
) -> usize {
    if depth >= MAX_DEPTH || offset + NODE_HEADER > end {
        return end;
    }
    let type_id = rdr::u32_at(bytes, offset).unwrap_or(0);
    let body_size = rdr::u32_at(bytes, offset + 4).unwrap_or(0) as usize;
    let child_count = rdr::u32_at(bytes, offset + 8).unwrap_or(0);
    let body_start = offset + NODE_HEADER;
    if body_start + body_size > end {
        return end;
    }
    let body = &bytes[body_start..body_start + body_size];

    let caption = body.get(0..0x40).map(text::sjis_slot).unwrap_or_default();
    let label = body.get(0x40..0x50).map(text::sjis_slot).unwrap_or_default();
    let type_name = node_name(type_id);
    let title = if !caption.is_empty() {
        caption
    } else if !label.is_empty() {
        label
    } else {
        type_name.to_string()
    };

    let (transform, placement) = match type_id {
        9 => {
            let origin = [
                raw_f32(body, MODEL_COMMON + 0x12),
                raw_f32(body, MODEL_COMMON + 0x16),
                raw_f32(body, MODEL_COMMON + 0x1a),
            ];
            let scale = [
                raw_f32(body, MODEL_COMMON + 0x2e),
                raw_f32(body, MODEL_COMMON + 0x32),
                raw_f32(body, MODEL_COMMON + 0x36),
            ];
            (trs(origin, scale), Some((origin, scale)))
        }
        11 => {
            let origin = [
                raw_f32(body, ANIME_COMMON + 0x12),
                raw_f32(body, ANIME_COMMON + 0x16),
                raw_f32(body, ANIME_COMMON + 0x1a),
            ];
            let scale = [
                raw_f32(body, ANIME_COMMON + 0x2e),
                raw_f32(body, ANIME_COMMON + 0x32),
                raw_f32(body, ANIME_COMMON + 0x36),
            ];
            (trs(origin, scale), Some((origin, scale)))
        }
        _ => (identity(), None),
    };

    let node_id = nodes.len() as u32;
    nodes.push(VizNodeDto {
        id: node_id,
        parent: Some(parent),
        name: format!("{type_name} · {title}"),
        kind: "group".into(),
        transform,
        mesh: None,
        mesh_ref: None,
        reference: None,
    });

    if let Some((origin, _scale)) = placement {
        expand_point(bounds, origin);
    }

    if type_id == 9 {
        let fmd_id = rdr::u16_at(body, BODY_PREFIX).unwrap_or(0) as usize;
        let ftx_id = rdr::u16_at(body, BODY_PREFIX + 2).unwrap_or(0) as usize;
        add_reference(resources, fmd_id, "model", node_id, nodes, references);
        add_reference(resources, ftx_id, "texture", node_id, nodes, references);
        if let Some((_, path, true)) = resources.get(fmd_id) {
            attach_model_meshes(path, placement, node_id, fmd_cache, nodes, bounds, budget);
        }
    } else if type_id == 11 {
        let fsb_id = rdr::u16_at(body, BODY_PREFIX + 4).unwrap_or(0) as usize;
        add_reference(resources, fsb_id, "animation", node_id, nodes, references);
    }

    let mut cursor = body_start + body_size;
    for _ in 0..child_count.min(MAX_CHILDREN) {
        cursor = collect_scene(
            bytes,
            cursor,
            end,
            depth + 1,
            node_id,
            resources,
            fmd_cache,
            nodes,
            references,
            bounds,
            budget,
        );
    }
    cursor
}

/// 把一个 FMD 的网格挂到 model 节点下；同一 FMD 的后续实例只引用首个实例的网格。
#[allow(clippy::too_many_arguments)]
fn attach_model_meshes(
    path: &Path,
    placement: Option<([f32; 3], [f32; 3])>,
    parent: u32,
    fmd_cache: &mut HashMap<PathBuf, FmdInstance>,
    nodes: &mut Vec<VizNodeDto>,
    bounds: &mut [f32; 6],
    budget: &mut SceneBudget,
) {
    if let Some(cached) = fmd_cache.get(path) {
        for (name, source_id) in cached.names.iter().zip(cached.mesh_ids.iter()) {
            let mesh_bounds = nodes
                .get(*source_id as usize)
                .and_then(|node| node.mesh.as_ref())
                .map(|mesh| mesh.bounds);
            if let (Some((origin, scale)), Some(mesh_bounds)) = (placement, mesh_bounds) {
                expand_mesh(bounds, &mesh_bounds, origin, scale);
            }
            nodes.push(VizNodeDto {
                id: nodes.len() as u32,
                parent: Some(parent),
                name: name.clone(),
                kind: "mesh".into(),
                transform: identity(),
                mesh: None,
                mesh_ref: Some(*source_id),
                reference: None,
            });
        }
        return;
    }

    let Ok(data) = std::fs::read(path) else {
        return;
    };
    let Ok(scene) = fmd::mesh_scene(&data) else {
        return;
    };
    if scene.truncated {
        budget.truncated = true;
    }

    let mut instance = FmdInstance {
        mesh_ids: Vec::new(),
        names: Vec::new(),
    };
    for mesh_node in scene.nodes.into_iter().skip(1) {
        let Some(mesh) = mesh_node.mesh else {
            continue;
        };
        if budget.vertices + mesh.positions.len() / 3 > 600_000
            || budget.indices + mesh.indices.len() > 2_000_000
        {
            budget.truncated = true;
            break;
        }
        budget.vertices += mesh.positions.len() / 3;
        budget.indices += mesh.indices.len();
        if let Some((origin, scale)) = placement {
            expand_mesh(bounds, &mesh.bounds, origin, scale);
        }
        let id = nodes.len() as u32;
        instance.mesh_ids.push(id);
        instance.names.push(mesh_node.name.clone());
        nodes.push(VizNodeDto {
            id,
            parent: Some(parent),
            name: mesh_node.name,
            kind: "mesh".into(),
            transform: identity(),
            mesh: Some(mesh),
            mesh_ref: None,
            reference: None,
        });
    }

    fmd_cache.insert(path.to_path_buf(), instance);
}

fn add_reference(
    resources: &[(String, PathBuf, bool)],
    index: usize,
    role: &str,
    parent: u32,
    nodes: &mut Vec<VizNodeDto>,
    references: &mut Vec<VizReferenceDto>,
) {
    let Some((rel, path, status)) = resources.get(index) else {
        return;
    };
    let reference = VizReferenceDto {
        path: path.to_string_lossy().into_owned(),
        ext: rel.rsplit('.').next().unwrap_or_default().to_ascii_lowercase(),
        role: role.to_string(),
        status: if *status { "resolved".into() } else { "missing".into() },
    };
    references.push(reference.clone());
    nodes.push(VizNodeDto {
        id: nodes.len() as u32,
        parent: Some(parent),
        name: rel.rsplit('/').next().unwrap_or(rel).to_string(),
        kind: "reference".into(),
        transform: identity(),
        mesh: None,
        mesh_ref: None,
        reference: Some(reference),
    });
}

fn trs(origin: [f32; 3], scale: [f32; 3]) -> [f32; 16] {
    [
        scale[0], 0.0, 0.0, 0.0,
        0.0, scale[1], 0.0, 0.0,
        0.0, 0.0, scale[2], 0.0,
        origin[0], origin[1], origin[2], 1.0,
    ]
}

fn expand_point(bounds: &mut [f32; 6], point: [f32; 3]) {
    bounds[0] = bounds[0].min(point[0]);
    bounds[1] = bounds[1].min(point[1]);
    bounds[2] = bounds[2].min(point[2]);
    bounds[3] = bounds[3].max(point[0]);
    bounds[4] = bounds[4].max(point[1]);
    bounds[5] = bounds[5].max(point[2]);
}

fn expand_mesh(bounds: &mut [f32; 6], mesh: &[f32; 6], origin: [f32; 3], scale: [f32; 3]) {
    for corner in 0..8 {
        let x = if corner & 1 == 0 { mesh[0] } else { mesh[3] };
        let y = if corner & 2 == 0 { mesh[1] } else { mesh[4] };
        let z = if corner & 4 == 0 { mesh[2] } else { mesh[5] };
        expand_point(bounds, [
            origin[0] + x * scale[0],
            origin[1] + y * scale[1],
            origin[2] + z * scale[2],
        ]);
    }
}

fn raw_f32(bytes: &[u8], at: usize) -> f32 {
    rdr::f32_at(bytes, at).unwrap_or(0.0)
}

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("OSB");

    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < 0x18 {
        builder.warn("文件不足 0x18 字节");
        builder.mark(root, DocMark::Error);
        return builder.finish();
    }

    let magic_ok = &bytes[..4] == b"OSB\0";
    let version = rdr::u32_at(bytes, 4).unwrap_or(0);
    let count = rdr::u32_at(bytes, 0x10).unwrap_or(0);
    let pool_end = rdr::u32_at(bytes, 0x14).unwrap_or(0);
    let scene = 0x10u64 + pool_end as u64;

    let header = builder.group(root, Some("header"), "Header");
    builder.field(header, "magic", rdr::ascii(&bytes[..4]));
    builder.field(header, "version", version.to_string());
    builder.field(header, "resourceCount", count.to_string());
    builder.field(header, "poolEnd", format!("0x{pool_end:x}"));
    builder.field(header, "sceneOffset", format!("0x{scene:x}"));

    builder.check(
        "osb.magic",
        "魔数为 OSB\\0",
        magic_ok,
        Some(rdr::hex(&bytes[..4])),
    );
    builder.check(
        "osb.scene",
        "场景区起点在文件内",
        scene <= bytes.len() as u64,
        Some(format!("{scene:#x} vs {:#x}", bytes.len())),
    );

    let resources = builder.array(root, Some("resources"), format!("Resources ({count})"));
    let shown = count.min(4096);
    if count > shown {
        builder.truncate();
    }
    for index in 0..shown {
        let Some(start_off) = rdr::u32_at(bytes, 0x18 + index as usize * 4) else {
            builder.warn(format!("资源路径表 {index} 越界"));
            break;
        };
        let start = 0x10usize + start_off as usize;
        let end = if index + 1 < count {
            rdr::u32_at(bytes, 0x18 + (index as usize + 1) * 4)
                .map(|next| 0x10usize + next as usize)
                .unwrap_or(bytes.len())
        } else {
            0x10usize + pool_end as usize
        };
        let end = end.min(bytes.len());
        let Some(raw) = bytes.get(start..end) else {
            builder.warn(format!("资源路径 {index} 区间越界"));
            continue;
        };
        let key = format!("r{index}");
        let path = text::sjis_slot(raw);
        builder.text(
            resources,
            &key,
            format!("resource[{index}]"),
            path,
            Some(start as u64),
        );
    }

    let scene_group = builder.group(root, Some("scene"), "Scene");
    let walk_end = parse_node(&mut builder, bytes, scene as usize, bytes.len(), 0, scene_group);
    builder.check(
        "osb.closed",
        "场景树精确闭合到 EOF",
        walk_end == bytes.len(),
        Some(format!("{walk_end:#x} vs {:#x}", bytes.len())),
    );

    builder.finish()
}

fn parse_node(
    builder: &mut Builder,
    bytes: &[u8],
    offset: usize,
    end: usize,
    depth: usize,
    parent: u32,
) -> usize {
    if depth >= MAX_DEPTH {
        builder.warn("节点树深度超过上限");
        builder.truncate();
        return end;
    }
    if offset + NODE_HEADER > end {
        builder.warn(format!("节点头越界 @{offset:#x}"));
        return end;
    }

    let type_id = rdr::u32_at(bytes, offset).unwrap_or(0);
    let body_size = rdr::u32_at(bytes, offset + 4).unwrap_or(0) as usize;
    let child_count = rdr::u32_at(bytes, offset + 8).unwrap_or(0);
    let body_start = offset + NODE_HEADER;

    if body_start + body_size > end {
        builder.warn(format!("节点 body 越界 @{offset:#x}"));
        return end;
    }

    let body = &bytes[body_start..body_start + body_size];
    let caption = body
        .get(0..0x40)
        .map(text::sjis_slot)
        .unwrap_or_default();
    let label = body
        .get(0x40..0x50)
        .map(text::sjis_slot)
        .unwrap_or_default();
    let group_id = body.get(0x50..0x52).and_then(|raw| rdr::u16_at(raw, 0)).unwrap_or(0);
    let type_name = node_name(type_id);
    let title = if !caption.is_empty() {
        caption.clone()
    } else if !label.is_empty() {
        label.clone()
    } else {
        type_name.to_string()
    };

    let key = format!("@{offset:x}");
    let node = builder.group(parent, Some(&key), format!("{type_name} · {title}"));
    builder.field(node, "type", format!("{type_id} ({type_name})"));
    builder.field(node, "bodySize", body_size.to_string());
    builder.field(node, "group", group_id.to_string());
    if !caption.is_empty() {
        builder.field(node, "caption", caption);
    }
    if !label.is_empty() {
        builder.field(node, "label", label);
    }

    if body_size > BODY_PREFIX {
        decode_schema(builder, node, type_id, body, BODY_PREFIX);
    }

    let mut cursor = body_start + body_size;
    let children = child_count.min(MAX_CHILDREN);
    if child_count > children {
        builder.truncate();
        builder.warn(format!("节点 {title} 的子节点过多（{child_count}）"));
    }
    for _ in 0..children {
        cursor = parse_node(builder, bytes, cursor, end, depth + 1, node);
    }
    cursor
}

fn decode_schema(builder: &mut Builder, node: u32, type_id: u32, body: &[u8], base: usize) {
    let at = |offset: usize| base + offset;
    match type_id {
        9 => {
            builder.field(node, "fmdResId", u16s(body, at(0)));
            builder.field(node, "ftxResId", u16s(body, at(2)));
            builder.field(node, "objName", slot(body, at(4), 0x40));
            common88(builder, node, body, at(0x44));
        }
        11 => {
            builder.field(node, "anime", f32s(body, at(0), 1));
            builder.field(node, "fsbResId", u16s(body, at(4)));
            builder.field(node, "animeWait", u16s(body, at(6)));
            builder.field(node, "initTime", f32s(body, at(8), 1));
            builder.field(node, "camShakeScale", f32s(body, at(0x0c), 1));
            common88(builder, node, body, at(0x11));
        }
        19 => {
            builder.field(node, "pos", f32s(body, at(0), 3));
            builder.field(node, "range", f32s(body, at(14), 1));
            builder.field(node, "power", f32s(body, at(18), 1));
            builder.field(node, "color", hex(body, at(0x1c), 4));
            builder.field(node, "ambientColor", hex(body, at(0x20), 4));
            builder.field(node, "freqTime", f32s(body, at(0x24), 1));
            builder.field(node, "freqRange", f32s(body, at(0x28), 1));
        }
        20 => {
            builder.field(node, "rangeMin", f32s(body, at(0), 1));
            builder.field(node, "rangeMax", f32s(body, at(4), 1));
            builder.field(node, "color", hex(body, at(8), 4));
        }
        22 => {
            builder.field(node, "minZ", f32s(body, at(0), 1));
            builder.field(node, "maxZ", f32s(body, at(4), 1));
            builder.field(node, "drawTiming", slot(body, at(8), 0x10));
        }
        6 => {
            builder.field(node, "scrollX", f32s(body, at(0), 1));
            builder.field(node, "scrollY", f32s(body, at(4), 1));
            builder.field(node, "loopX", f32s(body, at(8), 1));
            builder.field(node, "flags", hex(body, at(0x0c), 12));
        }
        7 => {
            builder.field(node, "orgStateNo", u16s(body, at(0)));
            builder.field(node, "ceLayerLabel", slot(body, at(2), 0x10));
            builder.field(node, "memoryOff", byte(body, at(0x12)));
            builder.field(node, "syncId", u32s(body, at(0x13)));
            builder.field(node, "syncSortZ", byte(body, at(0x17)));
        }
        8 => {
            builder.field(node, "points", f32s(body, at(0), 12));
            builder.field(node, "material", u32s(body, at(0x70)));
            builder.field(node, "triangle", byte(body, at(0x74)));
            builder.field(node, "opposite", byte(body, at(0x75)));
        }
        5 => {
            builder.field(node, "camLimitMin", f32s(body, at(0), 3));
            builder.field(node, "camLimitMax", f32s(body, at(12), 3));
            builder.field(node, "baseZ", f32s(body, at(24), 1));
            builder.field(node, "viewDist", f32s(body, at(28), 1));
            builder.field(node, "viewFov", f32s(body, at(32), 1));
            builder.field(node, "viewPitch", f32s(body, at(36), 1));
            builder.field(node, "viewRot", f32s(body, at(40), 1));
            builder.field(node, "smokeOff", byte(body, at(44)));
            builder.field(node, "overlayCharaOff", byte(body, at(45)));
        }
        14 => {
            builder.field(node, "yuragi", f32s(body, at(0), 1));
            builder.field(node, "color", hex(body, at(4), 4));
            builder.field(node, "fogColor", hex(body, at(8), 4));
            builder.field(node, "mirrorScaleY", f32s(body, at(12), 1));
            builder.field(node, "effectType", u32s(body, at(16)));
        }
        24 => {
            builder.field(node, "pos", f32s(body, at(0), 3));
            builder.field(node, "bokasiScale", f32s(body, at(12), 1));
            builder.field(node, "bloom", f32s(body, at(16), 1));
            builder.field(node, "gensui", f32s(body, at(20), 1));
            builder.field(node, "color", hex(body, at(0x19), 4));
        }
        23 => {
            builder.field(node, "pos", f32s(body, at(0), 3));
            builder.field(node, "raw", hex(body, at(12), 24));
        }
        18 => {
            builder.field(node, "pos", f32s(body, at(1), 3));
            builder.field(node, "range", f32s(body, at(0x11), 1));
            builder.field(node, "rimWidth", f32s(body, at(0x15), 1));
            builder.field(node, "shadeColor", hex(body, at(0x2b), 4));
        }
        10 => {
            builder.field(node, "scale", f32s(body, at(0), 2));
            builder.field(node, "pos", f32s(body, at(8), 3));
            builder.field(node, "offset", f32s(body, at(0x20), 3));
            builder.field(node, "color", hex(body, at(0x2c), 4));
        }
        12 => {
            builder.field(node, "view0", f32s(body, at(0x60), 4));
            builder.field(node, "view1", f32s(body, at(0x70), 4));
            builder.field(node, "ceType", u32s(body, at(0x84)));
        }
        15 => {
            builder.field(node, "triggerLabel", slot(body, at(0), 0x10));
            builder.field(node, "trigger", u32s(body, at(0x10)));
            builder.field(node, "actionLabel", slot(body, at(0x5d), 0x10));
            builder.field(node, "action", u32s(body, at(0xad)));
        }
        _ => {
            if body.len() > base {
                builder.field(node, "raw", hex(body, base, body.len() - base));
            }
        }
    }
}

fn common88(builder: &mut Builder, node: u32, body: &[u8], start: usize) {
    builder.field(node, "color", hex(body, start, 4));
    builder.field(node, "fogColor", hex(body, start + 4, 4));
    builder.field(node, "origin", f32s(body, start + 0x12, 3));
    builder.field(node, "sortOs", f32s(body, start + 0x1e, 1));
    builder.field(node, "scale", f32s(body, start + 0x2e, 3));
    builder.field(node, "rotate", f32s(body, start + 0x3a, 3));
    builder.field(node, "depthScaleZ", f32s(body, start + 0x46, 1));
}

fn node_name(type_id: u32) -> &'static str {
    NODE_NAMES
        .iter()
        .find(|(id, _)| *id == type_id)
        .map(|(_, name)| *name)
        .unwrap_or("unknown")
}

fn f32s(body: &[u8], at: usize, count: usize) -> String {
    (0..count)
        .map(|index| {
            rdr::f32_at(body, at + index * 4)
                .map(|value| format!("{value:.4}"))
                .unwrap_or_else(|| "—".into())
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn u16s(body: &[u8], at: usize) -> String {
    rdr::u16_at(body, at)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "—".into())
}

fn u32s(body: &[u8], at: usize) -> String {
    rdr::u32_at(body, at)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "—".into())
}

fn byte(body: &[u8], at: usize) -> String {
    body.get(at).map(u8::to_string).unwrap_or_else(|| "—".into())
}

fn hex(body: &[u8], at: usize, len: usize) -> String {
    body.get(at..at + len)
        .map(rdr::hex)
        .unwrap_or_else(|| "—".into())
}

fn slot(body: &[u8], at: usize, len: usize) -> String {
    body.get(at..at + len)
        .map(text::sjis_slot)
        .unwrap_or_else(|| "—".into())
}
