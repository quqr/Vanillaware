//! 音频容器探针：验证 ACB/AWB 的轨道列表与解码结果。
//!
//! `cargo run -p vw-media --example audio_probe -- <acb|awb>...`

use std::collections::BTreeMap;
use std::path::Path;

use vw_media::audio::{decode_track, open_acb, open_awb, Clip};

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("用法: audio_probe <acb|awb>...");
        return;
    }
    for path in &paths {
        let target = Path::new(path);
        let name = target
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.clone());
        let started = std::time::Instant::now();
        let bundle = if path.to_ascii_lowercase().ends_with(".awb") {
            open_awb(target)
        } else {
            open_acb(target)
        };
        let bundle = match bundle {
            Ok(bundle) => bundle,
            Err(error) => {
                println!("{name}: 失败 {error}");
                continue;
            }
        };
        let elapsed = started.elapsed();
        let mut codecs: BTreeMap<String, usize> = BTreeMap::new();
        for track in &bundle.tracks {
            *codecs.entry(track.codec.tag()).or_default() += 1;
        }
        println!(
            "{name}: {} 条 / {codecs:?} / 外部 AWB {} / 解析 {:?}",
            bundle.tracks.len(),
            bundle.external.len(),
            elapsed
        );
        for warning in &bundle.warnings {
            println!("  ! {warning}");
        }
        // 每种编码各解一条。
        let mut seen: BTreeMap<String, bool> = BTreeMap::new();
        for track in &bundle.tracks {
            let tag = track.codec.tag();
            if seen.get(&tag).copied().unwrap_or(false) {
                continue;
            }
            seen.insert(tag.clone(), true);
            let started = std::time::Instant::now();
            match decode_track(&bundle, track.index) {
                Ok(Clip::Pcm {
                    sample_rate,
                    channels,
                    frames,
                    samples,
                }) => {
                    let peak = samples
                        .iter()
                        .fold(0i32, |acc, value| acc.max((*value as i32).abs())) as f32
                        / i16::MAX as f32;
                    println!(
                        "  · {} [{tag}] {} Hz / {channels} ch / {frames} 帧 / {:.2}s / peak {peak:.3} / {:?}",
                        track.name,
                        sample_rate,
                        frames as f64 / sample_rate.max(1) as f64,
                        started.elapsed()
                    );
                }
                Ok(Clip::OpusPackets {
                    sample_rate,
                    channels,
                    frames,
                    stride,
                    skip,
                    packets,
                }) => println!(
                    "  · {} [{tag}] {} Hz / {channels} ch / {frames} 帧 / stride {stride} / skip {skip} / {} 包 / {:?}",
                    track.name,
                    sample_rate,
                    packets.len() / stride.max(1) as usize,
                    started.elapsed()
                ),
                Err(error) => println!("  · {} [{tag}] 失败 {error}", track.name),
            }
        }
    }
}
