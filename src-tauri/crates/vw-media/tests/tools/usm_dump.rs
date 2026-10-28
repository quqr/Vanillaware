//! USM 码流导出：把某一段帧的 Annex-B 字节写到文件，交给 ffmpeg 独立验证。
//!
//! `cargo run -p vw-media --example usm_dump -- <file.usm> <from> <to> <out.h264>`
//!
//! `from` / `to` 是帧下标（`to` 不含）。GOP 0 就是 `0 30`：0..29 帧。

use std::path::Path;

use vw_media::video::open;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 4 {
        eprintln!("用法: usm_dump <file.usm> <from> <to> <out.h264>");
        return;
    }
    let path = Path::new(&args[0]);
    let from: usize = args[1].parse().expect("from 必须是帧下标");
    let to: usize = args[2].parse().expect("to 必须是帧下标");
    let out = Path::new(&args[3]);

    let index = match open(path) {
        Ok(index) => index,
        Err(error) => {
            eprintln!("索引失败: {error}");
            std::process::exit(1);
        }
    };
    let count = index.frames.len();
    if from >= count || to > count || from >= to {
        eprintln!("帧区间 {from}..{to} 不成立（共 {count} 帧）");
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
            eprintln!("取字节失败: {error}");
            std::process::exit(1);
        }
    };
    if let Err(error) = std::fs::write(out, &bytes) {
        eprintln!("写文件失败: {error}");
        std::process::exit(1);
    }
    println!(
        "{from}..{to} → {} 帧 / {} 字节 → {}",
        to - from,
        bytes.len(),
        out.display()
    );
}
