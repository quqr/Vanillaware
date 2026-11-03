//! Corpus check for the timeline knobs the engine's player reads.
//!
//! ```text
//! $env:MBS_DIR="F:/Codes/UNICORN OVERLORD/extract/000/CharaNpc"
//! cargo test -p formats --test mbs_timeline -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};

use formats::mbs::quad::Pad;
use formats::mbs::{self, V55};

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

#[test]
#[ignore]
fn timeline_keeps_engine_knobs() {
    let dir = std::env::var("MBS_DIR")
        .unwrap_or_else(|_| "F:/Codes/UNICORN OVERLORD/extract/000/CharaNpc".to_string());
    let dir = PathBuf::from(dir);
    if !dir.is_dir() {
        eprintln!("skip: {} is not a directory", dir.display());
        return;
    }

    let mut files = Vec::new();
    collect(&dir, &mut files, 40);
    files.sort();

    let mut continuous = 0usize;
    let mut quantised = 0usize;
    let mut linear = 0usize;
    let mut spline = 0usize;
    let mut quad_linear = 0usize;
    let mut quad_spline = 0usize;
    let mut frames = 0usize;
    let mut timed = 0usize;

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
                if entry.time > 0 {
                    timed += 1;
                }
                if entry.in_rate <= 1 {
                    continuous += 1;
                } else {
                    quantised += 1;
                }
                match entry.matrix_mix_id {
                    1 => linear += 1,
                    2 => spline += 1,
                    _ => {}
                }
                match entry.dstquad_mix_id {
                    1 => quad_linear += 1,
                    2 => quad_spline += 1,
                    _ => {}
                }
            }
        }
    }

    eprintln!("files={} frames={frames} timed={timed}", files.len());
    eprintln!("in_rate: continuous={continuous} quantised={quantised}");
    eprintln!("matrix mix: linear={linear} spline={spline}");
    eprintln!("quad mix: linear={quad_linear} spline={quad_spline}");

    assert!(frames > 0, "no timeline entries were produced");
    assert_eq!(timed, frames, "every s8 frame should carry a duration");
    assert!(
        continuous > 0,
        "in_rate should default to the continuous ramp"
    );
    assert!(quantised > 0, "some frames should use a stepped ramp");
    assert!(
        linear + spline > 0,
        "some frames should request a transform mix"
    );
    assert!(
        quad_linear + quad_spline > 0,
        "some frames should request a quad mix"
    );
}
