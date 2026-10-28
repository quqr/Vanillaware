//! 13 Sentinels 的 **bundle 容器**：`.osb` / `.oeb` / `.orb` 共用同一套布局。
//!
//! ```text
//! +0x00  char[4] magic        "OSB\0" / "OEB\0" / "ORB\0"
//! +0x04  u32     version      恒为 4
//! +0x08  u32     未定（恒 0）
//! +0x0c  u32     未定（恒 0）
//! +0x10  u32     resourceCount
//! +0x14  u32     poolSize     —— 资源池总长
//! +0x18  u32     offset[N]    —— 相对 0x10 的偏移，首项恒等于表尾
//! 0x10+poolSize  ...           节点树（顺序排列，childCount 指定子树条数）
//! ```
//!
//! 节点 = `16 字节头 + body`：
//!
//! ```text
//! +0x00 u32 type          节点类型（每个 bundle 有自己的编号表）
//! +0x04 u32 bodySize      恒为 0x52 起（0x52 = Caption(0x40) + Label(0x10) + Group(u16)）
//! +0x08 u32 childCount
//! +0x0c u32 填充
//! +0x10 body:
//!        +0x00 char[0x40] caption   Shift-JIS
//!        +0x40 char[0x10] label     Shift-JIS
//!        +0x50 u16        group
//! ```
//!
//! **闭合判据**：`0x10 + poolSize + Σ(0x10 + bodySize)` 恰好等于文件长度。
//! 实测 OSB 94/94、OEB 107/107、ORB 15/15 全部精确闭合。

use super::arena::{Builder, DocArena, DocMark};
use super::{rdr, text};

const HEADER: usize = 0x10;
const NODE_HEADER: usize = 0x10;
const MAX_DEPTH: usize = 64;
const MAX_NODES: usize = 60_000;
const MAX_CHILDREN: u32 = 4096;
const MAX_RESOURCES: u32 = 8192;

/// 一种 bundle 的元信息。
pub struct Spec {
    pub magic: &'static [u8],
    pub title: &'static str,
    /// 节点类型编号 → 名字。
    pub types: &'static [(u32, &'static str)],
}

/// 树里的一个节点（文档视图与内容投影共用）。
pub struct Node {
    pub at: usize,
    pub type_id: u32,
    pub body_at: usize,
    pub body_size: usize,
    pub child_count: u32,
    /// 在 `nodes` 里的父下标；根为 `None`。
    pub parent: Option<usize>,
    pub caption: String,
    pub label: String,
    pub group: u16,
}

/// 解析出来的容器。
pub struct Bundle {
    pub version: u32,
    pub resources: Vec<String>,
    /// 节点树起点。
    pub tree_at: usize,
    /// 顺序遍历出来的节点（父子关系由 `parent` 表达）。
    pub nodes: Vec<Node>,
    /// 遍历结束位置；等于文件长度即闭合。
    pub end: usize,
    pub children_of: Vec<Vec<usize>>,
}

/// 解析容器；魔数不符或结构越界时返回 `None`。
pub fn parse(bytes: &[u8], spec: &Spec) -> Option<Bundle> {
    if bytes.len() < 0x18 || &bytes[..4] != spec.magic {
        return None;
    }
    let version = rdr::u32_at(bytes, 4)?;
    let resource_count = rdr::u32_at(bytes, 0x10)?.min(MAX_RESOURCES);
    let pool_size = rdr::u32_at(bytes, 0x14)? as usize;
    let tree_at = HEADER.checked_add(pool_size)?;
    if tree_at > bytes.len() {
        return None;
    }

    let mut resources = Vec::with_capacity(resource_count as usize);
    for index in 0..resource_count {
        let start_off = rdr::u32_at(bytes, 0x18 + index as usize * 4)?;
        let start = HEADER + start_off as usize;
        let end = if index + 1 < resource_count {
            rdr::u32_at(bytes, 0x18 + (index as usize + 1) * 4)
                .map(|next| HEADER + next as usize)
                .unwrap_or(bytes.len())
        } else {
            HEADER + pool_size
        };
        let end = end.min(bytes.len());
        let raw = bytes.get(start..end)?;
        resources.push(text::sjis_slot(raw));
    }

    let mut nodes = Vec::new();
    let mut children_of: Vec<Vec<usize>> = Vec::new();
    let end = walk(
        bytes,
        tree_at,
        bytes.len(),
        0,
        None,
        &mut nodes,
        &mut children_of,
    );

    Some(Bundle {
        version,
        resources,
        tree_at,
        nodes,
        end,
        children_of,
    })
}

fn walk(
    bytes: &[u8],
    offset: usize,
    end: usize,
    depth: usize,
    parent: Option<usize>,
    nodes: &mut Vec<Node>,
    children_of: &mut Vec<Vec<usize>>,
) -> usize {
    if depth >= MAX_DEPTH || offset + NODE_HEADER > end || nodes.len() >= MAX_NODES {
        return offset;
    }
    let type_id = rdr::u32_at(bytes, offset).unwrap_or(0);
    let body_size = rdr::u32_at(bytes, offset + 4).unwrap_or(0) as usize;
    let child_count = rdr::u32_at(bytes, offset + 8).unwrap_or(0);
    let body_at = offset + NODE_HEADER;
    if body_size == 0 || body_at + body_size > end {
        return offset;
    }

    let body = &bytes[body_at..body_at + body_size];
    let caption = body.get(0..0x40).map(text::sjis_slot).unwrap_or_default();
    let label = body
        .get(0x40..0x50)
        .map(text::sjis_slot)
        .unwrap_or_default();
    let group = body
        .get(0x50..0x52)
        .and_then(|raw| rdr::u16_at(raw, 0))
        .unwrap_or(0);

    let index = nodes.len();
    nodes.push(Node {
        at: offset,
        type_id,
        body_at,
        body_size,
        child_count,
        parent,
        caption,
        label,
        group,
    });
    children_of.push(Vec::new());
    if let Some(parent) = parent {
        children_of[parent].push(index);
    }

    let mut cursor = body_at + body_size;
    for _ in 0..child_count.min(MAX_CHILDREN) {
        if cursor + NODE_HEADER > end {
            break;
        }
        let next = walk(
            bytes,
            cursor,
            end,
            depth + 1,
            Some(index),
            nodes,
            children_of,
        );
        if next <= cursor {
            break;
        }
        cursor = next;
    }
    cursor
}

fn type_name(spec: &Spec, type_id: u32) -> &'static str {
    spec.types
        .iter()
        .find(|(id, _)| *id == type_id)
        .map(|(_, name)| *name)
        .unwrap_or("node")
}

/// 把容器投影成文档树。
pub fn build(bytes: &[u8], spec: &Spec) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root(spec.title);
    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < 0x18 {
        builder.warn("文件不足 0x18 字节");
        builder.mark(root, DocMark::Error);
        return builder.finish();
    }

    let magic_ok = &bytes[..4] == spec.magic;
    let version = rdr::u32_at(bytes, 4).unwrap_or(0);
    let resource_count = rdr::u32_at(bytes, 0x10).unwrap_or(0);
    let pool_size = rdr::u32_at(bytes, 0x14).unwrap_or(0);
    let tree_at = 0x10u64 + pool_size as u64;

    let header = builder.group(root, Some("header"), "Header");
    builder.field(header, "magic", rdr::ascii(&bytes[..4]));
    builder.field(header, "version", version.to_string());
    builder.field(header, "resourceCount", resource_count.to_string());
    builder.field(header, "poolSize", format!("0x{pool_size:x}"));
    builder.field(header, "treeOffset", format!("0x{tree_at:x}"));

    builder.check(
        "bundle.magic",
        "魔数符合",
        magic_ok,
        Some(rdr::hex(&bytes[..4])),
    );
    builder.check(
        "bundle.tree",
        "节点树起点落在文件内",
        tree_at <= bytes.len() as u64,
        Some(format!("{tree_at:#x} vs {:#x}", bytes.len())),
    );

    let Some(bundle) = parse(bytes, spec) else {
        builder.warn("容器结构不成立，只做头部转储");
        builder.mark(root, DocMark::Error);
        builder.check(
            "bundle.closed",
            "资源池 + 节点树恰好到 EOF",
            false,
            Some("解析失败".to_string()),
        );
        return builder.finish();
    };

    let resources = builder.array(
        root,
        Some("resources"),
        format!("Resources ({})", bundle.resources.len()),
    );
    for (index, path) in bundle.resources.iter().enumerate() {
        builder.text(
            resources,
            &format!("r{index}"),
            format!("resource[{index}]"),
            path.clone(),
            None,
        );
    }

    let tree = builder.group(root, Some("tree"), format!("Tree ({})", bundle.nodes.len()));
    // `walk` 是前序遍历，父下标恒小于子下标，所以一遍扫下去就能拿到父句柄。
    let mut handles: Vec<u32> = Vec::with_capacity(bundle.nodes.len());
    for node in bundle.nodes.iter() {
        let parent = node
            .parent
            .and_then(|index| handles.get(index).copied())
            .unwrap_or(tree);
        let name = type_name(spec, node.type_id);
        let title = if !node.caption.is_empty() {
            node.caption.clone()
        } else if !node.label.is_empty() {
            node.label.clone()
        } else {
            name.to_string()
        };
        let key = format!("@{:x}", node.at);
        let handle = builder.group(parent, Some(&key), format!("{name} · {title}"));
        builder.field(handle, "type", format!("{} ({name})", node.type_id));
        builder.field(handle, "bodySize", format!("0x{:x}", node.body_size));
        builder.field(handle, "children", node.child_count.to_string());
        if node.group != 0 {
            builder.field(handle, "group", node.group.to_string());
        }
        if !node.caption.is_empty() {
            builder.field(handle, "caption", node.caption.clone());
        }
        if !node.label.is_empty() {
            builder.field(handle, "label", node.label.clone());
        }
        handles.push(handle);
    }

    builder.check(
        "bundle.closed",
        "资源池 + 节点树恰好到 EOF",
        bundle.end == bytes.len(),
        Some(format!("{:#x} / {:#x}", bundle.end, bytes.len())),
    );

    builder.finish()
}
