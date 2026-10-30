use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::content::{ColorMap, Projection, Section};
use super::document::rdr;
use super::raster::{paint_quad, Page, Raster};

pub const HEAD: usize = 0x88;
const LAYER: usize = 0x40;
const PANEL: usize = 0xa0;
const MAX_AREAS: usize = 24;

#[derive(Clone, Copy, Default)]
pub struct Header {
    pub records: u32,
    pub layers: u32,
    pub areas: u32,
    pub f_records: u32,
    pub words: u32,
    pub pairs: u32,
    pub samples: u32,
    pub names: u32,
    pub records_end: usize,
}

#[derive(Clone, Copy)]
pub struct Layer {
    pub pair: i32,
    pub panel: u16,
    pub flags: u16,
    pub min: [f32; 3],
    pub max: [f32; 3],
}

pub struct Record {
    pub index: usize,
    pub layers: Vec<Layer>,
}

#[derive(Clone, Copy)]
pub struct Area {
    pub rect: [i32; 4],
    pub marked: u32,
    pub cells: u32,
    pub at: usize,
}

#[derive(Clone, Default)]
pub struct Pair {
    pub model: String,
    pub texture: String,
}

pub struct Layout {
    pub header: Header,
    pub records: Vec<Record>,
    pub areas: Vec<Area>,
    pub pairs: Vec<Pair>,
}

pub fn parse(bytes: &[u8]) -> Option<Layout> {
    if bytes.len() < HEAD {
        return None;
    }
    let header = Header {
        records: rdr::u32_at(bytes, 0x40)?,
        layers: rdr::u32_at(bytes, 0x44).unwrap_or(0),
        areas: rdr::u32_at(bytes, 0x48).unwrap_or(0),
        f_records: rdr::u32_at(bytes, 0x58).unwrap_or(0),
        words: rdr::u32_at(bytes, 0x5c).unwrap_or(0),
        pairs: rdr::u32_at(bytes, 0x60).unwrap_or(0),
        samples: rdr::u32_at(bytes, 0x64).unwrap_or(0),
        names: rdr::u32_at(bytes, 0x6c).unwrap_or(0),
        records_end: HEAD,
    };
    let area_count = rdr::u32_at(bytes, 0x54).unwrap_or(0);

    let mut cursor = HEAD;
    let mut records = Vec::with_capacity(header.records as usize);
    for index in 0..header.records as usize {
        let layers = rdr::u16_at(bytes, cursor + 0x20)? as usize;
        let ext = rdr::u8_at(bytes, cursor + 0x22)? as usize;
        let groups = rdr::u8_at(bytes, cursor + 0x23)? as usize;
        let body = cursor + 0x24;
        let end = body + layers * LAYER + ext * 0x90;
        if end > bytes.len() {
            break;
        }
        let mut list = Vec::with_capacity(layers);
        for slot in 0..layers {
            let desc = body + slot * LAYER;
            list.push(Layer {
                pair: bytes.get(desc + 0x39).copied().unwrap_or(0xff) as i8 as i32,
                panel: rdr::u16_at(bytes, desc + 0x3a).unwrap_or(0) & 0x7ff,
                flags: rdr::u16_at(bytes, desc + 0x34).unwrap_or(0),
                min: [
                    rdr::f32_at(bytes, desc).unwrap_or(0.0),
                    rdr::f32_at(bytes, desc + 4).unwrap_or(0.0),
                    rdr::f32_at(bytes, desc + 8).unwrap_or(0.0),
                ],
                max: [
                    rdr::f32_at(bytes, desc + 0x0c).unwrap_or(0.0),
                    rdr::f32_at(bytes, desc + 0x10).unwrap_or(0.0),
                    rdr::f32_at(bytes, desc + 0x14).unwrap_or(0.0),
                ],
            });
        }
        records.push(Record {
            index,
            layers: list,
        });
        cursor = end;
        for _ in 0..groups {
            let count = rdr::u8_at(bytes, cursor + 3)? as usize;
            cursor += 4 + count * 0xF0;
        }
        if cursor > bytes.len() {
            break;
        }
    }
    let mut header = header;
    header.records_end = cursor;

    let mut areas = Vec::with_capacity(area_count as usize);
    for _ in 0..area_count {
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
        let at = cursor + 0x3c;
        let end = at + width * height * 4;
        if end > bytes.len() {
            break;
        }
        let mut marked = 0u32;
        for cell in 0..width * height {
            let word = rdr::u32_at(bytes, at + cell * 4).unwrap_or(0);
            if word >> 16 == 0x6641 {
                marked += 1;
            }
        }
        areas.push(Area {
            rect,
            marked,
            cells: (width * height) as u32,
            at,
        });
        cursor = end;
    }

    let mut pairs = Vec::new();
    let mut tail = cursor + header.f_records as usize * 0x18 + header.words as usize * 4;
    for _ in 0..header.pairs {
        let Some((model, after)) = cstr(bytes, tail) else {
            break;
        };
        let Some((texture, next)) = cstr(bytes, after) else {
            break;
        };
        pairs.push(Pair { model, texture });
        tail = next;
    }

    Some(Layout {
        header,
        records,
        areas,
        pairs,
    })
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

pub struct Panel {
    pub page: u16,
    pub corners: [(f32, f32); 4],
    pub uv: [(f32, f32); 4],
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
            let base = 8 + corner * 0x18;
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
    pub record: usize,
    pub layers: usize,
    pub drawn: usize,
    pub skipped: usize,
    pub scale: f32,
    pub window: [f32; 4],
}

pub fn compose(
    layout: &Layout,
    record: usize,
    load: &dyn Fn(&Pair) -> Option<(Vec<u8>, Vec<u8>)>,
    max_side: u32,
) -> Option<Scene> {
    let rec = layout.records.get(record)?;
    if rec.layers.is_empty() {
        return None;
    }

    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for layer in &rec.layers {
        min_x = min_x.min(layer.min[0]);
        min_y = min_y.min(layer.min[1]);
        max_x = max_x.max(layer.max[0]);
        max_y = max_y.max(layer.max[1]);
    }
    let span_x = (max_x - min_x).max(1.0);
    let span_y = (max_y - min_y).max(1.0);
    let scale = (max_side as f32 / span_x.max(span_y)).min(1.0).max(0.02);

    let width = (span_x * scale).ceil().max(1.0) as u32;
    let height = (span_y * scale).ceil().max(1.0) as u32;
    let mut raster = Raster::new(width, height);

    let mut panel_cache: HashMap<usize, Vec<Panel>> = HashMap::new();
    let mut page_cache: HashMap<usize, Vec<(u32, u32, Vec<u8>)>> = HashMap::new();
    let mut drawn = 0usize;
    let mut skipped = 0usize;

    let mut order: Vec<&Layer> = rec.layers.iter().collect();
    order.sort_by(|left, right| {
        left.min[2]
            .partial_cmp(&right.min[2])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for layer in order {
        if layer.pair < 0 {
            skipped += 1;
            continue;
        }
        let Some(pair) = layout.pairs.get(layer.pair as usize) else {
            skipped += 1;
            continue;
        };
        let key = layer.pair as usize;
        if !panel_cache.contains_key(&key) {
            let list = load(pair)
                .map(|(model, _)| panels(&model))
                .unwrap_or_default();
            panel_cache.insert(key, list);
        }
        let Some(list) = panel_cache.get(&key) else {
            skipped += 1;
            continue;
        };
        let Some(panel) = list.get(layer.panel as usize) else {
            skipped += 1;
            continue;
        };

        if !page_cache.contains_key(&key) {
            let pages = load(pair)
                .and_then(|(_, texture)| decode_pages(&texture))
                .unwrap_or_default();
            page_cache.insert(key, pages);
        }
        let Some(pages) = page_cache.get(&key) else {
            skipped += 1;
            continue;
        };
        let Some((page_w, page_h, rgba)) = pages.get(panel.page as usize) else {
            skipped += 1;
            continue;
        };
        let Some(page) = Page::new(*page_w, *page_h, rgba) else {
            skipped += 1;
            continue;
        };

        let mut low = (f32::MAX, f32::MAX);
        let mut high = (f32::MIN, f32::MIN);
        for corner in &panel.corners {
            low = (low.0.min(corner.0), low.1.min(corner.1));
            high = (high.0.max(corner.0), high.1.max(corner.1));
        }
        let panel_center = ((low.0 + high.0) * 0.5, (low.1 + high.1) * 0.5);
        let target = (
            (layer.min[0] + layer.max[0]) * 0.5,
            (layer.min[1] + layer.max[1]) * 0.5,
        );
        let panel_w = (high.0 - low.0).max(1e-3);
        let panel_h = (high.1 - low.1).max(1e-3);
        let fit_w = (layer.max[0] - layer.min[0]) / panel_w;
        let fit_h = (layer.max[1] - layer.min[1]) / panel_h;
        let fit = fit_w.min(fit_h).clamp(0.02, 4.0);

        let mut dst = [(0.0f32, 0.0f32); 4];
        for corner in 0..4 {
            let world_x = target.0 + (panel.corners[corner].0 - panel_center.0) * fit;
            let world_y = target.1 + (panel.corners[corner].1 - panel_center.1) * fit;
            dst[corner] = ((world_x - min_x) * scale, (max_y - world_y) * scale);
        }
        paint_quad(&mut raster, dst, panel.uv, &page, [255, 255, 255, 255]);
        drawn += 1;
    }

    Some(Scene {
        raster,
        record,
        layers: rec.layers.len(),
        drawn,
        skipped,
        scale,
        window: [min_x, min_y, max_x, max_y],
    })
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

pub fn view(bytes: &[u8], dir: Option<&Path>, max_side: u32) -> Projection {
    let mut out = Projection::new("UFB · field background");
    let Some(layout) = parse(bytes) else {
        out.note = Some("file is shorter than 0x88 bytes".to_string());
        return out;
    };
    let header = layout.header;

    out.note = Some(format!(
        "{} placement records · {} layers · {} area masks · {} resource pairs",
        header.records,
        header.layers,
        layout.areas.len(),
        layout.pairs.len()
    ));

    let mut header_rows = vec![
        row2("placement records", header.records),
        row2("layers", header.layers),
        row2("area masks", layout.areas.len() as u32),
        row2("F records", header.f_records),
        row2("word table", header.words),
        row2("resource pairs", layout.pairs.len() as u32),
        row2("samples", header.samples),
        row2("names", header.names),
        vec![
            "record area".to_string(),
            format!("0x{HEAD:x}..0x{:x}", header.records_end),
        ],
    ];
    if let Some(scene) = dir.and_then(|dir| pick_scene(&layout, dir, max_side)) {
        out.color = Some(ColorMap {
            width: scene.raster.width,
            height: scene.raster.height,
            rgba: scene.raster.rgba.clone(),
            legend: format!(
                "Scene record #{} of {}: {} layers, {} textured panels drawn, {} skipped (3D model layers are not painted), 1 px = {:.2} world units. World window x {}..{}, y {}..{} with y growing upwards. Each panel is a textured quad from the paired .mpd; its atlas page comes from the paired .ftx; the quad is scaled into the layer box from the record and painted back to front by depth.",
                scene.record,
                header.records,
                scene.layers,
                scene.drawn,
                scene.skipped,
                1.0 / scene.scale,
                scene.window[0].round(),
                scene.window[2].round(),
                scene.window[1].round(),
                scene.window[3].round(),
            ),
        });
        header_rows.push(vec![
            "scene record".to_string(),
            format!("#{} ({} layers)", scene.record, scene.layers),
        ]);
    } else if let Some((width, height, rgba, legend)) = area_image(&layout, bytes) {
        out.color = Some(ColorMap {
            width,
            height,
            rgba,
            legend,
        });
    }
    out.sections
        .push(Section::new("Header", &["item", "value"], header_rows, 9));

    let mut ranked: Vec<&Record> = layout
        .records
        .iter()
        .filter(|record| !record.layers.is_empty())
        .collect();
    ranked.sort_by(|left, right| right.layers.len().cmp(&left.layers.len()));
    let record_rows: Vec<Vec<String>> = ranked
        .iter()
        .take(12)
        .map(|record| {
            let mut min_x = f32::MAX;
            let mut max_x = f32::MIN;
            let mut min_y = f32::MAX;
            let mut max_y = f32::MIN;
            for layer in &record.layers {
                min_x = min_x.min(layer.min[0]);
                max_x = max_x.max(layer.max[0]);
                min_y = min_y.min(layer.min[1]);
                max_y = max_y.max(layer.max[1]);
            }
            vec![
                record.index.to_string(),
                record.layers.len().to_string(),
                format!("{}×{}", (max_x - min_x).round(), (max_y - min_y).round()),
                format!("({:.0}, {:.0})..({:.0}, {:.0})", min_x, min_y, max_x, max_y),
            ]
        })
        .collect();
    out.sections.push(Section::new(
        "Scene records (by layer count)",
        &["record", "layers", "size", "bounds"],
        record_rows,
        ranked.len() as u64,
    ));

    let area_rows: Vec<Vec<String>> = layout
        .areas
        .iter()
        .take(MAX_AREAS)
        .enumerate()
        .map(|(index, area)| {
            vec![
                index.to_string(),
                format!(
                    "({}, {})..({}, {})",
                    area.rect[0], area.rect[1], area.rect[2], area.rect[3]
                ),
                format!(
                    "{}×{}",
                    area.rect[2] - area.rect[0] + 1,
                    area.rect[3] - area.rect[1] + 1
                ),
                format!(
                    "{:.1}%",
                    100.0 * area.marked as f64 / area.cells.max(1) as f64
                ),
                format!("{:#x}", area.at),
            ]
        })
        .collect();
    out.sections.push(Section::new(
        "Area masks (first 24)",
        &["#", "rect", "size", "marked", "data"],
        area_rows,
        layout.areas.len() as u64,
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

    out
}

fn pick_scene<'a>(layout: &Layout, dir: &Path, max_side: u32) -> Option<Scene> {
    let mut order: Vec<usize> = layout
        .records
        .iter()
        .filter(|record| !record.layers.is_empty())
        .map(|record| record.index)
        .collect();
    order.sort_by_key(|index| std::cmp::Reverse(layout.records[*index].layers.len()));
    let load = |pair: &Pair| read_pair(dir, pair);
    let mut best: Option<Scene> = None;
    for index in order.into_iter().take(6) {
        let Some(scene) = compose(layout, index, &load, max_side) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|current| scene.drawn > current.drawn)
        {
            best = Some(scene);
        }
    }
    best.filter(|scene| scene.drawn > 0)
}

fn area_image(layout: &Layout, bytes: &[u8]) -> Option<(u32, u32, Vec<u8>, String)> {
    let index = layout
        .areas
        .iter()
        .enumerate()
        .filter(|(_, area)| {
            let ratio = area.marked as f64 / area.cells.max(1) as f64;
            (0.2..=0.6).contains(&ratio)
        })
        .max_by_key(|(_, area)| area.cells)
        .or_else(|| {
            layout
                .areas
                .iter()
                .enumerate()
                .max_by_key(|(_, area)| area.marked)
        })
        .map(|(index, _)| index)?;
    let area = layout.areas[index];
    let width = (area.rect[2] - area.rect[0] + 1).max(1) as usize;
    let height = (area.rect[3] - area.rect[1] + 1).max(1) as usize;
    let step = width.max(height).div_ceil(800).max(1);
    let out_width = width.div_ceil(step) as u32;
    let out_height = height.div_ceil(step) as u32;
    let mut raster = Raster::filled(out_width, out_height, [10, 12, 18, 255]);
    let hue = super::content::area_hue(index as u32);
    for row in 0..height {
        let py = (row / step) as u32;
        for column in 0..width {
            let px = (column / step) as u32;
            let word = rdr::u32_at(bytes, area.at + (row * width + column) * 4).unwrap_or(0);
            let color = if word >> 16 == 0x6641 {
                super::content::hsv(hue, 0.55, 0.98)
            } else {
                super::content::hsv(hue, 0.55, 0.16)
            };
            raster.blend(px, py, [color[0], color[1], color[2], 255]);
        }
    }
    Some((
        out_width,
        out_height,
        raster.rgba,
        format!(
            "Area mask #{index} of {}: rectangle ({}, {})..({}, {}). Bright cells carry 0x6641 (inside the area), dark cells 0x6640. The paired .ftx and .mpd files hold the pixels; open the folder next to Field.ufb to render the textured scene.",
            layout.areas.len(),
            area.rect[0],
            area.rect[1],
            area.rect[2],
            area.rect[3]
        ),
    ))
}

fn row2(key: &str, value: u32) -> Vec<String> {
    vec![key.to_string(), value.to_string()]
}
