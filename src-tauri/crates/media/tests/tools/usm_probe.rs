//! USM 探针：验证块表、帧表与 SPS。

use std::path::Path;

use media::video::open;

fn main() {
    for path in std::env::args().skip(1) {
        let started = std::time::Instant::now();
        match open(Path::new(&path)) {
            Ok(index) => {
                let name = path.rsplit(['/', '\\']).next().unwrap_or(&path);
                println!(
                    "{name}: {}×{} {} (profile {} level {}) / {:.3} fps / {} 帧 / {} 块 / 流 {} MB / 音频 {} / 扫描 {:?}",
                    index.width,
                    index.height,
                    index.codec,
                    index.profile,
                    index.level,
                    index.fps_centi as f64 / 100.0,
                    index.frames.len(),
                    index.chunks.len(),
                    index.stream_len / (1024 * 1024),
                    index.audio,
                    started.elapsed()
                );
                let keys = index.keyframes();
                println!(
                    "  关键帧 {} 个 / 平均 GOP {:.1} 帧 / 首帧 {} B",
                    keys.len(),
                    index.frames.len() as f64 / keys.len().max(1) as f64,
                    index.frames[0].size
                );
                if let Some(&first) = keys.first() {
                    let next = keys.get(1).copied().unwrap_or(index.frames.len());
                    let from = index.frames[first].stream_offset;
                    let to = index
                        .frames
                        .get(next)
                        .map(|frame| frame.stream_offset)
                        .unwrap_or(index.stream_len);
                    match index.read_range(Path::new(&path), from, to - from) {
                        Ok(bytes) => {
                            let head: String = bytes
                                .iter()
                                .take(20)
                                .map(|value| format!("{value:02x}"))
                                .collect::<Vec<_>>()
                                .join(" ");
                            println!("  首个 GOP {} KB / head {head}", bytes.len() / 1024);
                        }
                        Err(error) => println!("  GOP 读取失败: {error}"),
                    }
                }
            }
            Err(error) => println!("{path}: 失败 {error}"),
        }
    }
}
