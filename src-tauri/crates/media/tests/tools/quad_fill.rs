use std::time::Instant;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;
const TEXTURE: u32 = 1024;

/// Fill rate of the rasteriser alone: one quad covering the whole window, so
/// the number is what a full-screen fog layer costs per frame.
fn main() {
    let mut texture = vec![0u8; (TEXTURE * TEXTURE * 4) as usize];
    for (index, pixel) in texture.chunks_exact_mut(4).enumerate() {
        let x = (index as u32 % TEXTURE) as u8;
        pixel.copy_from_slice(&[x, 128, 64, 160]);
    }
    let page = formats::raster::Page::new(TEXTURE, TEXTURE, &texture).unwrap();
    let mut raster = formats::raster::Raster::new(WIDTH, HEIGHT);
    let dst = [
        (0.0f32, 0.0f32),
        (WIDTH as f32, 0.0),
        (WIDTH as f32, HEIGHT as f32),
        (0.0, HEIGHT as f32),
    ];
    let uv = [(0.0f32, 0.0f32), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
    let tint = [255u8, 200, 180, 160];

    let runs: usize = std::env::args()
        .nth(1)
        .and_then(|value| value.parse().ok())
        .unwrap_or(20);
    let started = Instant::now();
    for _ in 0..runs {
        formats::raster::paint_quad(&mut raster, dst, uv, &page, tint);
    }
    let took = started.elapsed().as_secs_f64() * 1000.0;
    let painted = WIDTH as f64 * HEIGHT as f64 * runs as f64;
    println!(
        "{runs} full-window quads on {WIDTH}x{HEIGHT}: {:.2} ms each, {:.2} ns/px",
        took / runs as f64,
        took * 1e6 / painted
    );
}
