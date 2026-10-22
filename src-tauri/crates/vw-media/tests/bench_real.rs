//! 真实纹理的解码/编码耗时与产物体积。默认不跑：
//!
//! ```text
//! cargo test -p vw-media --test bench_real -- --ignored --nocapture
//! ```

use std::time::Instant;

use vw_media::encode::{render_full_png, render_thumb_png};

const GUI_DIR: &str = r"F:\Codes\UNICORN OVERLORD\extract\000\GUI";

#[test]
#[ignore = "需要本机 UNICORN OVERLORD 解包语料"]
fn decode_and_encode_cost() {
    let dir = std::path::Path::new(GUI_DIR);
    if !dir.is_dir() {
        eprintln!("跳过：{} 不存在", dir.display());
        return;
    }

    // 挑一个最大的 .ftx：最能代表最坏情况。
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
        eprintln!("跳过：目录里没有 .ftx");
        return;
    };
    eprintln!(
        "样本：{}（{:.1} MB）",
        path.display(),
        size as f64 / 1048576.0
    );

    let t = Instant::now();
    let infos = vw_formats::ftx::probe_file(&path).unwrap();
    eprintln!("probe {} 帧：{:?}", infos.len(), t.elapsed());

    let t = Instant::now();
    let decoded = vw_formats::ftx::decode_file_frame(&path, 0).unwrap();
    let decode_ms = t.elapsed();
    eprintln!(
        "decode 帧 0（{}×{}，{}ch）：{:?}",
        decoded.width, decoded.height, decoded.channels, decode_ms
    );

    let t = Instant::now();
    let thumb = render_thumb_png(&decoded).unwrap();
    eprintln!(
        "缩略图 PNG：{:?}，{:.0} KB",
        t.elapsed(),
        thumb.len() as f64 / 1024.0
    );

    let t = Instant::now();
    let full = render_full_png(&decoded).unwrap();
    eprintln!(
        "原尺寸 PNG：{:?}，{:.1} MB",
        t.elapsed(),
        full.len() as f64 / 1048576.0
    );
}
