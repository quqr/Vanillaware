use std::path::Path;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = Path::new(&args[1]);
    let map = args.get(2).cloned().unwrap_or_else(|| "AC00_00P".into());
    let x0: f32 = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(-7000.0);
    let width: f32 = args.get(4).and_then(|v| v.parse().ok()).unwrap_or(1920.0);
    let side: u32 = args.get(5).and_then(|v| v.parse().ok()).unwrap_or(1200);
    let frames: usize = args.get(6).and_then(|v| v.parse().ok()).unwrap_or(8);
    let terrain = args.get(7).map(|value| value == "1").unwrap_or(false);

    let bytes = std::fs::read(path).unwrap();
    let layout = formats::field::parse(&bytes).unwrap();
    let dir = path.parent().unwrap().to_path_buf();
    let load = |pair: &formats::field::Pair| formats::field::read_pair(&dir, pair);
    let render = formats::field::Render { clock: 0, terrain };

    let started = Instant::now();
    let Some(play) =
        formats::field::Playback::prepare(&bytes, &layout, &map, x0, width, &load, side, render)
    else {
        println!("no scene");
        return;
    };
    println!(
        "prepare {:.0} ms · {} layers · {} moving props · {:.1} MB of bands",
        started.elapsed().as_secs_f64() * 1000.0,
        play.layers,
        play.moving,
        play.footprint() as f64 / (1024.0 * 1024.0)
    );

    let mut times = Vec::new();
    for frame in 0..frames {
        let clock = frame as u64 * 1000;
        let started = Instant::now();
        let fast = play.frame(clock);
        let took = started.elapsed().as_secs_f64() * 1000.0;
        times.push(took);

        let exact = formats::field::compose_window_with(
            &bytes,
            &layout,
            &map,
            x0,
            width,
            &load,
            side,
            formats::field::Render { clock, terrain },
        )
        .expect("scene");
        let same = fast.raster.width == exact.raster.width
            && fast.raster.height == exact.raster.height
            && fast.raster.rgba == exact.raster.rgba;
        let worst = fast
            .raster
            .rgba
            .iter()
            .zip(exact.raster.rgba.iter())
            .map(|(a, b)| (*a as i32 - *b as i32).abs())
            .max()
            .unwrap_or(0);
        println!(
            "frame {frame}: {:.1} ms · {}x{} · {} · worst channel {worst}",
            took,
            fast.raster.width,
            fast.raster.height,
            if same { "identical" } else { "DIFFERS" }
        );
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("median {:.1} ms", times[times.len() / 2]);
}
