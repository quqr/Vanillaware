//! Corpus probe for the frame flag that removes a layer for one frame.
//!
//! `f_700058d188` returns right before it allocates the sprite when the current
//! frame's flag word has bit `0x400` set, so the track contributes nothing that
//! frame. This probe checks that such frames really carry a keyframe with
//! layers, and that the same keyframe is drawn on neighbouring frames — i.e.
//! that the bit is a visibility switch and not a "no data" marker.
//!
//! ```text
//! $env:MBS_DIR="F:/Codes/UNICORN OVERLORD/extract/000"
//! cargo test -p formats --test mbs_frame_flags -- --ignored --nocapture
//! ```

use std::collections::HashMap;
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

fn flags_of(text: &str) -> u32 {
    u32::from_str_radix(text.trim_start_matches("0x"), 16).unwrap_or(0)
}

/// `f_700058d188` mirrors a column of the local matrix when `s4.attrib`
/// (u32 at `+0x08`) has bit 0 / bit 1 set. This probe measures both candidate
/// sources so the preview can pick the one the data supports.
///
/// ```text
/// $env:MBS_DIR="F:/Codes/UNICORN OVERLORD/extract/000"
/// cargo test -p formats --test mbs_frame_flags -- --ignored --nocapture
/// ```
#[test]
#[ignore]
fn mirror_bits_live_on_the_layer() {
    let dir = std::env::var("MBS_DIR")
        .unwrap_or_else(|_| "F:/Codes/UNICORN OVERLORD/extract/000".to_string());
    let dir = PathBuf::from(dir);
    if !dir.is_dir() {
        eprintln!("skip: {} is not a directory", dir.display());
        return;
    }

    let mut files = Vec::new();
    collect(&dir, &mut files, 400);
    files.sort();

    let (mut frames, mut frame_bit0, mut frame_bit1, mut frame_other) =
        (0usize, 0usize, 0usize, 0usize);
    let (mut layers, mut layer_attr, mut layer_bit0, mut layer_bit1) =
        (0usize, 0usize, 0usize, 0usize);
    let mut histogram: HashMap<u32, usize> = HashMap::new();

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
                frames += 1;
                let flags = flags_of(&entry.debug);
                if flags & 0x1 != 0 {
                    frame_bit0 += 1;
                }
                if flags & 0x2 != 0 {
                    frame_bit1 += 1;
                }
                if flags & !0x3 != 0 {
                    frame_other += 1;
                }
            }
        }
        for keyframe in &quad.keyframe {
            let Pad::Filled(keyframe) = keyframe else {
                continue;
            };
            for cell in &keyframe.layer {
                let Pad::Filled(layer) = cell else {
                    continue;
                };
                layers += 1;
                let attr = flags_of(&layer.debug.3);
                *histogram.entry(attr).or_insert(0) += 1;
                if attr != 0 {
                    layer_attr += 1;
                }
                if attr & 0x1 != 0 {
                    layer_bit0 += 1;
                }
                if attr & 0x2 != 0 {
                    layer_bit1 += 1;
                }
            }
        }
    }

    println!("files {}", files.len());
    println!("frames {frames}: bit0 {frame_bit0}, bit1 {frame_bit1}, other bits {frame_other}");
    println!("layers {layers}: attrib set {layer_attr}, bit0 {layer_bit0}, bit1 {layer_bit1}");
    let mut top: Vec<_> = histogram.into_iter().collect();
    top.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    println!("attrib values (top 8):");
    for (value, count) in top.into_iter().take(8) {
        println!("  {value:#x}: {count}");
    }
}

#[test]
#[ignore]
fn hidden_frames_still_carry_layers() {
    let dir = std::env::var("MBS_DIR")
        .unwrap_or_else(|_| "F:/Codes/UNICORN OVERLORD/extract/000".to_string());
    let dir = PathBuf::from(dir);
    if !dir.is_dir() {
        eprintln!("skip: {} is not a directory", dir.display());
        return;
    }

    let mut files = Vec::new();
    collect(&dir, &mut files, 400);
    files.sort();

    let (mut frames, mut hidden, mut hidden_with_layers, mut hidden_without_layers) =
        (0usize, 0usize, 0usize, 0usize);
    let mut flipped: HashMap<(String, i64), (usize, usize)> = HashMap::new();

    for path in &files {
        let Ok(v55) = mbs::parse_file(path, TAG) else {
            continue;
        };
        let Ok(quad) = mbs::convert(&v55) else {
            continue;
        };
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
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
                let hidden_now = flags_of(&entry.debug) & 0x400 != 0;
                let layers = keyframe.layer.len();
                if hidden_now {
                    hidden += 1;
                    if layers == 0 {
                        hidden_without_layers += 1;
                    } else {
                        hidden_with_layers += 1;
                    }
                }
                let slot = flipped.entry((name.clone(), attach.id)).or_insert((0, 0));
                if hidden_now {
                    slot.1 += 1;
                } else {
                    slot.0 += 1;
                }
            }
        }
    }

    let both: Vec<_> = flipped
        .iter()
        .filter(|(_, (seen, hid))| *seen > 0 && *hid > 0)
        .collect();
    println!("files {}", files.len());
    println!("frames with a keyframe attachment {frames}");
    println!("frames flagged 0x400: {hidden}");
    println!(
        "  of those, keyframe has layers: {hidden_with_layers}, empty: {hidden_without_layers}"
    );
    println!(
        "keyframes drawn on some frames and flagged 0x400 on others: {} (of {})",
        both.len(),
        flipped.len()
    );

    assert!(frames > 0, "no frames were parsed");
    assert!(
        hidden_with_layers > hidden_without_layers * 4,
        "0x400 would be a data marker if the flagged frames had no layers"
    );
    assert!(!both.is_empty(), "no keyframe is ever toggled by 0x400");
}
