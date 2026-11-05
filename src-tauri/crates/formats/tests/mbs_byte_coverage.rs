//! Byte-coverage probe: which fields of a closed MBS file never reach the
//! preview, and how often do the meaningful ones actually carry data?
//!
//! ```text
//! $env:MBS_DIR="F:/Codes/UNICORN OVERLORD/extract/000"
//! cargo test -p formats --test mbs_byte_coverage -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};

use formats::mbs;
use formats::mbs::quad::Pad;
use formats::mbs::V55;

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

fn hexdec(text: &str) -> u32 {
    u32::from_str_radix(text.trim_start_matches("0x"), 16).unwrap_or(0)
}

#[test]
#[ignore]
fn coverage_of_the_closed_file() {
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

    let (mut s4_total, mut s4_color, mut s4_attr, mut s4_flags) =
        (0usize, 0usize, 0usize, [0usize; 8]);
    let (mut s5_total, mut s5_bits) = (0usize, 0usize);
    let (mut s6_total, mut s6_flags) = (0usize, 0usize);
    let (mut s3_total, mut s3_xyz) = (0usize, 0usize);
    let (mut s7_total, mut s7_fog) = (0usize, 0usize);
    let (mut s8_total, mut s8_unread) = (0usize, 0usize);

    for path in &files {
        let Ok(v55): Result<V55, _> = mbs::parse_file(path, TAG) else {
            continue;
        };
        for rec in &v55.s4 {
            s4_total += 1;
            if rec.color > 0 {
                s4_color += 1;
            }
            if hexdec(&rec.attr) > 0 {
                s4_attr += 1;
            }
            let flags = hexdec(&rec.bits);
            for bit in 0..8 {
                if flags & (1 << bit) != 0 {
                    s4_flags[bit] += 1;
                }
            }
        }
        for rec in &v55.s5 {
            s5_total += 1;
            if hexdec(&rec.bits) != 0 {
                s5_bits += 1;
            }
        }
        for rec in &v55.s6 {
            s6_total += 1;
            if hexdec(&rec.bits) != 0 {
                s6_flags += 1;
            }
        }
        for rec in &v55.s3 {
            s3_total += 1;
            if rec.xyz.iter().any(|v| *v != 0.0) {
                s3_xyz += 1;
            }
        }
        for rec in &v55.s7 {
            s7_total += 1;
            if rec.fog != "0xffffffff" && rec.fog != "#ffffffff" {
                s7_fog += 1;
            }
        }
        for rec in &v55.s8 {
            s8_total += 1;
            if rec.sfx != 0 {
                s8_unread += 1;
            }
        }
    }

    println!("files {}", files.len());
    println!("s4 {s4_total}: color>0 {s4_color}, attr>0 {s4_attr}");
    println!("  s4 flag bits (0..7): {s4_flags:?}");
    println!("s5 {s5_total}: bits!=0 {s5_bits}");
    println!("s6 {s6_total}: flags!=0 {s6_flags}");
    println!("s3 {s3_total}: xyz!=0 {s3_xyz}");
    println!("s7 {s7_total}: fog!=white {s7_fog}");
    println!("s8 {s8_total}: b13/b14/b18!=0 {s8_unread}");

    // The layer list must stay non-empty for the preview to show anything.
    let mut layers = 0usize;
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
            layers += keyframe
                .layer
                .iter()
                .filter(|l| matches!(l, Pad::Filled(_)))
                .count();
        }
    }
    println!("keyframe layers that survive the skip flag: {layers}");
}
