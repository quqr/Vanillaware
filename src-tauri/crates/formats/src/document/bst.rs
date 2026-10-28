//! ```text
//! +0x00 char[4] "BBST"
//! +0x08 u32     0
//! +0x0c u32     0
//! ```
//! |---|---|---|

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;

const MAGIC: &[u8] = b"BBST";
pub const STREAM_AT: usize = 0x16;
pub const RECORD_END: u8 = 0x34;

const MAX_RECORDS_SHOWN: usize = 48;
const MAX_PLATES_SHOWN: usize = 64;
const MAX_QUADS_SHOWN: usize = 48;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i32),
    Float(f32),
    Bool(bool),
    Rgba([u8; 4]),
    Raw4(u32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Mat4(Vec<f32>),
    Text(String),
}

#[derive(Debug, Clone)]
pub enum Node {
    Value(Value),
    End,
    Array {
        count: u32,
        grouped: bool,
        items: Vec<Node>,
    },
    Record(Vec<Node>),
}

#[derive(Debug, Clone)]
pub struct Plate {
    pub at: usize,
    pub id: i32,
    pub position: [f32; 3],
    pub corners: Vec<i32>,
}

#[derive(Debug, Clone)]
pub struct Quad {
    pub at: usize,
    pub corners: [[f32; 3]; 4],
    pub name: String,
}

pub struct Bst {
    pub names: Vec<String>,
    pub records: Vec<Node>,
    pub plates: Vec<Plate>,
    pub quads: Vec<Quad>,
    pub end: usize,
    pub unknown_tag: Option<(usize, u8)>,
}

struct Parser<'a> {
    bytes: &'a [u8],
}

impl<'a> Parser<'a> {
    fn skip_values(&self, at: usize, count: u32) -> Option<usize> {
        let mut cursor = at;
        for _ in 0..count {
            cursor = self.value_end(cursor)?;
        }
        Some(cursor)
    }

    fn skip_groups(&self, at: usize, count: u32) -> Option<usize> {
        let mut cursor = at;
        for _ in 0..count {
            let mut guard = 0usize;
            while *self.bytes.get(cursor)? != RECORD_END {
                cursor = self.value_end(cursor)?;
                guard += 1;
                if guard > 8192 {
                    return None;
                }
            }
            cursor += 1;
        }
        Some(cursor)
    }

    fn value_end(&self, at: usize) -> Option<usize> {
        let tag = *self.bytes.get(at)?;
        let fixed = match tag {
            0x05 | 0x06 | 0x09 | 0x14 | 0x15 => 4usize,
            0x0b => 1,
            0x08 | 0x1f => 8,
            0x20 => 12,
            0x23 => 64,
            0x28 => {
                let len = rdr::u32_at(self.bytes, at + 1)? as usize;
                let end = at.checked_add(6 + len)?;
                return (end <= self.bytes.len()).then_some(end);
            }
            0x33 => {
                let count = rdr::u32_at(self.bytes, at + 1)?;
                return self.skip_values(at + 5, count);
            }
            RECORD_END => return Some(at + 1),
            _ => return None,
        };
        let end = at + 1 + fixed;
        (end <= self.bytes.len()).then_some(end)
    }

    fn array_span(&self, at: usize, allow_group: bool) -> Option<(usize, bool)> {
        let count = rdr::u32_at(self.bytes, at + 1)?;
        let scalar = self.skip_values(at + 5, count)?;
        if self.lands(scalar) || !allow_group {
            return Some((scalar, false));
        }
        if let Some(grouped) = self.skip_groups(at + 5, count) {
            if self.lands(grouped) {
                return Some((grouped, true));
            }
        }
        Some((scalar, false))
    }

    fn lands(&self, at: usize) -> bool {
        at == self.bytes.len() || self.bytes.get(at) == Some(&RECORD_END)
    }

    fn node(&self, at: usize, allow_group: bool) -> Option<(Node, usize)> {
        let tag = *self.bytes.get(at)?;
        if tag == RECORD_END {
            return Some((Node::End, at + 1));
        }
        if tag == 0x33 {
            let count = rdr::u32_at(self.bytes, at + 1)?;
            let (end, grouped) = self.array_span(at, allow_group)?;
            let mut items = Vec::new();
            let mut cursor = at + 5;
            for _ in 0..count {
                let (node, next) = if grouped {
                    self.record(cursor)?
                } else {
                    self.node(cursor, false)?
                };
                items.push(node);
                cursor = next;
                if cursor > end {
                    return None;
                }
            }
            return Some((
                Node::Array {
                    count,
                    grouped,
                    items,
                },
                end,
            ));
        }
        let (value, end) = self.value(at)?;
        Some((Node::Value(value), end))
    }

    fn record(&self, at: usize) -> Option<(Node, usize)> {
        let mut items = Vec::new();
        let mut cursor = at;
        loop {
            if *self.bytes.get(cursor)? == RECORD_END {
                cursor += 1;
                break;
            }
            let allow_group = items.len() == 1;
            let (node, next) = self.node(cursor, allow_group)?;
            if next <= cursor {
                return None;
            }
            items.push(node);
            cursor = next;
        }
        Some((Node::Record(items), cursor))
    }

    fn value(&self, at: usize) -> Option<(Value, usize)> {
        let tag = *self.bytes.get(at)?;
        let end = self.value_end(at)?;
        let value = match tag {
            0x05 | 0x06 => Value::Int(rdr::i32_at(self.bytes, at + 1)?),
            0x09 => Value::Float(rdr::f32_at(self.bytes, at + 1)?),
            0x0b => Value::Bool(*self.bytes.get(at + 1)? != 0),
            0x14 => Value::Rgba([
                *self.bytes.get(at + 1)?,
                *self.bytes.get(at + 2)?,
                *self.bytes.get(at + 3)?,
                *self.bytes.get(at + 4)?,
            ]),
            0x15 => Value::Raw4(rdr::u32_at(self.bytes, at + 1)?),
            0x08 => Value::Vec2([
                rdr::f32_at(self.bytes, at + 1)?,
                rdr::f32_at(self.bytes, at + 5)?,
            ]),
            0x1f => Value::Vec2([
                rdr::f32_at(self.bytes, at + 1)?,
                rdr::f32_at(self.bytes, at + 5)?,
            ]),
            0x20 => Value::Vec3([
                rdr::f32_at(self.bytes, at + 1)?,
                rdr::f32_at(self.bytes, at + 5)?,
                rdr::f32_at(self.bytes, at + 9)?,
            ]),
            0x23 => {
                let mut matrix = Vec::with_capacity(16);
                for index in 0..16 {
                    matrix.push(rdr::f32_at(self.bytes, at + 1 + index * 4)?);
                }
                Value::Mat4(matrix)
            }
            0x28 => {
                let len = rdr::u32_at(self.bytes, at + 1)? as usize;
                let raw = self.bytes.get(at + 5..at + 5 + len)?;
                Value::Text(String::from_utf8_lossy(raw).to_string())
            }
            _ => return None,
        };
        Some((value, end))
    }
}

pub fn parse(bytes: &[u8]) -> Bst {
    let mut names = Vec::new();
    let mut records = Vec::new();
    let mut plates = Vec::new();
    let mut quads = Vec::new();
    let mut end = 0usize;
    let mut unknown_tag = None;

    if bytes.len() <= STREAM_AT {
        return Bst {
            names,
            records,
            plates,
            quads,
            end: bytes.len(),
            unknown_tag,
        };
    }

    let parser = Parser { bytes };
    let mut cursor = STREAM_AT;
    while cursor < bytes.len() {
        let Some((node, next)) = parser.record(cursor) else {
            unknown_tag = Some((cursor, bytes[cursor]));
            break;
        };
        if let Node::Record(items) = &node {
            for item in items {
                if let Node::Value(Value::Text(text)) = item {
                    if names.len() < 8
                        && (text.ends_with(".fmd")
                            || text.ends_with(".ftx")
                            || text.ends_with(".mbs"))
                    {
                        names.push(text.clone());
                    }
                }
            }
            classify(items, cursor, &mut plates, &mut quads);
            for item in items {
                if let Node::Array {
                    grouped: true,
                    items: inner,
                    ..
                } = item
                {
                    for child in inner {
                        if let Node::Record(child_items) = child {
                            classify(child_items, cursor, &mut plates, &mut quads);
                        }
                    }
                }
            }
        }
        records.push(node);
        cursor = next;
        end = cursor;
    }

    Bst {
        names,
        records,
        plates,
        quads,
        end,
        unknown_tag,
    }
}

fn classify(items: &[Node], at: usize, plates: &mut Vec<Plate>, quads: &mut Vec<Quad>) {
    if items.len() == 10 {
        if let (
            Node::Value(Value::Int(id)),
            Node::Value(Value::Vec3(position)),
            Node::Array { items: corners, .. },
        ) = (&items[6], &items[7], &items[9])
        {
            if matches!(items[3], Node::Value(Value::Text(_)))
                && matches!(items[2], Node::Value(Value::Int(_)))
            {
                plates.push(Plate {
                    at,
                    id: *id,
                    position: *position,
                    corners: corners
                        .iter()
                        .filter_map(|node| match node {
                            Node::Value(Value::Int(value)) => Some(*value),
                            _ => None,
                        })
                        .collect(),
                });
            }
        }
    }

    if items.len() >= 20 {
        let corners: Vec<[f32; 3]> = items
            .iter()
            .take(4)
            .filter_map(|node| match node {
                Node::Value(Value::Vec3(value)) => Some(*value),
                _ => None,
            })
            .collect();
        if corners.len() == 4 {
            let name = items
                .iter()
                .find_map(|node| match node {
                    Node::Value(Value::Text(text)) if !text.is_empty() => Some(text.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            let mut fixed = [[0.0f32; 3]; 4];
            for (slot, corner) in fixed.iter_mut().zip(corners) {
                *slot = corner;
            }
            quads.push(Quad {
                at,
                corners: fixed,
                name,
            });
        }
    }
}

fn describe(value: &Value) -> String {
    match value {
        Value::Int(v) => v.to_string(),
        Value::Float(v) => format!("{v:.4}"),
        Value::Bool(v) => if *v { "true" } else { "false" }.to_string(),
        Value::Rgba(c) => format!("#{:02x}{:02x}{:02x}{:02x}", c[0], c[1], c[2], c[3]),
        Value::Raw4(v) => format!("0x{v:08x}"),
        Value::Vec2(v) => format!("({:.3}, {:.3})", v[0], v[1]),
        Value::Vec3(v) => format!("({:.3}, {:.3}, {:.3})", v[0], v[1], v[2]),
        Value::Mat4(v) => format!(
            "[{:.2} {:.2} {:.2} {:.2} / {:.2} {:.2} {:.2} {:.2} / …]",
            v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]
        ),
        Value::Text(v) => v.clone(),
    }
}

fn push_node(builder: &mut Builder, parent: u32, node: &Node, key: &str, depth: usize) {
    match node {
        Node::End => {
            builder.field(parent, key, "END");
        }
        Node::Value(value) => {
            builder.field(parent, key, describe(value));
        }
        Node::Array {
            count,
            grouped,
            items,
        } => {
            let title = if *grouped {
                format!("Array ({count} records)")
            } else {
                format!("Array ({count} values)")
            };
            let group = builder.group(parent, Some(key), title);
            if depth > 4 {
                builder.field(
                    group,
                    "note",
                    format!("{} items (nesting too deep, not expanded)", items.len()),
                );
                return;
            }
            for (index, item) in items.iter().take(MAX_RECORDS_SHOWN).enumerate() {
                push_node(builder, group, item, &index.to_string(), depth + 1);
            }
            if items.len() > MAX_RECORDS_SHOWN {
                builder.truncate();
            }
        }
        Node::Record(items) => {
            let group = builder.group(parent, Some(key), format!("Record ({} values)", items.len()));
            for (index, item) in items.iter().enumerate().take(16) {
                push_node(builder, group, item, &index.to_string(), depth + 1);
            }
            if items.len() > 16 {
                builder.field(group, "…", format!("{} more values", items.len() - 16));
            }
        }
    }
}

pub fn build(bytes: &[u8]) -> DocArena {
    let bst = parse(bytes);
    let mut builder = Builder::new();
    let root = builder.root("BST");
    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < STREAM_AT {
        builder.warn("file is shorter than 0x16 bytes");
        builder.mark(root, DocMark::Error);
        return builder.finish();
    }

    let header = builder.group(root, Some("header"), "Header");
    builder.field(header, "magic", rdr::ascii(&bytes[..4]));
    builder.field(
        header,
        "count",
        rdr::u32_at(bytes, 0x10).unwrap_or(0).to_string(),
    );
    builder.field(
        header,
        "nameOffset",
        format!("0x{:02x}", bytes.get(0x14).copied().unwrap_or(0)),
    );
    builder.field(
        header,
        "variant",
        format!("0x{:02x}", bytes.get(0x15).copied().unwrap_or(0)),
    );

    builder.check(
        "bst.magic",
        "magic is BBST",
        &bytes[..4] == MAGIC,
        Some(rdr::hex(&bytes[..4])),
    );

    let names = bst.names.clone();
    builder.check(
        "bst.names",
        "the name table yields .fmd / .ftx",
        names.len() >= 2,
        Some(format!("{} items", names.len())),
    );
    let list = builder.array(
        root,
        Some("resources"),
        format!("Resources ({})", names.len()),
    );
    for (index, name) in names.iter().enumerate() {
        builder.field(list, &format!("r{index}"), name.clone());
    }

    if let Some(Node::Record(items)) = bst.records.first() {
        let stage = builder.group(
            root,
            Some("stage"),
            format!("Stage ({} values)", items.len()),
        );
        let corners = items
            .iter()
            .filter_map(|node| match node {
                Node::Value(Value::Vec3(value)) => Some(*value),
                _ => None,
            })
            .collect::<Vec<_>>();
        if corners.len() >= 4 {
            let quad = builder.array(
                stage,
                Some("bounds"),
                format!("Bounds ({})", corners.len().min(4)),
            );
            for (index, corner) in corners.iter().take(4).enumerate() {
                builder.field(
                    quad,
                    &format!("c{index}"),
                    format!("({:.2}, {:.2}, {:.2})", corner[0], corner[1], corner[2]),
                );
            }
        }
        let colours = items
            .iter()
            .filter_map(|node| match node {
                Node::Value(Value::Rgba(value)) => Some(*value),
                _ => None,
            })
            .count();
        builder.field(stage, "colours", colours.to_string());
        let waves = items
            .iter()
            .filter_map(|node| match node {
                Node::Array { items, .. } => Some(items),
                _ => None,
            })
            .flatten()
            .filter_map(|node| match node {
                Node::Value(Value::Text(text)) if !text.is_empty() => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        if !waves.is_empty() {
            builder.field(stage, "waveGens", waves.join(", "));
        }
    }

    let stats = builder.group(root, Some("stats"), "Stats");
    builder.field(stats, "records", bst.records.len().to_string());
    builder.field(stats, "plates", bst.plates.len().to_string());
    builder.field(stats, "quads", bst.quads.len().to_string());

    builder.check(
        "bst.closed",
        "the value stream stops exactly at EOF",
        bst.unknown_tag.is_none() && bst.end == bytes.len(),
        Some(match bst.unknown_tag {
            Some((at, tag)) => format!("unknown tag 0x{tag:02x} at 0x{at:x}"),
            None => format!("{:#x} / {:#x}", bst.end, bytes.len()),
        }),
    );

    let plates = builder.array(
        root,
        Some("plates"),
        format!("Plates ({})", bst.plates.len()),
    );
    for (index, plate) in bst.plates.iter().take(MAX_PLATES_SHOWN).enumerate() {
        let row = builder.group(plates, Some(&format!("p{index}")), format!("#{}", plate.id));
        builder.field(
            row,
            "position",
            format!(
                "({:.2}, {:.2}, {:.2})",
                plate.position[0], plate.position[1], plate.position[2]
            ),
        );
        builder.field(
            row,
            "corners",
            plate
                .corners
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
                .join(" "),
        );
        builder.field(row, "at", format!("0x{:x}", plate.at));
    }
    if bst.plates.len() > MAX_PLATES_SHOWN {
        builder.truncate();
    }

    let quads = builder.array(root, Some("quads"), format!("Quads ({})", bst.quads.len()));
    for (index, quad) in bst.quads.iter().take(MAX_QUADS_SHOWN).enumerate() {
        let row = builder.group(
            quads,
            Some(&format!("q{index}")),
            if quad.name.is_empty() {
                format!("q{index}")
            } else {
                quad.name.clone()
            },
        );
        for (slot, corner) in quad.corners.iter().enumerate() {
            builder.field(
                row,
                &format!("c{slot}"),
                format!("({:.2}, {:.2}, {:.2})", corner[0], corner[1], corner[2]),
            );
        }
        builder.field(row, "at", format!("0x{:x}", quad.at));
    }
    if bst.quads.len() > MAX_QUADS_SHOWN {
        builder.truncate();
    }

    let stream = builder.group(
        root,
        Some("stream"),
        format!("Stream ({} records)", bst.records.len()),
    );
    for (index, node) in bst.records.iter().take(MAX_RECORDS_SHOWN).enumerate() {
        push_node(&mut builder, stream, node, &index.to_string(), 0);
    }
    if bst.records.len() > MAX_RECORDS_SHOWN {
        builder.truncate();
    }

    builder.finish()
}
