use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::content::{ColorMap, Projection, Section};
use super::document::rdr;
use super::mbs;
mod play;

pub use play::Playback;

use super::raster::{paint_quad, Page, Raster};

pub const HEAD: usize = 0x88;
pub const RECORD_HEAD: usize = 0x24;
pub const LAYER: usize = 0x40;
pub const EXT: usize = 0x90;
pub const MASK_HEAD: usize = 0x3c;
pub const GROUP_HEAD: usize = 4;
pub const GROUP_ENTRY: usize = 0xf0;
/// A group block holds three parallel arrays per entry: four vertices (0x60),
/// four copies of one placement record (0x70) and four unknown pairs (0x20).
pub const GROUP_QUAD: usize = 0x60;
pub const GROUP_PLACE: usize = 0x70;
pub const VERTEX: usize = 0x18;
pub const PANEL: usize = 0xa0;
pub const COLUMNS: usize = 138;
pub const ROWS: usize = 105;
pub const MASK_CELL: f32 = 32.0;
pub const GRID_COLUMNS: usize = 2199;
pub const GRID_ROWS: usize = 1311;
pub const WORLD_X0: f32 = -42480.0;
pub const WORLD_Z0: f32 = -18416.0;
pub const MASK_UNDERLAY: u16 = 0x4700;
pub const SAMPLE_NONE: u16 = 0x7ff;

pub const ANGLE_UNIT: f32 = std::f32::consts::PI / 32768.0;
pub const SCALE_UNIT: f32 = 1.0 / 4096.0;

const MAX_AREAS: usize = 24;
const PROP_ROWS: usize = 12;

#[derive(Clone, Copy, Default)]
pub struct Counts {
    pub records: u32,
    pub group_blocks: u32,
    pub objects: u32,
    pub model_objects: u32,
    pub panel_objects: u32,
    pub masks: u32,
    pub f_records: u32,
    pub words: u32,
    pub pairs: u32,
    pub samples: u32,
    pub groups: u32,
    pub names: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub u: f32,
    pub v: f32,
    pub color: u32,
}

#[derive(Clone, Copy, Default)]
pub struct Layer {
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub color: u32,
    pub fog: u32,
    pub center_x: f32,
    pub center_z: f32,
    pub center_y: f32,
    pub pitch: i16,
    pub roll: i16,
    pub scale_x: u16,
    pub scale_y: u16,
    pub flags: u16,
    pub ext: u8,
    pub band: u8,
    pub slot: u8,
    pub angle: u16,
    pub pair: i32,
    pub panel: u16,
    pub sample: u16,
    pub at: usize,
}

/// One ground tile: a quad whose four corners are `corners`, plus an explicit
/// centre vertex so the atlas rect can bend instead of mapping affinely. The
/// sixth stored vertex is a repeat of `corners[0]` that closes the fan.
#[derive(Clone, Copy, Default)]
pub struct Ext {
    pub center: Vertex,
    pub corners: [Vertex; 4],
}

/// Ground tiles cover the record's tile on a 128-unit grid.
pub const TERRAIN_CELL: f32 = 128.0;

pub struct GroupBlock {
    pub at: usize,
    pub id: u16,
    pub slot: u8,
    pub count: u8,
}

/// One ground piece of a group block: four corners in the record's local space
/// plus the world cell it is pinned to.
#[derive(Clone, Copy, Default)]
pub struct GroundQuad {
    pub at: (f32, f32),
    pub corners: [(f32, f32); 4],
    pub uv: [(f32, f32); 4],
    pub color: u32,
}

pub struct Record {
    pub index: usize,
    pub at: usize,
    pub size: usize,
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub column: u32,
    pub row: u32,
    pub layers: Vec<Layer>,
    pub ext_at: usize,
    pub ext_count: usize,
    pub groups: Vec<GroupBlock>,
    pub groups_at: usize,
}

#[derive(Clone, Copy)]
pub struct Mask {
    pub at: usize,
    pub world: [f32; 6],
    pub rect: [i32; 4],
    pub marked: u32,
    pub cells: u32,
    pub sample: u16,
    pub group: i16,
    pub flag: u16,
    pub id: u16,
    pub data: usize,
}

#[derive(Clone, Copy)]
pub struct FRecord {
    pub at: usize,
    pub rect: [f32; 4],
    pub tags: [u16; 4],
}

#[derive(Clone, Default)]
pub struct Pair {
    pub model: String,
    pub texture: String,
}

impl Pair {
    pub fn is_panel(&self) -> bool {
        self.model.to_ascii_uppercase().ends_with(".MPD")
    }
}

#[derive(Clone, Copy, Default)]
pub struct Derived {
    pub layers: u64,
    pub ext_blocks: u64,
    pub group_blocks: u64,
    pub group_entries: u64,
    pub panel_layers: u64,
    pub model_layers: u64,
}

pub struct Layout {
    pub counts: Counts,
    pub records: Vec<Record>,
    pub masks: Vec<Mask>,
    pub f_records: Vec<FRecord>,
    pub words: Vec<u32>,
    pub pairs: Vec<Pair>,
    pub samples: Vec<String>,
    pub group_names: Vec<String>,
    pub names: Vec<String>,
    pub derived: Derived,
    pub end: usize,
}

impl Layout {
    pub fn area_name(&self, mask: &Mask) -> Option<&str> {
        if mask.group < 0 {
            return None;
        }
        self.group_names
            .get(mask.group as usize)
            .map(String::as_str)
    }

    pub fn prop_name(&self, layer: &Layer) -> Option<&str> {
        if layer.sample == SAMPLE_NONE {
            return None;
        }
        self.samples.get(layer.sample as usize).map(String::as_str)
    }
}

pub fn record_size(bytes: &[u8], at: usize) -> Option<(usize, usize, usize, usize)> {
    let layers = rdr::u16_at(bytes, at + 0x20)? as usize;
    let ext = rdr::u8_at(bytes, at + 0x22)? as usize;
    let groups = rdr::u8_at(bytes, at + 0x23)? as usize;
    let mut cursor = at + RECORD_HEAD + layers * LAYER + ext * EXT;
    for _ in 0..groups {
        let count = rdr::u8_at(bytes, cursor + 3)? as usize;
        cursor += GROUP_HEAD + count * GROUP_ENTRY;
    }
    Some((cursor - at, layers, ext, groups))
}

pub fn parse(bytes: &[u8]) -> Option<Layout> {
    if bytes.len() < HEAD {
        return None;
    }
    let counts = Counts {
        records: rdr::u32_at(bytes, 0x40)?,
        group_blocks: rdr::u32_at(bytes, 0x44).unwrap_or(0),
        objects: rdr::u32_at(bytes, 0x48).unwrap_or(0),
        model_objects: rdr::u32_at(bytes, 0x4c).unwrap_or(0),
        panel_objects: rdr::u32_at(bytes, 0x50).unwrap_or(0),
        masks: rdr::u32_at(bytes, 0x54).unwrap_or(0),
        f_records: rdr::u32_at(bytes, 0x58).unwrap_or(0),
        words: rdr::u32_at(bytes, 0x5c).unwrap_or(0),
        pairs: rdr::u32_at(bytes, 0x60).unwrap_or(0),
        samples: rdr::u32_at(bytes, 0x64).unwrap_or(0),
        groups: rdr::u32_at(bytes, 0x68).unwrap_or(0),
        names: rdr::u32_at(bytes, 0x6c).unwrap_or(0),
    };

    let mut cursor = HEAD;
    let mut derived = Derived::default();
    let mut records = Vec::with_capacity(counts.records as usize);
    for index in 0..counts.records as usize {
        let layers_count = rdr::u16_at(bytes, cursor + 0x20)? as usize;
        let ext_count = rdr::u8_at(bytes, cursor + 0x22)? as usize;
        let group_count = rdr::u8_at(bytes, cursor + 0x23)? as usize;
        let body = cursor + RECORD_HEAD;
        let ext_at = body + layers_count * LAYER;
        let groups_at = ext_at + ext_count * EXT;

        let mut layers = Vec::with_capacity(layers_count);
        for slot in 0..layers_count {
            let at = body + slot * LAYER;
            let pair = bytes.get(at + 0x39).copied().unwrap_or(0xff) as i8 as i32;
            layers.push(Layer {
                min: [
                    rdr::f32_at(bytes, at).unwrap_or(0.0),
                    rdr::f32_at(bytes, at + 4).unwrap_or(0.0),
                    rdr::f32_at(bytes, at + 8).unwrap_or(0.0),
                ],
                max: [
                    rdr::f32_at(bytes, at + 0x0c).unwrap_or(0.0),
                    rdr::f32_at(bytes, at + 0x10).unwrap_or(0.0),
                    rdr::f32_at(bytes, at + 0x14).unwrap_or(0.0),
                ],
                color: rdr::u32_at(bytes, at + 0x18).unwrap_or(0),
                fog: rdr::u32_at(bytes, at + 0x1c).unwrap_or(0),
                center_x: rdr::f32_at(bytes, at + 0x20).unwrap_or(0.0),
                center_y: rdr::f32_at(bytes, at + 0x24).unwrap_or(0.0),
                center_z: rdr::f32_at(bytes, at + 0x28).unwrap_or(0.0),
                pitch: rdr::i16_at(bytes, at + 0x2c).unwrap_or(0),
                roll: rdr::i16_at(bytes, at + 0x2e).unwrap_or(0),
                scale_x: rdr::u16_at(bytes, at + 0x30).unwrap_or(0),
                scale_y: rdr::u16_at(bytes, at + 0x32).unwrap_or(0),
                flags: rdr::u16_at(bytes, at + 0x34).unwrap_or(0),
                ext: rdr::u8_at(bytes, at + 0x36).unwrap_or(0),
                band: rdr::u8_at(bytes, at + 0x37).unwrap_or(0),
                slot: rdr::u8_at(bytes, at + 0x38).unwrap_or(0),
                angle: rdr::u16_at(bytes, at + 0x3c).unwrap_or(0),
                panel: rdr::u16_at(bytes, at + 0x3a).unwrap_or(0) & 0x7ff,
                sample: rdr::u16_at(bytes, at + 0x3e).unwrap_or(0) & 0x7ff,
                pair,
                at,
            });
        }

        let mut groups = Vec::with_capacity(group_count);
        let mut group_cursor = groups_at;
        for _ in 0..group_count {
            let id = rdr::u16_at(bytes, group_cursor)?;
            let slot = rdr::u8_at(bytes, group_cursor + 2)?;
            let count = rdr::u8_at(bytes, group_cursor + 3)?;
            groups.push(GroupBlock {
                at: group_cursor,
                id,
                slot,
                count,
            });
            derived.group_entries += count as u64;
            group_cursor += GROUP_HEAD + count as usize * GROUP_ENTRY;
        }

        derived.layers += layers_count as u64;
        derived.ext_blocks += ext_count as u64;
        derived.group_blocks += group_count as u64;

        records.push(Record {
            index,
            at: cursor,
            size: group_cursor - cursor,
            min: [
                rdr::f32_at(bytes, cursor).unwrap_or(0.0),
                rdr::f32_at(bytes, cursor + 4).unwrap_or(0.0),
                rdr::f32_at(bytes, cursor + 8).unwrap_or(0.0),
            ],
            max: [
                rdr::f32_at(bytes, cursor + 0x0c).unwrap_or(0.0),
                rdr::f32_at(bytes, cursor + 0x10).unwrap_or(0.0),
                rdr::f32_at(bytes, cursor + 0x14).unwrap_or(0.0),
            ],
            column: rdr::u32_at(bytes, cursor + 0x18).unwrap_or(0),
            row: rdr::u32_at(bytes, cursor + 0x1c).unwrap_or(0),
            ext_at,
            ext_count,
            groups,
            groups_at,
            layers,
        });
        cursor = group_cursor;
        if cursor > bytes.len() {
            break;
        }
    }

    let mut masks = Vec::with_capacity(counts.masks as usize);
    for _ in 0..counts.masks {
        let rect = [
            rdr::i32_at(bytes, cursor + 0x24)?,
            rdr::i32_at(bytes, cursor + 0x28)?,
            rdr::i32_at(bytes, cursor + 0x2c)?,
            rdr::i32_at(bytes, cursor + 0x30)?,
        ];
        if rect[2] < rect[0] || rect[3] < rect[1] {
            break;
        }
        let width = (rect[2] - rect[0] + 1) as usize;
        let height = (rect[3] - rect[1] + 1) as usize;
        let data = cursor + MASK_HEAD;
        let area = width * height;
        if data + area * 4 > bytes.len() {
            break;
        }
        let mut marked = 0u32;
        for cell in 0..area {
            if rdr::u32_at(bytes, data + cell * 4).unwrap_or(0) >> 16 == 0x6641 {
                marked += 1;
            }
        }
        masks.push(Mask {
            at: cursor,
            world: [
                rdr::f32_at(bytes, cursor).unwrap_or(0.0),
                rdr::f32_at(bytes, cursor + 4).unwrap_or(0.0),
                rdr::f32_at(bytes, cursor + 8).unwrap_or(0.0),
                rdr::f32_at(bytes, cursor + 0x0c).unwrap_or(0.0),
                rdr::f32_at(bytes, cursor + 0x10).unwrap_or(0.0),
                rdr::f32_at(bytes, cursor + 0x14).unwrap_or(0.0),
            ],
            rect,
            marked,
            cells: area as u32,
            sample: rdr::u16_at(bytes, cursor + 0x34).unwrap_or(0),
            group: rdr::u16_at(bytes, cursor + 0x36).unwrap_or(0) as i16,
            flag: rdr::u16_at(bytes, cursor + 0x3a).unwrap_or(0),
            id: rdr::u16_at(bytes, cursor + 0x38).unwrap_or(0),
            data,
        });
        cursor = data + area * 4;
    }

    let mut f_records = Vec::with_capacity(counts.f_records as usize);
    for _ in 0..counts.f_records {
        if cursor + 0x18 > bytes.len() {
            break;
        }
        f_records.push(FRecord {
            at: cursor,
            rect: [
                rdr::f32_at(bytes, cursor).unwrap_or(0.0),
                rdr::f32_at(bytes, cursor + 4).unwrap_or(0.0),
                rdr::f32_at(bytes, cursor + 8).unwrap_or(0.0),
                rdr::f32_at(bytes, cursor + 0x0c).unwrap_or(0.0),
            ],
            tags: [
                rdr::u16_at(bytes, cursor + 0x10).unwrap_or(0),
                rdr::u16_at(bytes, cursor + 0x12).unwrap_or(0),
                rdr::u16_at(bytes, cursor + 0x14).unwrap_or(0),
                rdr::u16_at(bytes, cursor + 0x16).unwrap_or(0),
            ],
        });
        cursor += 0x18;
    }

    let mut words = Vec::with_capacity(counts.words as usize);
    for index in 0..counts.words as usize {
        words.push(rdr::u32_at(bytes, cursor + index * 4)?);
    }
    cursor += counts.words as usize * 4;

    let mut pairs = Vec::with_capacity(counts.pairs as usize);
    for _ in 0..counts.pairs {
        let Some((model, after)) = cstr(bytes, cursor) else {
            break;
        };
        let Some((texture, next)) = cstr(bytes, after) else {
            break;
        };
        pairs.push(Pair { model, texture });
        cursor = next;
    }

    let samples = take_names(bytes, counts.samples, &mut cursor);
    let group_names = take_names(bytes, counts.groups, &mut cursor);
    let names = take_names(bytes, counts.names, &mut cursor);
    let end = cursor;

    for record in &records {
        for layer in &record.layers {
            if layer.flags & 4 != 0 {
                derived.panel_layers += 1;
            } else {
                derived.model_layers += 1;
            }
        }
    }

    Some(Layout {
        counts,
        records,
        masks,
        f_records,
        words,
        pairs,
        samples,
        group_names,
        names,
        derived,
        end,
    })
}

fn take_names(bytes: &[u8], count: u32, cursor: &mut usize) -> Vec<String> {
    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let Some((value, next)) = cstr(bytes, *cursor) else {
            break;
        };
        out.push(value);
        *cursor = next;
    }
    out
}

fn cstr(bytes: &[u8], at: usize) -> Option<(String, usize)> {
    if at >= bytes.len() {
        return None;
    }
    let rest = &bytes[at..];
    let end = rest
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(rest.len());
    Some((rdr::utf8_string(&rest[..end]), at + end + 1))
}

pub fn vertices(bytes: &[u8], at: usize, count: usize) -> Vec<Vertex> {
    let mut out = Vec::with_capacity(count);
    for index in 0..count {
        let base = at + index * VERTEX;
        out.push(Vertex {
            x: rdr::f32_at(bytes, base).unwrap_or(0.0),
            y: rdr::f32_at(bytes, base + 4).unwrap_or(0.0),
            color: rdr::u32_at(bytes, base + 0x0c).unwrap_or(0),
            u: rdr::f32_at(bytes, base + 0x10).unwrap_or(0.0),
            v: rdr::f32_at(bytes, base + 0x14).unwrap_or(0.0),
        });
    }
    out
}

#[derive(Clone, Copy)]
pub struct Panel {
    pub page: u16,
    pub corners: [(f32, f32); 4],
    pub uv: [(f32, f32); 4],
}

/// Placed ground pieces of a group block, in the order the file stores them.
/// The block is three parallel arrays: `count` quads of four vertices, then
/// `count` placements (each written out four times, once per vertex), then a
/// tail of four pairs per quad that is still unread.
pub fn ground_quads(bytes: &[u8], block: &GroupBlock) -> Vec<GroundQuad> {
    let count = block.count as usize;
    let data = block.at + GROUP_HEAD;
    let places = data + count * GROUP_QUAD;
    let mut out = Vec::with_capacity(count);
    for index in 0..count {
        let quad = data + index * GROUP_QUAD;
        let place = places + index * GROUP_PLACE;
        let mut corners = [(0.0f32, 0.0f32); 4];
        let mut uv = [(0.0f32, 0.0f32); 4];
        for slot in 0..4 {
            let at = quad + slot * VERTEX;
            corners[slot] = (
                rdr::f32_at(bytes, at).unwrap_or(0.0),
                rdr::f32_at(bytes, at + 4).unwrap_or(0.0),
            );
            uv[slot] = (
                rdr::f32_at(bytes, at + 0x10).unwrap_or(0.0),
                rdr::f32_at(bytes, at + 0x14).unwrap_or(0.0),
            );
        }
        out.push(GroundQuad {
            at: (
                rdr::f32_at(bytes, place).unwrap_or(0.0),
                rdr::f32_at(bytes, place + 8).unwrap_or(0.0),
            ),
            corners,
            uv,
            color: rdr::u32_at(bytes, quad + 0x0c).unwrap_or(0xffff_ffff),
        });
    }
    out
}

/// Ground tiles of a record, in the order the file stores them.
pub fn ext_blocks(bytes: &[u8], record: &Record) -> Vec<Ext> {
    let mut out = Vec::with_capacity(record.ext_count);
    for index in 0..record.ext_count {
        let at = record.ext_at + index * EXT;
        if at + 6 * VERTEX > bytes.len() {
            break;
        }
        let verts = vertices(bytes, at, 6);
        out.push(Ext {
            center: verts[0],
            corners: [verts[1], verts[2], verts[3], verts[4]],
        });
    }
    out
}

/// Tiles per row and per column: the record's footprint divided by 128.
pub fn ext_grid(record: &Record) -> (usize, usize) {
    let cols = ((record.max[0] - record.min[0]).abs() / TERRAIN_CELL)
        .round()
        .max(1.0) as usize;
    let rows = ((record.max[2] - record.min[2]).abs() / TERRAIN_CELL)
        .round()
        .max(1.0) as usize;
    (cols, rows)
}

/// World centre of ground tile `index`, in the record's tile.
pub fn ext_center(record: &Record, index: usize) -> (f32, f32, f32) {
    let (cols, _) = ext_grid(record);
    let column = (index % cols.max(1)) as f32;
    let row = (index / cols.max(1)) as f32;
    (
        record.min[0] + TERRAIN_CELL * 0.5 + column * TERRAIN_CELL,
        record.min[2] + TERRAIN_CELL * 0.5 + row * TERRAIN_CELL,
        0.0,
    )
}

pub fn panels(bytes: &[u8]) -> Vec<Panel> {
    if bytes.get(..4) != Some(b"MPD\0") {
        return Vec::new();
    }
    let Some(count) = rdr::u32_at(bytes, 0x14) else {
        return Vec::new();
    };
    let origin = rdr::u32_at(bytes, 8).unwrap_or(0x20) as usize;
    let mut out = Vec::with_capacity(count as usize);
    for index in 0..count as usize {
        let at = origin + index * PANEL;
        let Some(block) = bytes.get(at..at + PANEL) else {
            break;
        };
        let page = rdr::u16_at(block, 4).unwrap_or(0);
        let mut corners = [(0.0f32, 0.0f32); 4];
        let mut uv = [(0.0f32, 0.0f32); 4];
        for corner in 0..4usize {
            let base = 8 + corner * VERTEX;
            corners[corner] = (
                rdr::f32_at(block, base).unwrap_or(0.0),
                rdr::f32_at(block, base + 4).unwrap_or(0.0),
            );
            uv[corner] = (
                rdr::f32_at(block, base + 0x10).unwrap_or(0.0),
                rdr::f32_at(block, base + 0x14).unwrap_or(0.0),
            );
        }
        out.push(Panel { page, corners, uv });
    }
    out
}

pub struct Scene {
    pub raster: Raster,
    pub map: String,
    pub layers: usize,
    pub drawn: usize,
    pub models: usize,
    pub quads: usize,
    pub hollow: usize,
    pub missing: usize,
    pub tiles: usize,
    pub clock: u64,
    pub scale: f32,
    pub window: [f32; 4],
}

pub fn stage(pair: &Pair) -> Option<String> {
    let name = pair.model.rsplit(['/', '\\']).next()?;
    let stem = name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(name);
    if stem.is_empty() {
        return None;
    }
    Some(stem.to_ascii_uppercase())
}

pub fn family(pair: &Pair) -> Option<String> {
    let stem = stage(pair)?;
    let (head, _) = stem.split_once('_')?;
    if head.is_empty() {
        return None;
    }
    Some(head.to_string())
}

pub fn matches(scope: &str, pair: &Pair) -> bool {
    let scope = scope.to_ascii_uppercase();
    if scope.contains('_') {
        return stage(pair).as_deref() == Some(scope.as_str());
    }
    family(pair).as_deref() == Some(scope.as_str())
}

pub struct FieldMap {
    pub name: String,
    pub group: String,
    pub files: Vec<String>,
    pub pairs: Vec<usize>,
    pub layers: usize,
    pub models: usize,
    pub props: Vec<PropUse>,
    pub tiles: usize,
    pub window: [f32; 4],
}

pub struct PropUse {
    pub name: String,
    pub count: u32,
}

pub fn maps(layout: &Layout) -> Vec<FieldMap> {
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut out: Vec<FieldMap> = Vec::new();

    for (slot, pair) in layout.pairs.iter().enumerate() {
        let (Some(name), Some(group)) = (stage(pair), family(pair)) else {
            continue;
        };
        let at = match index.get(&name) {
            Some(at) => *at,
            None => {
                index.insert(name.clone(), out.len());
                out.push(FieldMap {
                    name,
                    group,
                    files: Vec::new(),
                    pairs: Vec::new(),
                    layers: 0,
                    models: 0,
                    props: Vec::new(),
                    tiles: 0,
                    window: [f32::MAX, f32::MAX, f32::MIN, f32::MIN],
                });
                out.len() - 1
            }
        };
        out[at].pairs.push(slot);
        out[at].files.push(
            pair.model
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(&pair.model)
                .to_string(),
        );
    }

    let mut tiles: Vec<HashSet<usize>> = vec![HashSet::new(); out.len()];
    let mut prop_counts: Vec<HashMap<String, u32>> = vec![HashMap::new(); out.len()];
    for record in &layout.records {
        let mut touched: HashSet<usize> = HashSet::new();
        for layer in &record.layers {
            if layer.pair < 0 {
                continue;
            }
            let Some(name) = layout.pairs.get(layer.pair as usize).and_then(stage) else {
                continue;
            };
            let Some(at) = index.get(&name) else {
                continue;
            };
            touched.insert(*at);
            let entry = &mut out[*at];
            entry.layers += 1;
            if layer.flags & 4 == 0 {
                entry.models += 1;
                if layer.sample != SAMPLE_NONE {
                    if let Some(prop) = layout.samples.get(layer.sample as usize) {
                        *prop_counts[*at].entry(prop.clone()).or_insert(0) += 1;
                    }
                }
            }
            entry.window[0] = entry.window[0].min(layer.min[0]);
            entry.window[1] = entry.window[1].min(layer.min[1]);
            entry.window[2] = entry.window[2].max(layer.max[0]);
            entry.window[3] = entry.window[3].max(layer.max[1]);
        }
        for at in touched {
            tiles[at].insert(record.index);
        }
    }
    for (at, set) in tiles.iter().enumerate() {
        out[at].tiles = set.len();
    }
    for (at, bucket) in prop_counts.into_iter().enumerate() {
        let mut list: Vec<PropUse> = bucket
            .into_iter()
            .map(|(name, count)| PropUse { name, count })
            .collect();
        list.sort_by(|left, right| {
            right
                .count
                .cmp(&left.count)
                .then_with(|| left.name.cmp(&right.name))
        });
        list.truncate(PROP_ROWS);
        out[at].props = list;
    }

    out.sort_by_key(|entry| std::cmp::Reverse(entry.layers));
    out
}

pub fn default_scope(layout: &Layout, width: f32) -> Option<(String, String, f32)> {
    let catalog = maps(layout);
    let mut groups: HashMap<String, (u64, String)> = HashMap::new();
    for entry in &catalog {
        let slot = groups
            .entry(entry.group.clone())
            .or_insert((0, entry.name.clone()));
        slot.0 += entry.layers as u64;
        if entry.layers > 0 && slot.1.is_empty() {
            slot.1 = entry.name.clone();
        }
    }
    let group = groups
        .iter()
        .max_by_key(|(_, (layers, _))| *layers)
        .map(|(name, _)| name.clone())?;
    let x0 = default_window(layout, &group, width);
    let best = catalog
        .iter()
        .filter(|entry| entry.group == group)
        .max_by_key(|entry| entry.layers)?;
    Some((group, best.name.clone(), x0))
}

pub fn default_window(layout: &Layout, map: &str, span: f32) -> f32 {
    let mut centers: Vec<f32> = Vec::new();
    for record in &layout.records {
        for layer in &record.layers {
            if layer.pair < 0 {
                continue;
            }
            let Some(pair) = layout.pairs.get(layer.pair as usize) else {
                continue;
            };
            if !matches(map, pair) {
                continue;
            }
            centers.push((layer.min[0] + layer.max[0]) * 0.5);
        }
    }
    if centers.is_empty() {
        return 0.0;
    }
    centers.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));

    let span = span.max(1.0);
    let (mut best_start, mut best_count) = (centers[0], 0usize);
    let mut tail = 0usize;
    for head in 0..centers.len() {
        while centers[head] - centers[tail] > span {
            tail += 1;
        }
        let count = head - tail + 1;
        if count > best_count {
            best_count = count;
            best_start = centers[tail];
        }
    }
    best_start - span * 0.1
}

/// Every layer of one layer set that can show up in `x0..x1`, in file order and
/// with duplicates dropped: a tile record repeats a layer verbatim across the
/// tiles it covers, and only one copy may be drawn.
pub(crate) fn gather<'a>(layout: &'a Layout, map: &str, x0: f32, x1: f32) -> Vec<&'a Layer> {
    let slots: HashSet<usize> = layout
        .pairs
        .iter()
        .enumerate()
        .filter(|(_, pair)| matches(map, pair))
        .map(|(slot, _)| slot)
        .collect();
    if slots.is_empty() {
        return Vec::new();
    }
    let mut layers: Vec<&Layer> = Vec::new();
    let mut seen: HashSet<[u32; 9]> = HashSet::new();
    for record in &layout.records {
        for layer in &record.layers {
            if layer.pair < 0 || !slots.contains(&(layer.pair as usize)) {
                continue;
            }
            if layer.max[0] < x0 || layer.min[0] > x1 {
                continue;
            }
            let key = [
                layer.pair as u32,
                layer.panel as u32,
                layer.sample as u32,
                layer.flags as u32,
                layer.ext as u32,
                layer.min[0].to_bits(),
                layer.min[1].to_bits(),
                layer.max[0].to_bits(),
                layer.max[1].to_bits(),
            ];
            if !seen.insert(key) {
                continue;
            }
            layers.push(layer);
        }
    }
    layers
}

/// Pixel scale and vertical window for a layer set. A window is tight in x, so
/// the scale comes from `max_side / width`; the height is capped at twice that
/// and squeezed if the layer stack is taller.
pub(crate) fn fit(layers: &[&Layer], width: f32, max_side: u32) -> (f32, f32, f32) {
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;
    for layer in layers {
        let half = (layer.max[1] - layer.min[1]).abs() * 0.5;
        min_y = min_y.min(layer.center_y - half);
        max_y = max_y.max(layer.center_y + half);
    }
    let span_y = (max_y - min_y).max(1.0);
    let mut scale = max_side as f32 / width.max(1.0);
    if span_y * scale > max_side as f32 * 2.0 {
        scale = max_side as f32 * 2.0 / span_y;
    }
    (scale, min_y, max_y)
}

pub type PairLoader<'a> = dyn Fn(&Pair) -> Option<(Vec<u8>, Vec<u8>)> + 'a;
/// What a render asks for beyond the window itself.
#[derive(Clone, Copy, Default)]
pub struct Render {
    /// Engine clock in units of 1000 per 1/60 s tick; prop poses are sampled at it.
    pub clock: u64,
    /// Draw the ground tiles of every record that carries them.
    pub terrain: bool,
}

pub fn compose_window(
    bytes: &[u8],
    layout: &Layout,
    map: &str,
    x0: f32,
    width: f32,
    load: &PairLoader<'_>,
    max_side: u32,
) -> Option<Scene> {
    compose_window_with(
        bytes,
        layout,
        map,
        x0,
        width,
        load,
        max_side,
        Render::default(),
    )
}

pub fn compose_window_with(
    bytes: &[u8],
    layout: &Layout,
    map: &str,
    x0: f32,
    width: f32,
    load: &PairLoader<'_>,
    max_side: u32,
    render: Render,
) -> Option<Scene> {
    let width = width.max(1.0);
    let x1 = x0 + width;

    let layers = gather(layout, map, x0, x1);
    if layers.is_empty() {
        return None;
    }
    let (scale, min_y, max_y) = fit(&layers, width, max_side);
    let span_y = (max_y - min_y).max(1.0);

    let out_width = (width * scale).ceil().max(1.0) as u32;
    let out_height = (span_y * scale).ceil().max(1.0) as u32;
    let mut raster = Raster::new(out_width, out_height);

    let mut panel_cache: HashMap<usize, Vec<Panel>> = HashMap::new();
    let mut page_cache: HashMap<usize, Vec<(u32, u32, Vec<u8>)>> = HashMap::new();
    let mut model_cache: HashMap<usize, Option<mbs::Quad>> = HashMap::new();
    let mut drawn = 0usize;
    let mut models = 0usize;
    let mut quads = 0usize;
    let mut hollow = 0usize;
    let mut missing = 0usize;
    let mut tiles = 0usize;

    let mut order: Vec<&Layer> = layers;
    order.sort_by(|left, right| {
        right.min[2]
            .partial_cmp(&left.min[2])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if render.terrain {
        tiles = paint_terrain(
            bytes,
            layout,
            &mut raster,
            &load,
            &mut page_cache,
            &Canvas {
                left: x0,
                top: max_y,
                scale,
            },
            x0,
            x1,
        );
    }

    for layer in order {
        let Some(pair) = layout.pairs.get(layer.pair as usize) else {
            missing += 1;
            continue;
        };
        let key = layer.pair as usize;

        if !pair.is_panel() || layer.flags & 4 == 0 {
            let quad = model_cache.entry(key).or_insert_with(|| {
                load(pair)
                    .and_then(|(model, _)| mbs::parse_bytes(&model, MBS_TAG).ok())
                    .and_then(|v55| mbs::convert(&v55).ok())
            });
            let Some(quad) = quad else {
                missing += 1;
                continue;
            };
            let list = prop_at(quad, layer.panel as usize, render.clock);
            if list.is_empty() {
                hollow += 1;
                continue;
            }

            let pages = page_cache.entry(key).or_insert_with(|| {
                load(pair)
                    .and_then(|(_, texture)| decode_pages(&texture))
                    .unwrap_or_default()
            });
            let mut painted = 0usize;
            for piece in list {
                let Some((page_w, page_h, rgba)) = pages.get(piece.page) else {
                    continue;
                };
                let Some(page) = Page::new(*page_w, *page_h, rgba) else {
                    continue;
                };
                let corners = piece_corners(layer, piece.dst);
                let mut dst = [(0.0f32, 0.0f32); 4];
                for corner in 0..4 {
                    let world_x = layer.center_x + corners[corner].0;
                    let world_y = layer.center_y + corners[corner].1;
                    dst[corner] = ((world_x - x0) * scale, (max_y - world_y) * scale);
                }
                let tint = shade(layer.color, piece.color);
                paint_quad(&mut raster, dst, piece.uv, &page, tint);
                painted += 1;
            }
            if painted == 0 {
                missing += 1;
            } else {
                models += 1;
                quads += painted;
            }
            continue;
        }

        let list = panel_cache.entry(key).or_insert_with(|| {
            load(pair)
                .map(|(model, _)| panels(&model))
                .unwrap_or_default()
        });
        let Some(panel) = list.get(layer.panel as usize) else {
            missing += 1;
            continue;
        };

        let pages = page_cache.entry(key).or_insert_with(|| {
            load(pair)
                .and_then(|(_, texture)| decode_pages(&texture))
                .unwrap_or_default()
        });
        let Some((page_w, page_h, rgba)) = pages.get(panel.page as usize) else {
            missing += 1;
            continue;
        };
        let Some(page) = Page::new(*page_w, *page_h, rgba) else {
            missing += 1;
            continue;
        };

        let target = (layer.center_x, layer.center_y);
        let tint = tint(layer.color);
        let quad = quad_corners(layer, panel);

        let mut dst = [(0.0f32, 0.0f32); 4];
        for corner in 0..4 {
            let world_x = target.0 + quad[corner].0;
            let world_y = target.1 + quad[corner].1;
            dst[corner] = ((world_x - x0) * scale, (max_y - world_y) * scale);
        }
        paint_quad(&mut raster, dst, panel.uv, &page, tint);
        drawn += 1;
    }

    let total = drawn + models + hollow + missing;
    Some(Scene {
        raster,
        map: map.to_string(),
        layers: total,
        drawn,
        models,
        quads,
        hollow,
        missing,
        tiles,
        clock: render.clock,
        scale,
        window: [x0, min_y, x1, max_y],
    })
}

/// Both page space (`.mpd` panel corners) and model space (`.mbs` quads) put y
/// downwards, while the field world puts y upwards, so every corner is mirrored
/// into world orientation first and only then turned by the layer's roll and
/// tilted by its pitch.
fn turn(layer: &Layer, local_x: f32, local_y: f32) -> (f32, f32) {
    let (sin_roll, cos_roll) = (layer.roll as f32 * ANGLE_UNIT).sin_cos();
    let cos_pitch = (layer.pitch as f32 * ANGLE_UNIT).cos();
    (
        cos_roll * local_x + sin_roll * local_y,
        cos_pitch * (cos_roll * local_y - sin_roll * local_x),
    )
}

pub fn quad_corners(layer: &Layer, panel: &Panel) -> [(f32, f32); 4] {
    let mut low = (f32::MAX, f32::MAX);
    let mut high = (f32::MIN, f32::MIN);
    for corner in &panel.corners {
        low = (low.0.min(corner.0), low.1.min(corner.1));
        high = (high.0.max(corner.0), high.1.max(corner.1));
    }
    let centre = ((low.0 + high.0) * 0.5, (low.1 + high.1) * 0.5);

    let scale_x = layer.scale_x as f32 * SCALE_UNIT;
    let scale_y = layer.scale_y as f32 * SCALE_UNIT;

    let mut out = [(0.0f32, 0.0f32); 4];
    for (slot, corner) in panel.corners.iter().enumerate() {
        let local_x = (corner.0 - centre.0) * 0.5 * scale_x;
        let local_y = -(corner.1 - centre.1) * 0.5 * scale_y;
        out[slot] = turn(layer, local_x, local_y);
    }
    out
}

/// Only UNICORN OVERLORD ships a field background, so the mbs tag is fixed.
pub const MBS_TAG: &str = "swi_unic";
/// Model quads are authored in texels, so one model unit is one world unit at
/// scale 1 (a panel span is twice its texels, a model quad is not; ADR-0032).
pub const MODEL_UNIT: f32 = 1.0;

/// World box covered by every layer of one layer set: `x0, y0, x1, y1`.
pub fn scope_span(layout: &Layout, map: &str) -> Option<[f32; 4]> {
    let mut out: Option<[f32; 4]> = None;
    for record in &layout.records {
        for layer in &record.layers {
            if layer.pair < 0 {
                continue;
            }
            let Some(pair) = layout.pairs.get(layer.pair as usize) else {
                continue;
            };
            if !matches(map, pair) {
                continue;
            }
            let half = (layer.max[1] - layer.min[1]).abs() * 0.5;
            let box_ = out.get_or_insert([f32::MAX, f32::MAX, f32::MIN, f32::MIN]);
            box_[0] = box_[0].min(layer.min[0]);
            box_[1] = box_[1].min(layer.center_y - half);
            box_[2] = box_[2].max(layer.max[0]);
            box_[3] = box_[3].max(layer.center_y + half);
        }
    }
    out
}

/// The whole layer set in one strip, for the minimap. No ground tiles and no
/// props: at this scale one pixel is tens of world units.
pub fn overview(
    bytes: &[u8],
    layout: &Layout,
    map: &str,
    load: &PairLoader<'_>,
    max_side: u32,
) -> Option<Scene> {
    let span = scope_span(layout, map)?;
    let width = (span[2] - span[0]).max(1.0);
    compose_window(bytes, layout, map, span[0], width, load, max_side)
}

/// The whole layer set as one strip, wrapped as a projection so the content
/// pipeline can encode it.
pub fn overview_view(
    bytes: &[u8],
    layout: &Layout,
    dir: Option<&Path>,
    map: &str,
    max_side: u32,
) -> Projection {
    let mut out = Projection::new("UFB · field overview");
    let Some(dir) = dir else {
        out.note = Some("the file has no directory".to_string());
        return out;
    };
    let load = |pair: &Pair| read_pair(dir, pair);
    let Some(scene) = overview(bytes, layout, map, &load, max_side) else {
        out.note = Some(format!("layer set {map} has no layers"));
        return out;
    };
    out.color = Some(ColorMap {
        width: scene.raster.width,
        height: scene.raster.height,
        rgba: scene.raster.rgba.clone(),
        legend: format!(
            "Layer set {}: world x {}..{}, {} layers, 1 px = {:.1} world units.",
            scene.map,
            scene.window[0].round(),
            scene.window[2].round(),
            scene.layers,
            1.0 / scene.scale
        ),
    });
    out
}

struct Canvas {
    left: f32,
    top: f32,
    scale: f32,
}

impl Canvas {
    fn pixel(&self, x: f32, y: f32) -> (f32, f32) {
        ((x - self.left) * self.scale, (self.top - y) * self.scale)
    }
}

/// Ground tiles are 128-unit atlas rects that line the back of a record's tile.
/// A group block carries their placement outright, so it is read first; the
/// extension blocks keep the older geometry, which only gets used for a record
/// that has no group block at all.
///
/// A record without layers carries no resource pair of its own, so it inherits
/// the stage of the last record that did: the records are stored in tile order
/// and a stage runs over a contiguous block of tiles.
///
/// Every piece sits in the same 128-unit band, so depth is what decides the
/// picture: the band is painted far to near by the placement's z, which leaves
/// the near row showing.
fn paint_terrain(
    bytes: &[u8],
    layout: &Layout,
    raster: &mut Raster,
    load: &PairLoader<'_>,
    pages: &mut HashMap<usize, Vec<(u32, u32, Vec<u8>)>>,
    canvas: &Canvas,
    x0: f32,
    x1: f32,
) -> usize {
    let mut carry: Option<usize> = None;
    let mut pieces: Vec<(usize, GroundQuad)> = Vec::new();
    for record in &layout.records {
        let mut tally: HashMap<usize, usize> = HashMap::new();
        for layer in &record.layers {
            if layer.pair < 0 {
                continue;
            }
            let slot = layer.pair as usize;
            if layout.pairs.get(slot).map(Pair::is_panel).unwrap_or(false) {
                *tally.entry(slot).or_insert(0) += 1;
            }
        }
        if let Some((slot, _)) = tally.into_iter().max_by_key(|(_, count)| *count) {
            carry = Some(slot);
        }
        if record.ext_count == 0 && record.groups.is_empty() {
            continue;
        }
        if record.max[0] < x0 || record.min[0] > x1 {
            continue;
        }
        let Some(slot) = carry else {
            continue;
        };
        if layout.pairs.get(slot).is_none() {
            continue;
        }
        if !record.groups.is_empty() {
            for block in &record.groups {
                for quad in ground_quads(bytes, block) {
                    pieces.push((slot, quad));
                }
            }
            continue;
        }
        for (index, block) in ext_blocks(bytes, record).into_iter().enumerate() {
            let (world_x, world_z, _) = ext_center(record, index);
            let mut corners = [(0.0f32, 0.0f32); 4];
            let mut uv = [(0.0f32, 0.0f32); 4];
            for corner in 0..4 {
                let vertex = block.corners[corner];
                corners[corner] = (vertex.x, vertex.y);
                uv[corner] = (vertex.u, vertex.v);
            }
            pieces.push((
                slot,
                GroundQuad {
                    at: (world_x - 64.0, world_z - 64.0),
                    corners,
                    uv,
                    color: block.center.color,
                },
            ));
        }
    }

    pieces.sort_by(|left, right| {
        right
            .1
            .at
            .1
            .partial_cmp(&left.1.at.1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut drawn = 0usize;
    for (slot, quad) in pieces {
        if !pages.contains_key(&slot) {
            let Some(pair) = layout.pairs.get(slot) else {
                continue;
            };
            let list = load(pair)
                .and_then(|(_, texture)| decode_pages(&texture))
                .unwrap_or_default();
            pages.insert(slot, list);
        }
        let Some((page_w, page_h, rgba)) = pages.get(&slot).and_then(|list| list.first()) else {
            continue;
        };
        let Some(page) = Page::new(*page_w, *page_h, rgba) else {
            continue;
        };
        let mut dst = [(0.0f32, 0.0f32); 4];
        for corner in 0..4 {
            dst[corner] = canvas.pixel(quad.at.0 + quad.corners[corner].0, quad.corners[corner].1);
        }
        paint_quad(raster, dst, quad.uv, &page, tint(quad.color));
        drawn += 1;
    }
    drawn
}

#[derive(Clone, Copy)]
pub struct Piece {
    pub page: usize,
    pub dst: [(f32, f32); 4],
    pub uv: [(f32, f32); 4],
    pub color: [u8; 4],
}

/// One entry per skeleton of the paired `.mbs` at the rest pose (clock 0):
/// every prop is drawn with its own `s7` transform applied.
pub fn props(quad: &mbs::Quad) -> Vec<Vec<Piece>> {
    props_at(quad, 0)
}

pub fn props_at(quad: &mbs::Quad, clock: u64) -> Vec<Vec<Piece>> {
    (0..quad.skeleton.len())
        .map(|index| prop_at(quad, index, clock))
        .collect()
}

pub fn prop_at(quad: &mbs::Quad, index: usize, clock: u64) -> Vec<Piece> {
    match quad.skeleton.get(index) {
        Some(mbs::quad::Pad::Filled(skeleton)) => skeleton_pieces(quad, skeleton, clock),
        _ => Vec::new(),
    }
}

/// Pose of one animation at the engine clock. A tick is 1/60 s and advances the
/// clock by 1000 units; a timeline entry's `time` counts 60 Hz frames, so it is
/// multiplied by 1000 to compare. `matrix_mix_id` says how to reach the next key:
/// 0 holds the key, 1 blends, 2 is the engine's Catmull-Rom (treated as blend
/// here: the difference only shows on curves that overshoot).
fn pose_matrix(animation: &mbs::quad::Animation, index: usize, clock: u64) -> Option<[f64; 16]> {
    let timeline = &animation.timeline;
    let current = timeline.get(index)?;
    let flags = mbs::quad::hexdec(&current.debug);
    let mut move_ = current.transform.move_;
    let mut rotate = current.transform.rotate;
    let mut scale = current.transform.scale;

    if let Some(next) = timeline.get(index + 1) {
        let span = ((next.time - current.time).max(1) * 1000) as f32;
        let elapsed = clock.saturating_sub(current.time.max(0) as u64 * 1000) as f32;
        let mut t = if current.matrix_mix_id == 0 {
            0.0
        } else {
            elapsed / span
        };
        if current.in_rate > 1 {
            let steps = current.in_rate as f32;
            t = (t * steps).floor() / steps;
        }
        let t = t.clamp(0.0, 1.0);
        for axis in 0..3 {
            move_[axis] = blend(current.transform.move_[axis], next.transform.move_[axis], t);
            rotate[axis] = blend(
                current.transform.rotate[axis],
                next.transform.rotate[axis],
                t,
            );
        }
        for axis in 0..2 {
            scale[axis] = blend(current.transform.scale[axis], next.transform.scale[axis], t);
        }
    }

    Some(mbs::matrix::engine_matrix(
        &move_,
        &rotate,
        &scale,
        flags & 0x01 != 0,
        flags & 0x02 != 0,
    ))
}

/// Index of the key that is in force at `clock`; the last key holds forever.
fn timeline_index(animation: &mbs::quad::Animation, clock: u64) -> Option<usize> {
    if animation.timeline.is_empty() {
        return None;
    }
    Some(
        animation
            .timeline
            .partition_point(|entry| entry.time.max(0) as u64 * 1000 <= clock)
            .saturating_sub(1),
    )
}

fn blend(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

fn skeleton_pieces(quad: &mbs::Quad, skeleton: &mbs::quad::Skeleton, clock: u64) -> Vec<Piece> {
    let mut out = Vec::new();
    for bone in &skeleton.bone {
        if bone.attach.kind != "animation" {
            continue;
        }
        let Some(mbs::quad::Pad::Filled(animation)) = quad.animation.get(bone.attach.id as usize)
        else {
            continue;
        };
        let Some(index) = timeline_index(animation, clock) else {
            continue;
        };
        let Some(matrix) = pose_matrix(animation, index, clock) else {
            continue;
        };
        let entry = &animation.timeline[index];
        let Some(attach) = &entry.attach else {
            continue;
        };
        let Some(frame) = frame_of(quad, &attach.kind, attach.id) else {
            continue;
        };
        out.extend(pieces(quad, frame).into_iter().map(|mut piece| {
            for point in piece.dst.iter_mut() {
                *point = place(&matrix, *point);
            }
            piece
        }));
    }
    out
}

fn frame_of(quad: &mbs::Quad, kind: &str, id: i64) -> Option<usize> {
    match kind {
        "slot" => match quad.slot.get(id as usize) {
            Some(mbs::quad::Pad::Filled(slot)) => frame_of(quad, &slot[0].kind, slot[0].id),
            _ => None,
        },
        "keyframe" => Some(id as usize),
        _ => None,
    }
}

fn place(matrix: &[f64; 16], point: (f32, f32)) -> (f32, f32) {
    let x = f64::from(point.0);
    let y = f64::from(point.1);
    let out_x = x * matrix[0] + y * matrix[4] + matrix[12];
    let out_y = x * matrix[1] + y * matrix[5] + matrix[13];
    (out_x as f32, out_y as f32)
}

pub fn pieces(quad: &mbs::Quad, frame: usize) -> Vec<Piece> {
    let Some(mbs::quad::Pad::Filled(keyframe)) = quad.keyframe.get(frame) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for cell in &keyframe.layer {
        let mbs::quad::Pad::Filled(layer) = cell else {
            continue;
        };
        let Some(page) = layer.tex_id.filter(|page| *page >= 0) else {
            continue;
        };
        if layer.dstquad.len() < 8 {
            continue;
        }
        let source = layer
            .srcquad
            .clone()
            .unwrap_or_else(|| vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0]);
        if source.len() < 8 {
            continue;
        }
        let mut dst = [(0.0f32, 0.0f32); 4];
        let mut uv = [(0.0f32, 0.0f32); 4];
        for corner in 0..4 {
            dst[corner] = (layer.dstquad[corner * 2], layer.dstquad[corner * 2 + 1]);
            uv[corner] = (source[corner * 2], source[corner * 2 + 1]);
        }
        let color = match &layer.fogquad {
            Some(mbs::v55::FogQuad::One(text)) => parse_color(text),
            Some(mbs::v55::FogQuad::Four(list)) => parse_color(&list[0]),
            None => [255, 255, 255, 255],
        };
        out.push(Piece {
            page: page as usize,
            dst,
            uv,
            color,
        });
    }
    out
}

fn parse_color(text: &str) -> [u8; 4] {
    let digits: Vec<u8> = text
        .trim_start_matches('#')
        .bytes()
        .filter_map(|byte| (byte as char).to_digit(16).map(|digit| digit as u8))
        .collect();
    if digits.len() < 8 {
        return [255, 255, 255, 255];
    }
    [
        digits[0] * 16 + digits[1],
        digits[2] * 16 + digits[3],
        digits[4] * 16 + digits[5],
        digits[6] * 16 + digits[7],
    ]
}

pub fn piece_corners(layer: &Layer, dst: [(f32, f32); 4]) -> [(f32, f32); 4] {
    let scale_x = layer.scale_x as f32 * SCALE_UNIT;
    let scale_y = layer.scale_y as f32 * SCALE_UNIT;

    let mut out = [(0.0f32, 0.0f32); 4];
    for (slot, (x, y)) in dst.iter().enumerate() {
        out[slot] = turn(layer, x * MODEL_UNIT * scale_x, -y * MODEL_UNIT * scale_y);
    }
    out
}

pub fn model_bounds(layer: &Layer, list: &[Piece]) -> Option<[f32; 4]> {
    let mut low = (f32::MAX, f32::MAX);
    let mut high = (f32::MIN, f32::MIN);
    for piece in list {
        for (x, y) in piece_corners(layer, piece.dst) {
            low = (low.0.min(x), low.1.min(y));
            high = (high.0.max(x), high.1.max(y));
        }
    }
    if low.0 > high.0 {
        return None;
    }
    Some([
        layer.center_x + low.0,
        layer.center_y + low.1,
        layer.center_x + high.0,
        layer.center_y + high.1,
    ])
}

pub fn tint(color: u32) -> [u8; 4] {
    [
        color as u8,
        (color >> 8) as u8,
        (color >> 16) as u8,
        (color >> 24) as u8,
    ]
}

pub fn shade(base: u32, color: [u8; 4]) -> [u8; 4] {
    let layer = tint(base);
    let mut out = [0u8; 4];
    for channel in 0..4 {
        out[channel] = (layer[channel] as u32 * color[channel] as u32 / 255) as u8;
    }
    out
}

fn decode_pages(texture: &[u8]) -> Option<Vec<(u32, u32, Vec<u8>)>> {
    let list = crate::ftx::decode_bytes(texture).ok()?;
    Some(
        list.into_iter()
            .map(|page| (page.width, page.height, page.rgba()))
            .collect(),
    )
}

pub fn read_pair(dir: &Path, pair: &Pair) -> Option<(Vec<u8>, Vec<u8>)> {
    let model = base_name(&pair.model)?;
    let texture = base_name(&pair.texture)?;
    let model = std::fs::read(dir.join(model)).ok()?;
    let texture = std::fs::read(dir.join(texture)).ok()?;
    Some((model, texture))
}

fn base_name(path: &str) -> Option<PathBuf> {
    let name = path.rsplit(['/', '\\']).next()?;
    if name.is_empty() {
        return None;
    }
    Some(PathBuf::from(name))
}

pub struct Check {
    pub id: &'static str,
    pub what: &'static str,
    pub ok: bool,
    pub detail: String,
}

pub fn checks(layout: &Layout, size: usize) -> Vec<Check> {
    let counts = layout.counts;
    let derived = layout.derived;
    let mut out = Vec::new();

    out.push(Check {
        id: "ufb.eof",
        what: "every byte is consumed by a section (slack = 0)",
        ok: layout.end == size,
        detail: format!("ends at 0x{:x} / file 0x{size:x}", layout.end),
    });
    out.push(Check {
        id: "ufb.tiles",
        what: "records fill the 138 x 105 tile grid and carry column = index % 138, row = index / 138",
        ok: layout.records.len() == counts.records as usize
            && counts.records as usize == COLUMNS * ROWS
            && layout
                .records
                .iter()
                .enumerate()
                .all(|(index, record)| {
                    record.column as usize == index % COLUMNS
                        && record.row as usize == index / COLUMNS
                }),
        detail: format!(
            "{}/{} records, {} x {} tiles",
            layout.records.len(),
            counts.records,
            COLUMNS,
            ROWS
        ),
    });
    out.push(Check {
        id: "ufb.objects",
        what: "model objects + panel objects equals the object count, and both match the layers",
        ok: counts.model_objects + counts.panel_objects == counts.objects
            && derived.model_layers + counts.f_records as u64 == counts.model_objects as u64
            && derived.panel_layers == counts.panel_objects as u64,
        detail: format!(
            "{} + {} = {} ; {} model layers + {} F records, {} panel layers",
            counts.model_objects,
            counts.panel_objects,
            counts.objects,
            derived.model_layers,
            counts.f_records,
            derived.panel_layers
        ),
    });
    out.push(Check {
        id: "ufb.f_records",
        what: "every F record is a non-degenerate world rectangle followed by four u16 tags",
        ok: layout.f_records.len() == counts.f_records as usize
            && layout
                .f_records
                .iter()
                .all(|record| record.rect[0] < record.rect[2] && record.rect[1] < record.rect[3]),
        detail: format!(
            "{} rectangles, tags {}",
            layout.f_records.len(),
            layout
                .f_records
                .first()
                .map(|record| record
                    .tags
                    .iter()
                    .map(|tag| tag.to_string())
                    .collect::<Vec<_>>()
                    .join(", "))
                .unwrap_or_default()
        ),
    });
    out.push(Check {
        id: "ufb.groups",
        what: "group block count equals the sum of the per-record group counts",
        ok: derived.group_blocks == counts.group_blocks as u64,
        detail: format!(
            "{} blocks, {} entries",
            derived.group_blocks, derived.group_entries
        ),
    });
    out.push(Check {
        id: "ufb.masks",
        what: "mask count matches the header and every mask carries its own index",
        ok: layout.masks.len() == counts.masks as usize
            && layout
                .masks
                .iter()
                .enumerate()
                .all(|(index, mask)| mask.id as usize == index),
        detail: format!("{} masks", layout.masks.len()),
    });
    out.push(Check {
        id: "ufb.mask.cell",
        what: "every mask rectangle spans its world box at 32 world units per cell",
        ok: layout.masks.iter().all(|mask| {
            let width = (mask.rect[2] - mask.rect[0] + 1) as f32 * MASK_CELL;
            let height = (mask.rect[3] - mask.rect[1] + 1) as f32 * MASK_CELL;
            (width - (mask.world[3] - mask.world[0])).abs() < 1.0
                && (height - (mask.world[5] - mask.world[2])).abs() < 1.0
        }),
        detail: format!("{} masks at {MASK_CELL} units per cell", layout.masks.len()),
    });
    out.push(Check {
        id: "ufb.world",
        what: "every mask box sits inside the 2199 x 1311 world grid of Field.ucb",
        ok: layout.masks.iter().all(|mask| {
            mask.world[0] >= WORLD_X0 - 1.0
                && mask.world[3] <= WORLD_X0 + GRID_COLUMNS as f32 * MASK_CELL + 1.0
                && mask.world[2] >= WORLD_Z0 - 1.0
                && mask.world[5] <= WORLD_Z0 + GRID_ROWS as f32 * MASK_CELL + 1.0
        }),
        detail: format!(
            "world x {}..{}, z {}..{} at {MASK_CELL} units per cell",
            WORLD_X0,
            WORLD_X0 + GRID_COLUMNS as f32 * MASK_CELL,
            WORLD_Z0,
            WORLD_Z0 + GRID_ROWS as f32 * MASK_CELL
        ),
    });
    out.push(Check {
        id: "ufb.pairs",
        what: "resource pair count matches the header",
        ok: layout.pairs.len() == counts.pairs as usize,
        detail: format!("{}/{}", layout.pairs.len(), counts.pairs),
    });
    out.push(Check {
        id: "ufb.names",
        what: "the name pool holds samples + group names + names and ends at EOF",
        ok: layout.samples.len() == counts.samples as usize
            && layout.group_names.len() == counts.groups as usize
            && layout.names.len() == counts.names as usize
            && layout.end == size,
        detail: format!(
            "{} samples + {} groups + {} names = {}",
            layout.samples.len(),
            layout.group_names.len(),
            layout.names.len(),
            layout.samples.len() + layout.group_names.len() + layout.names.len()
        ),
    });

    out
}

pub struct Grid {
    pub ids: Vec<u16>,
    pub painted: u64,
    pub underlay: u32,
    pub outside: u64,
}

impl Grid {
    pub fn at(&self, column: usize, row: usize) -> u16 {
        self.ids[row * GRID_COLUMNS + column]
    }
}

pub fn grid_x(world_x: f32) -> f32 {
    (world_x - WORLD_X0) / MASK_CELL
}

pub fn grid_y(world_z: f32) -> f32 {
    (world_z - WORLD_Z0) / MASK_CELL
}

pub fn rasterise(layout: &Layout, bytes: &[u8]) -> Grid {
    let mut ids = vec![0u16; GRID_COLUMNS * GRID_ROWS];
    let mut order: Vec<usize> = (0..layout.masks.len()).collect();
    order.sort_by_key(|index| {
        let mask = &layout.masks[*index];
        (
            u8::from(mask.flag != MASK_UNDERLAY),
            mask.sample,
            std::cmp::Reverse(*index),
        )
    });

    let mut painted = 0u64;
    let mut underlay = 0u32;
    let mut outside = 0u64;
    for index in order {
        let mask = &layout.masks[index];
        if mask.flag == MASK_UNDERLAY {
            underlay += 1;
        }
        let width = (mask.rect[2] - mask.rect[0] + 1).max(1) as usize;
        let height = (mask.rect[3] - mask.rect[1] + 1).max(1) as usize;
        for row in 0..height {
            let y = grid_y(mask.world[2] + row as f32 * MASK_CELL).round();
            let inside_row = (0.0..GRID_ROWS as f32).contains(&y);
            let py = if inside_row { y as usize } else { 0 };
            for column in 0..width {
                let word = rdr::u32_at(bytes, mask.data + (row * width + column) * 4).unwrap_or(0);
                if word >> 16 != 0x6641 {
                    continue;
                }
                let x = grid_x(mask.world[0] + column as f32 * MASK_CELL).round();
                if !inside_row || !(0.0..GRID_COLUMNS as f32).contains(&x) {
                    outside += 1;
                    continue;
                }
                ids[py * GRID_COLUMNS + x as usize] = index as u16;
                painted += 1;
            }
        }
    }

    Grid {
        ids,
        painted,
        underlay,
        outside,
    }
}

pub struct Match {
    pub cells: u64,
    pub same: u64,
    pub open: u64,
    pub painted: u64,
    pub clipped: u64,
    pub underlay: u32,
    pub first: Option<(usize, usize, u16, u16)>,
}

impl Match {
    pub fn share(&self) -> f64 {
        self.same as f64 / self.cells.max(1) as f64
    }
}

pub fn cross_check(layout: &Layout, bytes: &[u8], ucb: &[u8]) -> Option<Match> {
    let cells = GRID_COLUMNS * GRID_ROWS;
    if ucb.len() != cells * super::document::ucb::RECORD {
        return None;
    }
    let grid = rasterise(layout, bytes);
    let mut same = 0u64;
    let mut open = 0u64;
    let mut first = None;
    for index in 0..cells {
        let word = rdr::u32_at(ucb, index * super::document::ucb::RECORD + 8).unwrap_or(0);
        let actual = (word & 0xffff) as u16;
        if word == 0 {
            open += 1;
        }
        if actual == grid.ids[index] {
            same += 1;
        } else if first.is_none() {
            first = Some((
                index % GRID_COLUMNS,
                index / GRID_COLUMNS,
                actual,
                grid.ids[index],
            ));
        }
    }
    Some(Match {
        cells: cells as u64,
        same,
        open,
        painted: grid.painted,
        clipped: grid.outside,
        underlay: grid.underlay,
        first,
    })
}

/// Prepare a window for playback, defaulting the layer set and the left edge
/// exactly the way a view does.
#[allow(clippy::too_many_arguments)]
pub fn prepare_window(
    bytes: &[u8],
    layout: &Layout,
    dir: &Path,
    map: Option<&str>,
    x0: Option<f32>,
    width: f32,
    max_side: u32,
    render: Render,
) -> Option<Playback> {
    let width = width.clamp(400.0, 12000.0);
    let scope = map
        .map(str::to_string)
        .or_else(|| default_scope(layout, width).map(|(_, stage, _)| stage))?;
    let x0 = x0.unwrap_or_else(|| default_window(layout, &scope, width));
    let load = |pair: &Pair| read_pair(dir, pair);
    Playback::prepare(bytes, layout, &scope, x0, width, &load, max_side, render)
}

pub const WINDOW_DEFAULT: f32 = 3000.0;

pub fn view(bytes: &[u8], dir: Option<&Path>, max_side: u32) -> Projection {
    view_at(bytes, dir, None, None, WINDOW_DEFAULT, max_side)
}

pub fn view_at(
    bytes: &[u8],
    dir: Option<&Path>,
    map: Option<&str>,
    x0: Option<f32>,
    width: f32,
    max_side: u32,
) -> Projection {
    let Some(layout) = parse(bytes) else {
        let mut out = Projection::new("UFB · field background");
        out.note = Some("file is shorter than 0x88 bytes".to_string());
        return out;
    };
    view_layout(
        &layout,
        bytes,
        dir,
        map,
        x0,
        width,
        max_side,
        Render::default(),
    )
}

fn header(layout: &Layout) -> (Projection, Vec<Vec<String>>) {
    let mut out = Projection::new("UFB · field background");
    let counts = layout.counts;
    let derived = layout.derived;
    let total_names = layout.samples.len() + layout.group_names.len() + layout.names.len();

    out.note = Some(format!(
        "{} records on a {}×{} tile grid · {} layers ({} panel / {} model) · {} masks · {} pairs · {} strings",
        counts.records,
        COLUMNS,
        ROWS,
        derived.layers,
        derived.panel_layers,
        derived.model_layers,
        counts.masks,
        counts.pairs,
        total_names
    ));

    let rows = vec![
        row2("records", counts.records),
        row2("group blocks", counts.group_blocks),
        row2("objects", counts.objects),
        row2("model objects", counts.model_objects),
        row2("panel objects", counts.panel_objects),
        row2("area masks", counts.masks),
        row2("F records", counts.f_records),
        row2("u32 table", counts.words),
        row2("resource pairs", counts.pairs),
        row2("samples", counts.samples),
        row2("group names", counts.groups),
        row2("names", counts.names),
        vec![
            "tile grid".to_string(),
            format!("{} × {} = {}", COLUMNS, ROWS, COLUMNS as u64 * ROWS as u64),
        ],
        vec![
            "layers".to_string(),
            format!(
                "{} ({} panel, {} model)",
                derived.layers, derived.panel_layers, derived.model_layers
            ),
        ],
        vec![
            "group entries".to_string(),
            derived.group_entries.to_string(),
        ],
    ];
    (out, rows)
}

/// One scene as a projection: the picture plus the sentence that says what was
/// drawn, without the format tables.
pub fn scene_projection(scene: Scene, layout: &Layout) -> Projection {
    let (mut out, mut header_rows) = header(layout);
    out.color = Some(ColorMap {
        width: scene.raster.width,
        height: scene.raster.height,
        rgba: scene.raster.rgba.clone(),
        legend: scene_legend(&scene),
    });
    header_rows.push(vec!["layer set".to_string(), scene.map.clone()]);
    header_rows.push(vec![
        "world window".to_string(),
        format!(
            "x {}..{} · {} layers · {} panels · {} props ({}) · {} ground tiles",
            scene.window[0].round(),
            scene.window[2].round(),
            scene.layers,
            scene.drawn,
            scene.models,
            scene.quads,
            scene.tiles
        ),
    ]);
    out
}

/// What a rendered window is, in one sentence.
pub fn scene_legend(scene: &Scene) -> String {
    let mut legend = format!(
        "Layer set {}: {} layers inside the window, {} textured panels drawn, {} props drawn from {} model quads, {} prop layers with no art, {} unresolved. 1 px = {:.2} world units. World window x {}..{}, y {}..{} with y growing upwards. A panel layer names a quad of the paired .mpd and is textured from the paired .ftx; a prop layer names a skeleton of the paired .mbs, and that whole prop is posed at the engine clock (tick {}, 1/60 s each) and then placed. Both are kept as authored: page space and model space put y downwards, so every quad is mirrored into the upwards world before the layer's scale, roll and pitch are applied; they are painted far to near by z.",
        scene.map,
        scene.layers,
        scene.drawn,
        scene.models,
        scene.quads,
        scene.hollow,
        scene.missing,
        1.0 / scene.scale,
        scene.window[0].round(),
        scene.window[2].round(),
        scene.window[1].round(),
        scene.window[3].round(),
        scene.clock,
    );
    if scene.tiles > 0 {
        legend.push_str(&format!(
            " {} ground tiles are drawn behind the billboards: a group block lists each 128-unit atlas rect with the world cell it is pinned to, and the band is painted far to near so the near row shows. A record with no layers inherits the stage of the last record that had one.",
            scene.tiles
        ));
    }
    legend.push_str(" One tile record mixes layers from several layer sets, so the set is chosen first and only its layers are drawn. Stages of the same battlefield (.00P, .02P, .00F2 …) can be stacked, but the .01P kind is soft cloud: it covers the ground by design, so the viewer shows one set unless the scope names a whole battlefield (AC00).");
    legend
}

pub fn view_layout(
    layout: &Layout,
    bytes: &[u8],
    dir: Option<&Path>,
    map: Option<&str>,
    x0: Option<f32>,
    width: f32,
    max_side: u32,
    render: Render,
) -> Projection {
    let (mut out, mut header_rows) = header(layout);

    let catalog = maps(layout);
    let total_names = layout.samples.len() + layout.group_names.len() + layout.names.len();
    let width = width.clamp(400.0, 12000.0);
    let scope = map
        .map(|name| name.to_string())
        .or_else(|| default_scope(layout, width).map(|(_, stage, _)| stage));

    let rendered = scope.and_then(|scope| {
        let dir = dir?;
        let x0 = x0.unwrap_or_else(|| default_window(layout, &scope, width));
        let load = |pair: &Pair| read_pair(dir, pair);
        compose_window_with(bytes, layout, &scope, x0, width, &load, max_side, render)
    });

    if let Some(scene) = rendered {
        let legend = scene_legend(&scene);
        out.color = Some(ColorMap {
            width: scene.raster.width,
            height: scene.raster.height,
            rgba: scene.raster.rgba.clone(),
            legend,
        });
        header_rows.push(vec!["layer set".to_string(), scene.map.clone()]);
        header_rows.push(vec![
            "world window".to_string(),
            format!(
                "x {}..{} · {} layers · {} panels · {} props ({}) · {} ground tiles",
                scene.window[0].round(),
                scene.window[2].round(),
                scene.layers,
                scene.drawn,
                scene.models,
                scene.quads,
                scene.tiles
            ),
        ]);
    } else if let Some((width, height, rgba, legend)) = area_image(layout, bytes) {
        out.color = Some(ColorMap {
            width,
            height,
            rgba,
            legend,
        });
    }
    if let Some(dir) = dir {
        if let Ok(ucb) = std::fs::read(dir.join("Field.ucb")) {
            if let Some(m) = cross_check(layout, bytes, &ucb) {
                header_rows.push(vec![
                    "ucb match".to_string(),
                    format!("{:.3}% of {} cells", m.share() * 100.0, m.cells),
                ]);
                let rows = vec![
                    row2("cells compared", m.cells as u32),
                    vec!["identical".to_string(), m.same.to_string()],
                    vec!["mask cells painted".to_string(), m.painted.to_string()],
                    vec!["mask cells clipped".to_string(), m.clipped.to_string()],
                    vec!["cells with no region".to_string(), m.open.to_string()],
                    vec!["underlay masks painted first".to_string(), m.underlay.to_string()],
                    vec![
                        "first mismatch".to_string(),
                        match m.first {
                            Some((x, y, actual, painted)) => format!(
                                "({x}, {y}) ucb {actual} vs painted {painted}"
                            ),
                            None => "none".to_string(),
                        },
                    ],
                    vec![
                        "rule".to_string(),
                        format!(
                            "paint every mask cell (0x6641) at grid({}, {}) + cell x {MASK_CELL} in order of its +0x34 field; masks flagged {MASK_UNDERLAY:#06x} go first and end up fully covered",
                            WORLD_X0, WORLD_Z0
                        ),
                    ],
                ];
                let total = m.cells;
                out.sections.push(Section::new(
                    "UCB cross-check",
                    &["item", "value"],
                    rows,
                    total,
                ));
            }
        }
    }
    out.sections.push(Section::new(
        "Header (cross-checked counts)",
        &["item", "value"],
        header_rows,
        15,
    ));

    let map_rows: Vec<Vec<String>> = catalog
        .iter()
        .take(28)
        .map(|entry| {
            vec![
                entry.name.clone(),
                entry.group.clone(),
                entry.files.len().to_string(),
                entry.layers.to_string(),
                entry.tiles.to_string(),
                format!(
                    "{:.0}×{:.0}",
                    entry.window[2] - entry.window[0],
                    entry.window[3] - entry.window[1]
                ),
                format!(
                    "({:.0}, {:.0})..({:.0}, {:.0})",
                    entry.window[0], entry.window[1], entry.window[2], entry.window[3]
                ),
            ]
        })
        .collect();
    out.sections.push(Section::new(
        "Layer sets (one battlefield stage each)",
        &[
            "stage",
            "battlefield",
            "files",
            "layers",
            "tiles",
            "size",
            "world box",
        ],
        map_rows,
        catalog.len() as u64,
    ));

    let layer_rows = vec![
        row3("+0x00", "3 × f32", "layer box min (x, y, z)"),
        row3("+0x0c", "3 × f32", "layer box max"),
        row3("+0x18", "u32", "tint RGBA"),
        row3("+0x1c", "u32", "fog RGB, 0xffffff = none"),
        row3("+0x20", "f32", "box center x"),
        row3(
            "+0x24",
            "f32",
            "quad centre y (same triple as +0x20 / +0x28)",
        ),
        row3("+0x28", "f32", "box center z"),
        row3("+0x2c", "u32", "rotation words, zero in 92% of layers"),
        row3("+0x30", "2 × u16", "scale x / y, 1.0 = 0x1000"),
        row3(
            "+0x34",
            "u16",
            "flags: bit 2 = page panel (clear = prop of the paired .mbs); the other bits are not decoded yet",
        ),
        row3("+0x36", "u8", "extension block index"),
        row3(
            "+0x37",
            "u8",
            "draw band 0/1, folded into the runtime sort key",
        ),
        row3("+0x38", "u8", "slot number from 16, 0xff on model layers"),
        row3("+0x39", "s8", "resource pair index"),
        row3(
            "+0x3a",
            "u16",
            "quad index, & 0x7ff: a page panel when bit 2 of +0x34 is set, otherwise a prop (skeleton) of the paired .mbs",
        ),
        row3(
            "+0x3c",
            "u16",
            "angle in degrees, non-zero on 11 layers only",
        ),
        row3("+0x3e", "u16", "sample index, & 0x7ff, 2047 = none"),
    ];
    out.sections.push(Section::new(
        "Layer record (0x40 bytes)",
        &["offset", "type", "meaning"],
        layer_rows,
        17,
    ));

    let vertex_rows = vec![
        row3("+0x00", "f32 x", "local x"),
        row3("+0x04", "f32 y", "local y"),
        row3("+0x08", "f32", "zero in all 1587606 extension vertices"),
        row3("+0x0c", "u32", "vertex RGBA"),
        row3("+0x10", "f32 u", "atlas u"),
        row3("+0x14", "f32 v", "atlas v"),
    ];
    out.sections.push(Section::new(
        "Vertex (0x18 bytes, shared by extension blocks and group entries)",
        &["offset", "type", "meaning"],
        vertex_rows,
        6,
    ));

    let area_rows: Vec<Vec<String>> = layout
        .masks
        .iter()
        .take(MAX_AREAS)
        .enumerate()
        .map(|(index, mask)| {
            vec![
                index.to_string(),
                format!(
                    "({:.0}, {:.0})..({:.0}, {:.0})",
                    mask.world[0], mask.world[2], mask.world[3], mask.world[5]
                ),
                format!(
                    "({}, {})..({}, {})",
                    mask.rect[0], mask.rect[1], mask.rect[2], mask.rect[3]
                ),
                format!(
                    "{:.1}%",
                    100.0 * mask.marked as f64 / mask.cells.max(1) as f64
                ),
                mask.sample.to_string(),
                match layout.area_name(mask) {
                    Some(name) => format!("{} · {name}", mask.group),
                    None => "—".to_string(),
                },
                format!("{:#06x}", mask.flag),
            ]
        })
        .collect();
    out.sections.push(Section::new(
        "Area masks (first 24)",
        &[
            "#",
            "world box",
            "cell rect",
            "marked",
            "sample",
            "area",
            "kind",
        ],
        area_rows,
        layout.masks.len() as u64,
    ));

    let f_rows: Vec<Vec<String>> = layout
        .f_records
        .iter()
        .map(|record| {
            vec![
                format!("{:#x}", record.at),
                format!(
                    "x {:.0}..{:.0}, z {:.0}..{:.0}",
                    record.rect[0], record.rect[2], record.rect[1], record.rect[3],
                ),
                record
                    .tags
                    .iter()
                    .map(|tag| tag.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
            ]
        })
        .collect();
    out.sections.push(Section::new(
        "F records (world rectangles)",
        &["offset", "rect", "tags"],
        f_rows,
        layout.f_records.len() as u64,
    ));

    let pair_rows: Vec<Vec<String>> = layout
        .pairs
        .iter()
        .take(48)
        .map(|pair| vec![pair.model.clone(), pair.texture.clone()])
        .collect();
    out.sections.push(Section::new(
        "Resource pairs",
        &["model / panels", "texture atlas"],
        pair_rows,
        layout.pairs.len() as u64,
    ));

    let mut name_rows: Vec<Vec<String>> = Vec::new();
    for (index, name) in layout.samples.iter().take(6).enumerate() {
        name_rows.push(vec!["sample".to_string(), index.to_string(), name.clone()]);
    }
    for (index, name) in layout.group_names.iter().take(6).enumerate() {
        name_rows.push(vec!["group".to_string(), index.to_string(), name.clone()]);
    }
    for (index, name) in layout.names.iter().enumerate() {
        name_rows.push(vec!["name".to_string(), index.to_string(), name.clone()]);
    }
    out.sections.push(Section::new(
        "Name pool (first of each block)",
        &["block", "#", "text"],
        name_rows,
        total_names as u64,
    ));

    out
}

fn area_image(layout: &Layout, bytes: &[u8]) -> Option<(u32, u32, Vec<u8>, String)> {
    let index = layout
        .masks
        .iter()
        .enumerate()
        .filter(|(_, mask)| {
            let ratio = mask.marked as f64 / mask.cells.max(1) as f64;
            (0.2..=0.6).contains(&ratio)
        })
        .max_by_key(|(_, mask)| mask.cells)
        .or_else(|| {
            layout
                .masks
                .iter()
                .enumerate()
                .max_by_key(|(_, mask)| mask.marked)
        })
        .map(|(index, _)| index)?;
    let mask = layout.masks[index];
    let width = (mask.rect[2] - mask.rect[0] + 1).max(1) as usize;
    let height = (mask.rect[3] - mask.rect[1] + 1).max(1) as usize;
    let step = width.max(height).div_ceil(800).max(1);
    let out_width = width.div_ceil(step) as u32;
    let out_height = height.div_ceil(step) as u32;
    let mut raster = Raster::filled(out_width, out_height, [10, 12, 18, 255]);
    let hue = super::content::area_hue(index as u32);
    for row in 0..height {
        let py = (row / step) as u32;
        for column in 0..width {
            let px = (column / step) as u32;
            let word = rdr::u32_at(bytes, mask.data + (row * width + column) * 4).unwrap_or(0);
            let color = if word >> 16 == 0x6641 {
                super::content::hsv(hue, 0.55, 0.98)
            } else {
                super::content::hsv(hue, 0.55, 0.16)
            };
            raster.blend(px, py, [color[0], color[1], color[2], 255]);
        }
    }
    let area = layout
        .area_name(&mask)
        .map(|name| name.to_string())
        .unwrap_or_else(|| "unnamed".to_string());
    Some((
        out_width,
        out_height,
        raster.rgba,
        format!(
            "Area mask #{index} of {} — {area}: cell rect ({}, {})..({}, {}), world box ({:.0}, {:.0})..({:.0}, {:.0}). Bright cells carry 0x6641 (inside the area), dark cells 0x6640. The paired .ftx and .mpd files hold the pixels; open the folder next to Field.ufb to render the textured scene.",
            layout.masks.len(),
            mask.rect[0],
            mask.rect[1],
            mask.rect[2],
            mask.rect[3],
            mask.world[0],
            mask.world[2],
            mask.world[3],
            mask.world[5]
        ),
    ))
}

fn row2(key: &str, value: u32) -> Vec<String> {
    vec![key.to_string(), value.to_string()]
}

fn row3(offset: &str, kind: &str, meaning: &str) -> Vec<String> {
    vec![offset.to_string(), kind.to_string(), meaning.to_string()]
}
