use std::path::Path;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(source) = args.next() else {
        println!("usage: ufb_render <Field.ufb> <out.png> [map] [x0] [width]");
        return;
    };
    let out = args.next().unwrap_or_else(|| "ufb.png".to_string());
    let wanted = args.next();
    let x0: Option<f32> = args.next().and_then(|value| value.parse().ok());
    let width: f32 = args
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(formats::field::WINDOW_DEFAULT);
    let max_side: u32 = args
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1500);

    let path = Path::new(&source);
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            println!("read failed: {error}");
            return;
        }
    };
    let Some(layout) = formats::field::parse(&bytes) else {
        println!("parse failed");
        return;
    };
    let dir = path.parent().unwrap_or(Path::new("."));

    let catalog = formats::field::maps(&layout);
    println!("{} layer sets", catalog.len());
    for entry in catalog.iter().take(10) {
        println!(
            "  {:<12} {:5} {:6} layers {:5} tiles  {:.0}x{:.0}  x {:.0}..{:.0}",
            entry.name,
            entry.group,
            entry.layers,
            entry.tiles,
            entry.window[2] - entry.window[0],
            entry.window[3] - entry.window[1],
            entry.window[0],
            entry.window[2]
        );
    }

    let scope = match wanted {
        Some(name) => name,
        None => match formats::field::default_scope(&layout, width) {
            Some((_, stage, _)) => stage,
            None => {
                println!("no layer set");
                return;
            }
        },
    };

    let x0 = x0.unwrap_or_else(|| formats::field::default_window(&layout, &scope, width));
    println!(
        "rendering {scope} with window x {:.0}..{:.0}",
        x0,
        x0 + width
    );
    let load = |pair: &formats::field::Pair| formats::field::read_pair(dir, pair);
    let Some(scene) = formats::field::compose_window(&layout, &scope, x0, width, &load, max_side)
    else {
        println!("compose returned nothing");
        return;
    };
    println!(
        "  {} layers, {} drawn, {} model layers skipped, {} unresolved, raster {}x{}",
        scene.layers,
        scene.drawn,
        scene.models,
        scene.missing,
        scene.raster.width,
        scene.raster.height
    );
    let Some(image) =
        image::RgbaImage::from_raw(scene.raster.width, scene.raster.height, scene.raster.rgba)
    else {
        println!("  raster is not a valid RGBA image");
        return;
    };
    match image.save(&out) {
        Ok(()) => println!("  wrote {out}"),
        Err(error) => println!("  failed to write {out}: {error}"),
    }
}
