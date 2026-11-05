//! Playback of a field window at the engine clock.
//!
//! Panels never move, so a window is composited once and only the props whose
//! pose changes with the clock are redrawn per frame. Between two moving props
//! the composite is captured as a band the size of the prop that follows it; a
//! frame paints the base, then for every moving prop draws the prop and restores
//! the band in front of it. The result is pixel-identical to
//! `compose_window_with` at that clock, because the painter's order is preserved
//! and every band holds exactly the layers that sit between two moving props.

use std::collections::HashMap;
use std::sync::Arc;

use super::{
    decode_pages, fit, gather, mbs, paint_terrain, panels, piece_corners, prop_at, quad_corners,
    shade, tint, Canvas, Layer, Layout, PairLoader, Panel, Render, Scene, MBS_TAG,
};
use crate::raster::{paint_quad, Page, Raster};

#[derive(Clone)]
struct PanelItem {
    layer: Layer,
    panel: Panel,
    slot: usize,
}

#[derive(Clone)]
struct PropItem {
    layer: Layer,
    quad: Arc<mbs::Quad>,
    index: usize,
    slot: usize,
}

enum Item {
    Panel(PanelItem),
    Prop(PropItem),
    Moving(PropItem),
    Dead,
}

struct Band {
    /// Props drawn at the far side of this band, in the order they are painted.
    props: Vec<PropItem>,
    /// What sits in front of them, cropped to the pixels it actually covers.
    image: Raster,
    left: u32,
    top: u32,
}

/// A prepared window: everything that does not depend on the clock plus the
/// bands between the props that do.
pub struct Playback {
    pub map: String,
    pub window: [f32; 4],
    pub scale: f32,
    pub tiles: usize,
    pub layers: usize,
    pub moving: usize,
    base: Raster,
    canvas: Canvas,
    pages: HashMap<usize, Vec<(u32, u32, Vec<u8>)>>,
    bands: Vec<Band>,
    drawn: usize,
    models: usize,
    quads: usize,
    hollow: usize,
    missing: usize,
}

impl Playback {
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        bytes: &[u8],
        layout: &Layout,
        map: &str,
        x0: f32,
        width: f32,
        load: &PairLoader<'_>,
        max_side: u32,
        render: Render,
    ) -> Option<Self> {
        let width = width.max(1.0);
        let x1 = x0 + width;
        let layers = gather(layout, map, x0, x1);
        if layers.is_empty() {
            return None;
        }
        let (scale, min_y, max_y) = fit(&layers, width, max_side);
        let out_width = (width * scale).ceil().max(1.0) as u32;
        let out_height = ((max_y - min_y).max(1.0) * scale).ceil().max(1.0) as u32;
        let canvas = Canvas {
            left: x0,
            top: max_y,
            scale,
        };

        let mut pages: HashMap<usize, Vec<(u32, u32, Vec<u8>)>> = HashMap::new();
        let mut panel_cache: HashMap<usize, Vec<Panel>> = HashMap::new();
        let mut quad_cache: HashMap<usize, Option<Arc<mbs::Quad>>> = HashMap::new();
        let mut items: Vec<Item> = Vec::new();
        let (mut drawn, mut models, mut quads, mut hollow, mut missing) = (0, 0, 0, 0, 0);

        let mut order = layers;
        order.sort_by(|left, right| {
            right.min[2]
                .partial_cmp(&left.min[2])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for layer in order {
            let Some(pair) = layout.pairs.get(layer.pair as usize) else {
                missing += 1;
                items.push(Item::Dead);
                continue;
            };
            let slot = layer.pair as usize;
            if !pair.is_panel() || layer.flags & 4 == 0 {
                let quad = quad_cache.entry(slot).or_insert_with(|| {
                    load(pair)
                        .and_then(|(model, _)| mbs::parse_bytes(&model, MBS_TAG).ok())
                        .and_then(|v55| mbs::convert(&v55).ok())
                        .map(Arc::new)
                });
                let Some(quad) = quad.clone() else {
                    missing += 1;
                    items.push(Item::Dead);
                    continue;
                };
                let rest = prop_at(&quad, layer.panel as usize, 0);
                if rest.is_empty() {
                    hollow += 1;
                    items.push(Item::Dead);
                    continue;
                }
                let moving = [15_000u64, 120_000, 900_000]
                    .iter()
                    .any(|clock| !same(&rest, &prop_at(&quad, layer.panel as usize, *clock)));
                pages.entry(slot).or_insert_with(|| {
                    load(pair)
                        .and_then(|(_, texture)| decode_pages(&texture))
                        .unwrap_or_default()
                });
                models += 1;
                quads += rest.len();
                let item = PropItem {
                    layer: *layer,
                    quad,
                    index: layer.panel as usize,
                    slot,
                };
                items.push(if moving {
                    Item::Moving(item)
                } else {
                    Item::Prop(item)
                });
                continue;
            }

            let list = panel_cache.entry(slot).or_insert_with(|| {
                load(pair)
                    .map(|(model, _)| panels(&model))
                    .unwrap_or_default()
            });
            let Some(panel) = list.get(layer.panel as usize).copied() else {
                missing += 1;
                items.push(Item::Dead);
                continue;
            };
            pages.entry(slot).or_insert_with(|| {
                load(pair)
                    .and_then(|(_, texture)| decode_pages(&texture))
                    .unwrap_or_default()
            });
            drawn += 1;
            items.push(Item::Panel(PanelItem {
                layer: *layer,
                panel,
                slot,
            }));
        }

        let mut base = if render.terrain {
            let mut base = Raster::new(out_width, out_height);
            let painted =
                paint_terrain(bytes, layout, &mut base, load, &mut pages, &canvas, x0, x1);
            (base, painted)
        } else {
            (Raster::new(out_width, out_height), 0)
        };
        let terrain_tiles = base.1;
        let base = &mut base.0;

        let mut index = 0usize;
        while index < items.len() {
            if matches!(items[index], Item::Moving(_)) {
                break;
            }
            draw(&mut *base, &canvas, &pages, &items[index], 0);
            index += 1;
        }

        let mut bands: Vec<Band> = Vec::new();
        let mut props: Vec<PropItem> = Vec::new();
        let mut image = Raster::new(out_width, out_height);
        let mut area: Option<(f32, f32, f32, f32)> = None;
        while index < items.len() {
            if let Item::Moving(prop) = &items[index] {
                let mut blank = Raster::new(out_width, out_height);
                std::mem::swap(&mut blank, &mut image);
                let (crop, left, top) = crop(blank, area.take(), out_width, out_height);
                if crop.width > 0 {
                    bands.push(Band {
                        props: std::mem::take(&mut props),
                        image: crop,
                        left,
                        top,
                    });
                }
                props.push(prop.clone());
            } else {
                area = union(area, draw(&mut image, &canvas, &pages, &items[index], 0));
            }
            index += 1;
        }
        let (crop, left, top) = crop(image, area, out_width, out_height);
        if crop.width > 0 || !props.is_empty() {
            bands.push(Band {
                props,
                image: crop,
                left,
                top,
            });
        }

        let raster = std::mem::replace(base, Raster::new(0, 0));
        Some(Self {
            map: map.to_string(),
            window: [x0, min_y, x1, max_y],
            scale,
            tiles: terrain_tiles,
            layers: drawn + models + hollow + missing,
            moving: bands.len(),
            base: raster,
            canvas,
            pages,
            bands,
            drawn,
            models,
            quads,
            hollow,
            missing,
        })
    }

    /// One frame at the engine clock, indistinguishable from a full render.
    pub fn frame(&self, clock: u64) -> Scene {
        let mut raster = self.base.clone();
        for band in &self.bands {
            for prop in &band.props {
                draw(
                    &mut raster,
                    &self.canvas,
                    &self.pages,
                    &Item::Prop(prop.clone()),
                    clock,
                );
            }
            if band.image.width > 0 {
                raster.over_at(&band.image, band.left, band.top);
            }
        }
        Scene {
            raster,
            map: self.map.clone(),
            layers: self.layers,
            drawn: self.drawn,
            models: self.models,
            quads: self.quads,
            hollow: self.hollow,
            missing: self.missing,
            tiles: self.tiles,
            clock,
            scale: self.scale,
            window: self.window,
        }
    }

    /// Bytes the prepared window holds, for the viewer to report.
    pub fn footprint(&self) -> usize {
        let mut total = self.base.rgba.len();
        for band in &self.bands {
            total += band.image.rgba.len();
        }
        total
    }
}

fn same(left: &[super::Piece], right: &[super::Piece]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right.iter()).all(|(a, b)| {
            a.page == b.page
                && a.dst
                    .iter()
                    .zip(b.dst.iter())
                    .all(|(a, b)| (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3)
        })
}

/// Paint one item and report the pixel box it touched, so a band only has to be
/// cropped to what was actually drawn instead of scanned for coverage.
fn draw(
    raster: &mut Raster,
    canvas: &Canvas,
    pages: &HashMap<usize, Vec<(u32, u32, Vec<u8>)>>,
    item: &Item,
    clock: u64,
) -> Option<(f32, f32, f32, f32)> {
    match item {
        Item::Panel(panel) => {
            let Some((width, height, rgba)) = pages
                .get(&panel.slot)
                .and_then(|list| list.get(panel.panel.page as usize))
            else {
                return None;
            };
            let Some(page) = Page::new(*width, *height, rgba) else {
                return None;
            };
            let quad = quad_corners(&panel.layer, &panel.panel);
            let mut dst = [(0.0f32, 0.0f32); 4];
            for corner in 0..4 {
                dst[corner] = canvas.pixel(
                    panel.layer.center_x + quad[corner].0,
                    panel.layer.center_y + quad[corner].1,
                );
            }
            paint_quad(raster, dst, panel.panel.uv, &page, tint(panel.layer.color));
            Some(bounds(dst))
        }
        Item::Prop(prop) | Item::Moving(prop) => {
            let Some(list) = pages.get(&prop.slot) else {
                return None;
            };
            let mut boxed: Option<(f32, f32, f32, f32)> = None;
            for piece in prop_at(&prop.quad, prop.index, clock) {
                let Some((width, height, rgba)) = list.get(piece.page) else {
                    continue;
                };
                let Some(page) = Page::new(*width, *height, rgba) else {
                    continue;
                };
                let corners = piece_corners(&prop.layer, piece.dst);
                let mut dst = [(0.0f32, 0.0f32); 4];
                for corner in 0..4 {
                    dst[corner] = canvas.pixel(
                        prop.layer.center_x + corners[corner].0,
                        prop.layer.center_y + corners[corner].1,
                    );
                }
                paint_quad(
                    raster,
                    dst,
                    piece.uv,
                    &page,
                    shade(prop.layer.color, piece.color),
                );
                boxed = Some(match boxed {
                    Some((lx, ly, hx, hy)) => {
                        let (nx, ny, xx, xy) = bounds(dst);
                        (lx.min(nx), ly.min(ny), hx.max(xx), hy.max(xy))
                    }
                    None => bounds(dst),
                });
            }
            boxed
        }
        Item::Dead => None,
    }
}

fn bounds(dst: [(f32, f32); 4]) -> (f32, f32, f32, f32) {
    let mut boxed = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for point in dst {
        boxed.0 = boxed.0.min(point.0);
        boxed.1 = boxed.1.min(point.1);
        boxed.2 = boxed.2.max(point.0);
        boxed.3 = boxed.3.max(point.1);
    }
    boxed
}

fn union(
    left: Option<(f32, f32, f32, f32)>,
    right: Option<(f32, f32, f32, f32)>,
) -> Option<(f32, f32, f32, f32)> {
    match (left, right) {
        (Some(a), Some(b)) => Some((a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3))),
        (Some(a), None) => Some(a),
        (None, other) => other,
    }
}

/// Copy the part of a band that was actually painted, so compositing skips a
/// window of transparent black.
fn crop(
    image: Raster,
    area: Option<(f32, f32, f32, f32)>,
    width: u32,
    height: u32,
) -> (Raster, u32, u32) {
    let Some((low_x, low_y, high_x, high_y)) = area else {
        return (Raster::new(0, 0), 0, 0);
    };
    let left = (low_x.floor() - 1.0).clamp(0.0, width as f32) as u32;
    let top = (low_y.floor() - 1.0).clamp(0.0, height as f32) as u32;
    let right = (high_x.ceil() + 1.0).clamp(0.0, width as f32) as u32;
    let bottom = (high_y.ceil() + 1.0).clamp(0.0, height as f32) as u32;
    if right <= left || bottom <= top {
        return (Raster::new(0, 0), 0, 0);
    }
    let (crop_width, crop_height) = (right - left, bottom - top);
    let mut out = Raster::new(crop_width, crop_height);
    for y in 0..crop_height {
        let from = (((top + y) * width + left) * 4) as usize;
        let to = (y * crop_width * 4) as usize;
        out.rgba[to..to + crop_width as usize * 4]
            .copy_from_slice(&image.rgba[from..from + crop_width as usize * 4]);
    }
    (out, left, top)
}
