//! `cargo run -p media --example usm_dump -- <file.usm> <from> <to> <out.h264>`

use std::path::Path;

use media::video::open;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 4 {
        eprintln!("usage: usm_dump <file.usm> <from> <to> <out.h264>");
        return;
    }
    let path = Path::new(&args[0]);
    let from: usize = args[1].parse().expect("from must be a frame index");
    let to: usize = args[2].parse().expect("to must be a frame index");
    let out = Path::new(&args[3]);

    let index = match open(path) {
        Ok(index) => index,
        Err(error) => {
            eprintln!("indexing failed: {error}");
            std::process::exit(1);
        }
    };
    let count = index.frames.len();
    if from >= count || to > count || from >= to {
        eprintln!("invalid frame range {from}..{to} ({count} frames total)");
        std::process::exit(1);
    }
    let start = index.frames[from].stream_offset;
    let end = index
        .frames
        .get(to)
        .map(|frame| frame.stream_offset)
        .unwrap_or(index.stream_len);
    let bytes = match index.read_range(path, start, end - start) {
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
    println!(
        "{from}..{to} -> {} frames / {} bytes -> {}",
        to - from,
        bytes.len(),
        out.display()
    );
}
