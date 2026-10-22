//! OSB 场景容器的结构视图（13 Sentinels，94/94 闭合）。
//!
//! 依据 `osb_parse.py`：16B 头 + 路径池 + 递归节点树；节点 = 16B 头 + body。

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;
use super::text;

const NODE_HEADER: usize = 0x10;
const BODY_PREFIX: usize = 0x52;
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
