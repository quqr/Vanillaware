use std::path::Path;

use formats::ftx;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        return Err("usage: ftx_page <file.ftx> <page> <out.png>".into());
    }
    let path = Path::new(&args[1]);
    let page = args[2]
        .parse::<usize>()
        .map_err(|_| "page must be a number".to_string())?;
    let out = Path::new(&args[3]);

    let info = ftx::probe_file(path).map_err(|error| format!("{error}"))?;
    for (index, entry) in info.iter().enumerate() {
        println!(
            "page {index}: {} {}x{} {} ch{}",
            entry.name,
            entry.width,
            entry.height,
            entry.format_name(),
            entry.channels
        );
    }
    let texture = ftx::decode_file_frame(path, page).map_err(|error| format!("{error}"))?;
    let image = image::RgbaImage::from_raw(texture.width, texture.height, texture.rgba())
        .ok_or_else(|| "decoded size mismatch".to_string())?;
    image.save(out).map_err(|error| format!("{error}"))?;
    println!(
        "wrote {} ({}x{})",
        out.display(),
        texture.width,
        texture.height
    );
    Ok(())
}
