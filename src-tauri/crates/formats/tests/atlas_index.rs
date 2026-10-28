//! ```text
//! $env:MBS_ROOT="F:\Codes\UNICORN OVERLORD\extract\000"
//! $env:ATLAS_PNG_DIR="F:\Codes\UNICORN OVERLORD\extract\Main-Output\textures"
//! cargo test -p formats --test atlas_index -- --nocapture
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use formats::atlas::Atlas;

const SAMPLE: usize = 24;

#[test]
fn atlas_frame_count_matches_png_numbering() {
    let Ok(mbs_root) = std::env::var("MBS_ROOT") else {
        eprintln!("skip: MBS_ROOT is not set");
        return;
    };
    let Ok(png_dir) = std::env::var("ATLAS_PNG_DIR") else {
        eprintln!("skip: ATLAS_PNG_DIR is not set");
        return;
    };
    let mbs_root = PathBuf::from(mbs_root);
    let png_dir = PathBuf::from(png_dir);
    if !mbs_root.is_dir() || !png_dir.is_dir() {
        eprintln!("skip: directory does not exist");
        return;
    }

    let expected = png_counts(&png_dir);
    let mut files: Vec<PathBuf> = Vec::new();
    collect_mbs(&mbs_root, &mut files);
    files.sort();
    files.sort_by_key(|path| stable_hash(&path.to_string_lossy()));

    let mut checked = 0usize;
    let mut lines = Vec::new();
    let mut skipped = Vec::new();
    for path in files.iter().take(SAMPLE) {
        let atlas = match Atlas::open(path) {
            Ok(Some(atlas)) => atlas,
            Ok(None) => continue,
            Err(e) => {
                skipped.push(format!("{}: {e}", path.display()));
                continue;
            }
        };

        let Some(png_stem) = atlas
            .path()
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
        else {
            continue;
        };
        let Some(&png_count) = expected.get(&png_stem) else {
            lines.push(format!("{png_stem}: the container exists but has no matching png"));
            continue;
        };

        checked += 1;
        if atlas.len() != png_count {
            lines.push(format!(
                "{png_stem}: container has {} frames, {png_count} png files ({})",
                atlas.len(),
                atlas.path().display()
            ));
        }
    }

    if checked == 0 {
        eprintln!("skip: no pairable sample in the corpus");
        return;
    }
    eprintln!("checked the atlas frame count of {checked} stems");
    if !skipped.is_empty() {
        eprintln!("skipped {} (container ownership unknown):", skipped.len());
        for line in skipped.iter().take(5) {
            eprintln!("  {line}");
        }
    }
    assert!(
        lines.is_empty(),
        "atlas index convention mismatch:\n{}",
        lines.join("\n")
    );
}

fn png_counts(dir: &Path) -> HashMap<String, usize> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return HashMap::new();
    };
    let mut counts: HashMap<String, usize> = HashMap::new();
    for path in entries.flatten().map(|entry| entry.path()) {
        let Some(name) = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            continue;
        };
        let Some(rest) = name.strip_suffix(".png") else {
            continue;
        };
        let Some((stem, index)) = rest.rsplit_once('.') else {
            continue;
        };
        if index.parse::<usize>().is_ok() {
            *counts.entry(stem.to_string()).or_insert(0) += 1;
        }
    }
    counts
}

fn collect_mbs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.is_dir() {
            collect_mbs(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "mbs") {
            out.push(path);
        }
    }
}

fn stable_hash(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
