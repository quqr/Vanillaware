use std::path::Path;

use formats::DecodedTexture;
use tempfile::TempDir;

use media::cache::{Stamp, ThumbCache};
use media::encode::render_thumb_png;

fn texture(width: u32, height: u32, channels: u8) -> DecodedTexture {
    let len = (width * height * u32::from(channels)) as usize;
    let mut data = Vec::with_capacity(len);
    for i in 0..len {
        data.push((i % 251) as u8);
    }

    DecodedTexture {
        name: "test".into(),
        width,
        height,
        format: 0x4D,
        channels,
        data,
    }
}

fn touch_mtime(path: &Path) {
    let mut content = std::fs::read(path).unwrap();
    content.push(0);
    std::fs::write(path, content).unwrap();
}

#[test]
fn png_is_bounded_by_max_edge_and_never_upscales() {
    let edge = media::encode::THUMBNAIL_MAX_EDGE;

    let big = render_thumb_png(&texture(1024, 512, 4)).unwrap();
    let decoded = image::load_from_memory(&big).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (edge, edge / 2));

    let small = render_thumb_png(&texture(96, 96, 4)).unwrap();
    let decoded = image::load_from_memory(&small).unwrap();
    assert_eq!(
        (decoded.width(), decoded.height()),
        (96, 96),
        "small images are not upscaled"
    );

    let odd = render_thumb_png(&texture(960, 540, 4)).unwrap();
    let decoded = image::load_from_memory(&odd).unwrap();
    assert_eq!(
        (decoded.width(), decoded.height()),
        (edge, edge * 540 / 960)
    );
}

#[test]
fn single_channel_texture_expands_to_rgba() {
    let png = render_thumb_png(&texture(64, 64, 1)).unwrap();
    let decoded = image::load_from_memory(&png).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (64, 64));
    assert_eq!(decoded.color(), image::ColorType::Rgba8);
}

#[test]
fn cache_hit_and_invalidation() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("Sample.ftx");
    std::fs::write(&source, vec![1u8; 128]).unwrap();

    let cache_root = dir.path().join("thumbs");
    let cache = ThumbCache::new(&cache_root);

    let frames = vec![media::TextureInfo {
        name: "Sample.0".into(),
        width: 64,
        height: 64,
        format: 0x4D,
        channels: 4,
    }];
    cache
        .ensure_meta(&source, Stamp::of(&source), &frames)
        .unwrap();

    let misses_at_start = cache.misses();
    let first = cache
        .ensure_png(&source, Stamp::of(&source), 0, || Ok(texture(64, 64, 4)))
        .unwrap();
    assert_eq!(cache.misses(), misses_at_start + 1);

    let hits_at_start = cache.hits();
    let second = cache
        .ensure_png(&source, Stamp::of(&source), 0, || Ok(texture(64, 64, 4)))
        .unwrap();
    assert_eq!(
        cache.hits(),
        hits_at_start + 1,
        "the second lookup must hit"
    );
    assert_eq!(first, second, "a hit returns identical content");

    touch_mtime(&source);
    let misses_before = cache.misses();
    cache
        .ensure_png(&source, Stamp::of(&source), 0, || Ok(texture(64, 64, 4)))
        .unwrap();
    assert_eq!(
        cache.misses(),
        misses_before + 1,
        "an mtime change must invalidate"
    );

    let info = cache.info();
    assert_eq!(info.entries, 1);
    assert!(info.bytes > 0);
}

#[test]
fn clear_empties_cache_but_keeps_other_files() {
    let dir = TempDir::new().unwrap();
    let cache_root = dir.path().join("thumbs");
    let cache = ThumbCache::new(&cache_root);

    let source = dir.path().join("A.ftx");
    std::fs::write(&source, [1u8; 8]).unwrap();
    cache
        .ensure_png(&source, Stamp::of(&source), 0, || Ok(texture(64, 64, 4)))
        .unwrap();

    let settings = dir.path().join("settings.json");
    std::fs::write(&settings, b"{}").unwrap();

    let removed = cache.clear().unwrap();
    assert_eq!(removed.entries, 1);
    assert_eq!(cache_root.read_dir().unwrap().count(), 0);
    assert!(
        settings.exists(),
        "unrelated files in the app directory must survive"
    );

    cache
        .ensure_png(&source, Stamp::of(&source), 0, || Ok(texture(64, 64, 4)))
        .unwrap();
}
