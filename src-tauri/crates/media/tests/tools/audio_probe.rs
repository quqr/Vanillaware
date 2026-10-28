//! `cargo run -p media --example audio_probe -- <acb|awb>...`

use std::collections::BTreeMap;
use std::path::Path;

use media::audio::{decode_track, open_acb, open_awb, Clip};

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: audio_probe <acb|awb>...");
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
                println!("{name}: failed {error}");
                continue;
            }
        };
        let elapsed = started.elapsed();
        let mut codecs: BTreeMap<String, usize> = BTreeMap::new();
        for track in &bundle.tracks {
            *codecs.entry(track.codec.tag()).or_default() += 1;
        }
        println!(
            "{name}: {} tracks / {codecs:?} / external AWB {} / parsed in {:?}",
            bundle.tracks.len(),
            bundle.external.len(),
            elapsed
        );
        for warning in &bundle.warnings {
            println!("  ! {warning}");
        }
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
                        "  - {} [{tag}] {} Hz / {channels} ch / {frames} frames / {:.2}s / peak {peak:.3} / {:?}",
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
                    "  - {} [{tag}] {} Hz / {channels} ch / {frames} frames / stride {stride} / skip {skip} / {} packets / {:?}",
                    track.name,
                    sample_rate,
                    packets.len() / stride.max(1) as usize,
                    started.elapsed()
                ),
                Err(error) => println!("  - {} [{tag}] failed {error}", track.name),
            }
        }
    }
}
