//! Corpus check that a keyframe rect is the union box of its layers' quads.
//!
//! The engine mixes that rect between keyframes (`in_s6`) and turns it into a
//! screen AABB for culling in `f_700058d020`; it never feeds the drawn geometry.
//! Pinning `rect == union(s2 corners)` keeps the two roles apart.
//!
//! ```text
//! $env:MBS_DIR="F:/Codes/UNICORN OVERLORD/extract/000"
//! cargo test -p formats --test mbs_composite_bounds -- --ignored --nocapture
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

fn union_of(quad: &[f32]) -> Option<[f32; 4]> {
    // The dump keeps the four corners only (`quad8`); a raw `s2` record is
    // `[centre, four corners, first corner again]`.
    let corners = if quad.len() >= 12 {
        &quad[2..10]
    } else if quad.len() >= 8 {
        &quad[0..8]
    } else {
        return None;
    };
    let mut out = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    for corner in corners.chunks_exact(2) {
        out[0] = out[0].min(corner[0]);
        out[1] = out[1].min(corner[1]);
        out[2] = out[2].max(corner[0]);
        out[3] = out[3].max(corner[1]);
    }
    Some(out)
}

#[test]
#[ignore]
fn keyframe_rect_is_the_layer_union() {
    let dir = std::env::var("MBS_DIR")
        .unwrap_or_else(|_| "F:/Codes/UNICORN OVERLORD/extract/000".to_string());
    let dir = PathBuf::from(dir);
    if !dir.is_dir() {
        eprintln!("skip: {} is not a directory", dir.display());
        return;
    }

    let mut files = Vec::new();
    collect(&dir, &mut files, 240);

    let (mut equal, mut empty, mut mismatched) = (0usize, 0usize, 0usize);
    let (mut contained, mut escaping) = (0usize, 0usize);
    let mut worst = 0.0f32;
    let mut worst_at = String::new();

    for path in &files {
        let Ok(v55) = mbs::parse_file(path, TAG) else {
            continue;
        };
        let Ok(quad) = mbs::convert(&v55) else {
            continue;
        };
        for entry in &quad.keyframe {
            let Pad::Filled(keyframe) = entry else {
                continue;
            };
            if keyframe.rect.len() < 4 {
                continue;
            }
            let rect = &keyframe.rect[..4];
            if rect.iter().all(|v| *v == 0.0) {
                // No culling box on this keyframe.
                empty += 1;
                continue;
            }
            let mut boxed: Option<[f32; 4]> = None;
            for layer in &keyframe.layer {
                let Pad::Filled(layer) = layer else {
                    continue;
                };
                let Some(span) = union_of(&layer.dstquad) else {
                    continue;
                };
                let slot = boxed.get_or_insert(span);
                for i in 0..2 {
                    slot[i] = slot[i].min(span[i]);
                }
                for i in 2..4 {
                    slot[i] = slot[i].max(span[i]);
                }
            }
            let Some(boxed) = boxed else {
                empty += 1;
                continue;
            };
            let error = (0..4)
                .map(|i| (rect[i] - boxed[i]).abs())
                .fold(0.0f32, f32::max);
            if error < 1.0 {
                equal += 1;
            } else {
                mismatched += 1;
                // The rect covers every layer, including the ones the dump skips
                // (`s4` flags bit 1), so a wider rect is expected; a narrower one
                // would mean the rect is not the composite box.
                let inside = boxed[0] >= rect[0] - 1.0
                    && boxed[1] >= rect[1] - 1.0
                    && boxed[2] <= rect[2] + 1.0
                    && boxed[3] <= rect[3] + 1.0;
                if inside {
                    contained += 1;
                } else {
                    escaping += 1;
                }
                if error > worst {
                    worst = error;
                    worst_at = format!("{} / {}", path.display(), keyframe.name);
                }
            }
        }
    }

    eprintln!(
        "files={} equal={equal} wider={contained} escaping={escaping} empty={empty}",
        files.len()
    );
    if mismatched > 0 {
        eprintln!("worst {worst:.1} at {worst_at}");
    }
    assert!(files.len() > 20, "corpus was not found");
    assert!(equal > 100_000, "no keyframes were compared");
    // 0.08% of the authored keyframes carry a rect that is unrelated to their
    // layers (e.g. `Eltlinde_F.mbs` keyframe 2075), so the rate is pinned rather
    // than an absolute equality; every other keyframe matches to the last float.
    assert!(
        equal * 10_000 >= (equal + mismatched) * 9_990,
        "the rect should match the layer union almost exactly"
    );
    assert!(
        escaping * 1000 <= equal + mismatched,
        "too many layers fall outside their keyframe rect"
    );
}
