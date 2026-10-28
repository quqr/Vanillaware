//! ```text
//! cargo test -p media --test bench_real -- --ignored --nocapture
//! ```

use std::time::Instant;

use media::encode::{render_full_png, render_thumb_png};

const GUI_DIR: &str = r"F:\Codes\UNICORN OVERLORD\extract\000\GUI";

#[test]
#[ignore = "requires the local UNICORN OVERLORD extraction"]
fn decode_and_encode_cost() {
    let dir = std::path::Path::new(GUI_DIR);
    if !dir.is_dir() {
        eprintln!("skipped: {} does not exist", dir.display());
        return;
    }

    let mut best: Option<(u64, std::path::PathBuf)> = None;
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("ftx") {
            continue;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        if best.as_ref().is_none_or(|(b, _)| size > *b) {
            best = Some((size, path));
        }
    }

    let Some((size, path)) = best else {
        eprintln!("skipped: no .ftx in the directory");
        return;
    };
    eprintln!(
        "sample: {} ({:.1} MB)",
        path.display(),
        size as f64 / 1048576.0
    );

    let t = Instant::now();
    let infos = formats::ftx::probe_file(&path).unwrap();
    eprintln!("probe {} frames: {:?}", infos.len(), t.elapsed());

    let t = Instant::now();
    let decoded = formats::ftx::decode_file_frame(&path, 0).unwrap();
    let decode_ms = t.elapsed();
    eprintln!(
        "decode frame 0 ({}×{}, {}ch): {:?}",
        decoded.width, decoded.height, decoded.channels, decode_ms
    );

    let t = Instant::now();
    let thumb = render_thumb_png(&decoded).unwrap();
    eprintln!(
        "thumbnail PNG: {:?}, {:.0} KB",
        t.elapsed(),
        thumb.len() as f64 / 1024.0
    );

    let t = Instant::now();
    let full = render_full_png(&decoded).unwrap();
    eprintln!(
        "full-size PNG: {:?}, {:.1} MB",
        t.elapsed(),
        full.len() as f64 / 1048576.0
    );
}
