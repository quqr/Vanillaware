//! ```text
//! +0x00  char[4] magic        "OSB\0" / "OEB\0" / "ORB\0"
//! +0x10  u32     resourceCount
//! ```
//! ```text
//! +0x08 u32 childCount
//! +0x10 body:
//!        +0x00 char[0x40] caption   Shift-JIS
//!        +0x40 char[0x10] label     Shift-JIS
//!        +0x50 u16        group
//! ```

use super::arena::{Builder, DocArena, DocMark};
use super::{rdr, text};

const HEADER: usize = 0x10;
const NODE_HEADER: usize = 0x10;
const MAX_DEPTH: usize = 64;
const MAX_NODES: usize = 60_000;
const MAX_CHILDREN: u32 = 4096;
const MAX_RESOURCES: u32 = 8192;

pub struct Spec {
    pub magic: &'static [u8],
    pub title: &'static str,
    pub types: &'static [(u32, &'static str)],
}

pub struct Node {
    pub at: usize,
    pub type_id: u32,
    pub body_at: usize,
    pub body_size: usize,
    pub child_count: u32,
    pub parent: Option<usize>,
    pub caption: String,
    pub label: String,
    pub group: u16,
}

pub struct Bundle {
    pub version: u32,
    pub resources: Vec<String>,
    pub tree_at: usize,
    pub nodes: Vec<Node>,
    pub end: usize,
    pub children_of: Vec<Vec<usize>>,
}

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

pub fn build(bytes: &[u8], spec: &Spec) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root(spec.title);
    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < 0x18 {
        builder.warn("file is shorter than 0x18 bytes");
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
        "magic matches",
        magic_ok,
        Some(rdr::hex(&bytes[..4])),
    );
    builder.check(
        "bundle.tree",
        "the node tree starts inside the file",
        tree_at <= bytes.len() as u64,
        Some(format!("{tree_at:#x} vs {:#x}", bytes.len())),
    );

    let Some(bundle) = parse(bytes, spec) else {
        builder.warn("container structure does not hold, dumping the header only");
        builder.mark(root, DocMark::Error);
        builder.check(
            "bundle.closed",
            "resource pool plus node tree end exactly at EOF",
            false,
            Some("parse failed".to_string()),
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
        "resource pool plus node tree end exactly at EOF",
        bundle.end == bytes.len(),
        Some(format!("{:#x} / {:#x}", bundle.end, bytes.len())),
    );

    builder.finish()
}
