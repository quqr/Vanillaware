//! Corpus probe: is the layer extent carried by `s7.scale`, or by `s2`?
//!
//! The geometry item the engine builds seeds its 4x4 from a shared default block
//! (`*(*(0x7003acd6e8)+0x40)+0x280`), and `s7.scale` turns out to be a *relative*
//! scale (median exactly 1.0). A unit quad times that scale would draw every
//! layer one world unit across, which the corpus refutes: inside a single
//! keyframe the `s2` widths span two orders of magnitude. So the per-layer
//! extent has to come from `s2`, the only per-layer geometry in the file.
//!
//! ```text
//! $env:MBS_DIR="F:/Codes/UNICORN OVERLORD/extract/000"
//! cargo test -p formats --test mbs_quad_extent -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};

use formats::mbs;
use formats::mbs::quad::Pad;

const TAG: &str = "swi_unic";

fn collect(dir: &Path, out: &mut Vec<PathBuf>, budget: usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= budget {
            return;
        }
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out, budget);
        } else if path.extension().is_some_and(|ext| ext == "mbs") {
            out.push(path);
        }
    }
}

fn extent(quad: &[f32]) -> Option<(f32, f32)> {
    let corners = if quad.len() >= 12 {
        &quad[2..10]
    } else if quad.len() >= 8 {
        &quad[0..8]
    } else {
        return None;
    };
    let mut low = [f32::MAX, f32::MAX];
    let mut high = [f32::MIN, f32::MIN];
    for corner in corners.chunks_exact(2) {
        low[0] = low[0].min(corner[0]);
        low[1] = low[1].min(corner[1]);
        high[0] = high[0].max(corner[0]);
        high[1] = high[1].max(corner[1]);
    }
    Some((high[0] - low[0], high[1] - low[1]))
}

#[test]
#[ignore]
fn s2_carries_the_layer_extent() {
    let dir = std::env::var("MBS_DIR")
        .unwrap_or_else(|_| "F:/Codes/UNICORN OVERLORD/extract/000".to_string());
    let dir = PathBuf::from(dir);
    if !dir.is_dir() {
        eprintln!("skip: {} is not a directory", dir.display());
        return;
    }

    let mut files = Vec::new();
    collect(&dir, &mut files, 40);
    files.sort();

    let (mut frames, mut scaled, mut uniform_frames, mut spread_frames) =
        (0usize, 0usize, 0usize, 0usize);
    let mut widest = 1.0f32;
    let mut widest_at = String::new();
    let mut unit_scale_layers = 0usize;
    let mut layers = 0usize;
    let mut scale_samples: Vec<f32> = Vec::new();

    for path in &files {
        let Ok(v55) = mbs::parse_file(path, TAG) else {
            continue;
        };
        let Ok(quad) = mbs::convert(&v55) else {
            continue;
        };
        for animation in &quad.animation {
            let Pad::Filled(animation) = animation else {
                continue;
            };
            for entry in &animation.timeline {
                let Some(attach) = entry.attach.as_ref() else {
                    continue;
                };
                if attach.kind != "keyframe" || attach.id < 0 {
                    continue;
                }
                let Some(Pad::Filled(keyframe)) = quad.keyframe.get(attach.id as usize) else {
                    continue;
                };
                frames += 1;
                let scale = entry.transform.scale;
                scale_samples.push(scale[0]);
                scale_samples.push(scale[1]);
                if (scale[0] - 1.0).abs() < 1e-3 && (scale[1] - 1.0).abs() < 1e-3 {
                    unit_scale_layers += keyframe.layer.len();
                } else {
                    scaled += 1;
                }
                let mut low = f32::MAX;
                let mut high = 0.0f32;
                for layer in &keyframe.layer {
                    let Pad::Filled(layer) = layer else {
                        continue;
                    };
                    layers += 1;
                    let Some((width, _)) = extent(&layer.dstquad) else {
                        continue;
                    };
                    if width <= 0.0 {
                        continue;
                    }
                    low = low.min(width);
                    high = high.max(width);
                }
                if low.is_finite() && high > 0.0 {
                    let spread = high / low;
                    if spread < 1.05 {
                        uniform_frames += 1;
                    } else {
                        spread_frames += 1;
                    }
                    if spread > widest {
                        widest = spread;
                        widest_at = format!(
                            "{} {} scale=({:.3},{:.3}) width {:.1}..{:.1}",
                            path.file_name().unwrap_or_default().to_string_lossy(),
                            animation.name,
                            scale[0],
                            scale[1],
                            low,
                            high
                        );
                    }
                }
            }
        }
    }

    println!("files {}", files.len());
    println!("frames with a keyframe attachment {}", frames);
    println!("layers {}", layers);
    scale_samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if !scale_samples.is_empty() {
        let at = |q: f32| scale_samples[((scale_samples.len() - 1) as f32 * q) as usize];
        println!(
            "s7 scale percentiles: p01 {:.3} p25 {:.3} p50 {:.3} p75 {:.3} p99 {:.3} max {:.3}",
            at(0.01),
            at(0.25),
            at(0.50),
            at(0.75),
            at(0.99),
            scale_samples[scale_samples.len() - 1]
        );
    }
    println!(
        "frames whose s7 scale is 1.0: {} ({} layers), non-unit: {}",
        frames - scaled,
        unit_scale_layers,
        scaled
    );
    println!("keyframes with one layer width: {uniform_frames}, with a spread: {spread_frames}");
    println!("widest width spread inside one keyframe: {widest:.1}x at {widest_at}");

    assert!(frames > 0, "no frames were parsed");
    assert!(
        (frames - scaled) * 2 > frames,
        "s7 scale is a relative scale, so most frames must sit at 1.0"
    );
    assert!(
        widest > 4.0,
        "a unit quad times s7 cannot span {widest}x inside one keyframe"
    );
}
