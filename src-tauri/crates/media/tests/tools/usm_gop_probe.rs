//! `cargo run -p media --example usm_gop_probe -- <file.usm> <gop> <out.h264>`

use std::path::Path;

use media::video::open;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 3 {
        eprintln!("usage: usm_gop_probe <file.usm> <gop> <out.h264>");
        std::process::exit(2);
    }
    let path = Path::new(&args[0]);
    let gop: usize = args[1].parse().expect("gop must be a number");
    let out = Path::new(&args[2]);

    let index = match open(path) {
        Ok(index) => index,
        Err(error) => {
            eprintln!("indexing failed: {error}");
            std::process::exit(1);
        }
    };

    let keys = index.keyframes();
    let start_frame = match keys.get(gop) {
        Some(value) => *value,
        None => {
            eprintln!("GOP {gop} out of range ({} total)", keys.len());
            std::process::exit(1);
        }
    };
    let end_frame = keys.get(gop + 1).copied().unwrap_or(index.frames.len());
    let from = index.frames[start_frame].stream_offset;
    let to = index
        .frames
        .get(end_frame)
        .map(|frame| frame.stream_offset)
        .unwrap_or(index.stream_len);
    let bytes = match index.read_range(path, from, to.saturating_sub(from)) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("reading bytes failed: {error}");
            std::process::exit(1);
        }
    };
    if let Err(error) = std::fs::write(out, &bytes) {
        eprintln!("writing the file failed: {error}");
        std::process::exit(1);
    }

    let hex = |bytes: &[u8]| -> String {
        let mut text = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            text.push_str(&format!("{byte:02x}"));
        }
        text
    };
    println!(
        "{{\"width\":{},\"height\":{},\"codec\":\"{}\",\"profile\":{},\"level\":{},\
\"fps\":{:.4},\"frames\":{},\"gop\":{},\"gopCount\":{},\"startFrame\":{},\
\"gopFrames\":{},\"bytes\":{},\"sps\":\"{}\",\"pps\":\"{}\",\"audio\":\"{}\"}}",
        index.width,
        index.height,
        index.codec,
        index.profile,
        index.level,
        index.fps_centi as f32 / 100.0,
        index.frames.len(),
        gop,
        keys.len(),
        start_frame,
        end_frame - start_frame,
        bytes.len(),
        hex(&index.sps),
        hex(&index.pps),
        index.audio,
    );
}
