use std::path::PathBuf;

use formats::atlas::Atlas;
use formats::mbs::quad::{Keyframe, KeyframeLayer, Pad};
use image::{ImageFormat, RgbaImage};

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        return Err("usage: quad_probe <scene.mbs> <tag> <out.png> [keyframe] [scale]".into());
    }
    let mbs = PathBuf::from(&args[1]);
    let tag = &args[2];
    let out = PathBuf::from(&args[3]);
    let keyframe = args.get(4).and_then(|value| value.parse::<usize>().ok());
    let scale = args
        .get(5)
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.25);

    let v55 = formats::mbs::parse_file(&mbs, tag).map_err(|error| format!("{error}"))?;
    let quad = formats::mbs::convert(&v55).map_err(|error| format!("{error}"))?;
    println!(
        "keyframes {} hitboxes {}",
        quad.keyframe.len(),
        quad.hitbox.len()
    );

    let atlas = Atlas::open(&mbs)
        .map_err(|error| format!("{error}"))?
        .ok_or_else(|| "no sibling ftx".to_string())?;
    let pages: Vec<(u32, u32, Vec<u8>)> = (0..atlas.len())
        .map(|index| {
            let texture = atlas
                .decode(index as i64)
                .map_err(|error| format!("{error}"))?;
            println!(
                "page {index}: {} {}x{} {}",
                texture.name, texture.width, texture.height, texture.format
            );
            Ok((texture.width, texture.height, texture.rgba()))
        })
        .collect::<Result<_, String>>()?;

    let score = |cell: &Pad<Keyframe>| match cell {
        Pad::Filled(frame) => frame
            .layer
            .iter()
            .filter_map(|cell| match cell {
                Pad::Filled(layer) => Some(layer),
                Pad::Empty(_) => None,
            })
            .map(|layer| {
                let mut low = (f32::MAX, f32::MAX);
                let mut high = (f32::MIN, f32::MIN);
                for point in layer.dstquad.chunks_exact(2) {
                    low = (low.0.min(point[0]), low.1.min(point[1]));
                    high = (high.0.max(point[0]), high.1.max(point[1]));
                }
                (high.0 - low.0).max(0.0) * (high.1 - low.1).max(0.0)
            })
            .sum::<f32>(),
        Pad::Empty(_) => 0.0,
    };

    for (index, cell) in quad.keyframe.iter().enumerate() {
        if let Pad::Filled(frame) = cell {
            let area = score(cell);
            println!(
                "keyframe {index}: {} layers, covered {:.0} units^2",
                frame.layer.len(),
                area
            );
        }
    }

    let index = keyframe.unwrap_or_else(|| {
        let deepest = quad
            .keyframe
            .iter()
            .filter_map(|cell| match cell {
                Pad::Filled(frame) => Some(frame.layer.len()),
                Pad::Empty(_) => None,
            })
            .max()
            .unwrap_or(0);
        quad.keyframe
            .iter()
            .position(|cell| matches!(cell, Pad::Filled(frame) if frame.layer.len() == deepest))
            .unwrap_or(0)
    });
    let Some(Pad::Filled(frame)) = quad.keyframe.get(index) else {
        return Err(format!("keyframe {index} is missing"));
    };
    println!(
        "using keyframe {index} ({}) with {} layers",
        frame.name,
        frame.layer.len()
    );

    if std::env::var_os("QUAD_DUMP").is_some() {
        for (frame_index, cell) in quad.keyframe.iter().enumerate() {
            let Pad::Filled(frame) = cell else { continue };
            for (slot, cell) in frame.layer.iter().enumerate() {
                let Pad::Filled(layer) = cell else { continue };
                let src = layer.srcquad.clone().unwrap_or_default();
                let fog = layer
                    .fogquad
                    .as_ref()
                    .map(|fog| match fog {
                        formats::mbs::v55::FogQuad::One(text) => text.clone(),
                        formats::mbs::v55::FogQuad::Four(list) => list[0].clone(),
                    })
                    .unwrap_or_else(|| "none".to_string());
                println!(
                    "DUMP {frame_index} {slot} {:?} dst={:?} src={:?} fog={fog}",
                    layer.tex_id, layer.dstquad, src
                );
            }
        }
    }

    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for cell in &frame.layer {
        let Pad::Filled(layer) = cell else { continue };
        for point in layer.dstquad.chunks_exact(2) {
            min_x = min_x.min(point[0]);
            min_y = min_y.min(point[1]);
            max_x = max_x.max(point[0]);
            max_y = max_y.max(point[1]);
        }
    }
    let width = ((max_x - min_x) * scale).ceil() as u32;
    let height = ((max_y - min_y) * scale).ceil() as u32;
    println!("canvas {width}x{height} from {min_x}..{max_x} / {min_y}..{max_y}");

    let mut canvas = RgbaImage::new(width, height);
    for cell in &frame.layer {
        let Pad::Filled(layer) = cell else { continue };
        draw(&mut canvas, &pages, layer, min_x, min_y, scale);
    }

    let file = std::fs::File::create(&out).map_err(|error| error.to_string())?;
    let mut writer = std::io::BufWriter::new(file);
    canvas
        .write_to(&mut writer, ImageFormat::Png)
        .map_err(|error| error.to_string())?;
    println!("wrote {}", out.display());
    Ok(())
}

fn draw(
    canvas: &mut RgbaImage,
    pages: &[(u32, u32, Vec<u8>)],
    layer: &KeyframeLayer,
    origin_x: f32,
    origin_y: f32,
    scale: f32,
) {
    let Some(tex_id) = layer.tex_id else { return };
    if tex_id < 0 {
        return;
    }
    let Some((page_w, page_h, rgba)) = pages.get(tex_id as usize) else {
        return;
    };
    if layer.dstquad.len() < 8 {
        return;
    }
    let src = layer
        .srcquad
        .clone()
        .unwrap_or_else(|| vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0]);
    if src.len() < 8 {
        return;
    }

    let mut dst = [(0.0f32, 0.0f32); 4];
    let mut uv = [(0.0f32, 0.0f32); 4];
    for corner in 0..4 {
        dst[corner] = (
            (layer.dstquad[corner * 2] - origin_x) * scale,
            (layer.dstquad[corner * 2 + 1] - origin_y) * scale,
        );
        uv[corner] = (src[corner * 2], src[corner * 2 + 1]);
    }

    let tint = layer
        .fogquad
        .as_ref()
        .map(|fog| match fog {
            formats::mbs::v55::FogQuad::One(text) => parse_color(text),
            formats::mbs::v55::FogQuad::Four(list) => parse_color(&list[0]),
        })
        .unwrap_or([255, 255, 255, 255]);

    for triangle in [[0usize, 1, 2], [0, 2, 3]] {
        let (p0, p1, p2) = (dst[triangle[0]], dst[triangle[1]], dst[triangle[2]]);
        let area = (p1.0 - p0.0) * (p2.1 - p0.1) - (p1.1 - p0.1) * (p2.0 - p0.0);
        if area.abs() < 1e-6 {
            continue;
        }
        let min_px = p0.0.min(p1.0).min(p2.0).floor().max(0.0) as u32;
        let min_py = p0.1.min(p1.1).min(p2.1).floor().max(0.0) as u32;
        let max_px = (p0.0.max(p1.0).max(p2.0).ceil() as i64).max(0) as u32;
        let max_py = (p0.1.max(p1.1).max(p2.1).ceil() as i64).max(0) as u32;
        let max_px = max_px.min(canvas.width());
        let max_py = max_py.min(canvas.height());

        for py in min_py..max_py {
            for px in min_px..max_px {
                let point = (px as f32 + 0.5, py as f32 + 0.5);
                let w0 = ((p1.0 - point.0) * (p2.1 - point.1)
                    - (p1.1 - point.1) * (p2.0 - point.0))
                    / area;
                let w1 = ((p2.0 - point.0) * (p0.1 - point.1)
                    - (p2.1 - point.1) * (p0.0 - point.0))
                    / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < -0.001 || w1 < -0.001 || w2 < -0.001 {
                    continue;
                }
                let u = w0 * uv[triangle[0]].0 + w1 * uv[triangle[1]].0 + w2 * uv[triangle[2]].0;
                let v = w0 * uv[triangle[0]].1 + w1 * uv[triangle[1]].1 + w2 * uv[triangle[2]].1;
                let texel = sample(rgba, *page_w, *page_h, u, v);
                if texel[3] == 0 {
                    continue;
                }
                let src_px = [
                    (texel[0] as u32 * tint[0] as u32 / 255) as u8,
                    (texel[1] as u32 * tint[1] as u32 / 255) as u8,
                    (texel[2] as u32 * tint[2] as u32 / 255) as u8,
                    (texel[3] as u32 * tint[3] as u32 / 255) as u8,
                ];
                blend(canvas, px, py, src_px);
            }
        }
    }
}

fn parse_color(text: &str) -> [u8; 4] {
    let digits: String = text
        .trim_start_matches('#')
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect();
    if digits.len() < 8 {
        return [255, 255, 255, 255];
    }
    let value = u32::from_str_radix(&digits[..8], 16).unwrap_or(0xffff_ffff);
    [
        (value >> 24) as u8,
        (value >> 16) as u8,
        (value >> 8) as u8,
        value as u8,
    ]
}

fn sample(rgba: &[u8], width: u32, height: u32, u: f32, v: f32) -> [u8; 4] {
    if width == 0 || height == 0 {
        return [0, 0, 0, 0];
    }
    let fx = (u * width as f32 - 0.5).clamp(0.0, width as f32 - 1.0);
    let fy = (v * height as f32 - 0.5).clamp(0.0, height as f32 - 1.0);
    let x0 = fx.floor() as u32;
    let y0 = fy.floor() as u32;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;
    let mut out = [0u8; 4];
    for channel in 0..4 {
        let at = |x: u32, y: u32| rgba[((y * width + x) * 4 + channel as u32) as usize] as f32;
        let top = at(x0, y0) * (1.0 - tx) + at(x1, y0) * tx;
        let bottom = at(x0, y1) * (1.0 - tx) + at(x1, y1) * tx;
        out[channel] = (top * (1.0 - ty) + bottom * ty).round().clamp(0.0, 255.0) as u8;
    }
    out
}

fn blend(canvas: &mut RgbaImage, x: u32, y: u32, src: [u8; 4]) {
    let pixel = canvas.get_pixel_mut(x, y);
    let alpha = src[3] as f32 / 255.0;
    if alpha >= 1.0 {
        *pixel = image::Rgba(src);
        return;
    }
    let dst = pixel.0;
    let out = [
        (src[0] as f32 * alpha + dst[0] as f32 * (1.0 - alpha)) as u8,
        (src[1] as f32 * alpha + dst[1] as f32 * (1.0 - alpha)) as u8,
        (src[2] as f32 * alpha + dst[2] as f32 * (1.0 - alpha)) as u8,
        ((src[3] as f32 + dst[3] as f32 * (1.0 - alpha)).min(255.0)) as u8,
    ];
    *pixel = image::Rgba(out);
}
