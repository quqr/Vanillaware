use std::path::Path;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(source) = args.next() else {
        println!("usage: content_render <file> <out.png> [max_side]");
        return;
    };
    let out = args.next().unwrap_or_else(|| "content.png".to_string());
    let path = Path::new(&source);
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            println!("read failed: {error}");
            return;
        }
    };
    let ext = path
        .extension()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let projection = match ext.as_str() {
        "ufb" => formats::field::view(&bytes, path.parent(), 1500),
        other => match formats::content::project(&bytes, other) {
            Some(projection) => projection,
            None => {
                println!(".{other} has no content view");
                return;
            }
        },
    };
    if let Some(note) = projection.note.as_ref() {
        println!("note: {note}");
    }
    let Some(map) = projection.color.as_ref() else {
        println!("no colour map");
        if let Some(bitmap) = projection.bitmap.as_ref() {
            println!("grayscale bitmap {}x{}", bitmap.width, bitmap.height);
        }
        return;
    };
    println!("colour map {}x{}", map.width, map.height);
    println!("legend: {}", map.legend);
    let Some(image) = image::RgbaImage::from_raw(map.width, map.height, map.rgba.clone()) else {
        println!("not a valid RGBA map");
        return;
    };
    match image.save(&out) {
        Ok(()) => println!("wrote {out}"),
        Err(error) => println!("write failed: {error}"),
    }
}
