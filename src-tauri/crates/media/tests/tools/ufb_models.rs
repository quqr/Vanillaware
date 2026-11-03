use std::collections::HashMap;
use std::path::Path;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(source) = args.next() else {
        println!("usage: ufb_models <Field.ufb> [record] [tag]");
        return;
    };
    let record: usize = args
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(6970);
    let tag = args.next().unwrap_or_else(|| "swi_unic".to_string());

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
    let Some(rec) = layout.records.get(record) else {
        println!("no record {record}");
        return;
    };

    type ModelCache = HashMap<usize, Option<Vec<(String, usize, [f32; 4])>>>;
    let mut cache: ModelCache = HashMap::new();
    let mut matched = 0usize;
    let mut total = 0usize;

    for (slot, layer) in rec.layers.iter().enumerate() {
        if layer.flags & 4 != 0 || layer.pair < 0 {
            continue;
        }
        let pair = &layout.pairs[layer.pair as usize];
        if pair.is_panel() {
            continue;
        }
        let frames = cache
            .entry(layer.pair as usize)
            .or_insert_with(|| load_frames(dir, &pair.model, &tag))
            .as_ref();
        let Some(frames) = frames else {
            println!("{slot:3} {} could not be parsed", pair.model);
            continue;
        };
        let Some((name, layers, bounds)) = frames.get(layer.panel as usize) else {
            println!(
                "{slot:3} {} panel {} is out of {}-frame range",
                pair.model,
                layer.panel,
                frames.len()
            );
            continue;
        };
        let box_w = (layer.max[0] - layer.min[0]).max(1e-3);
        let box_h = (layer.max[1] - layer.min[1]).max(1e-3);
        let frame_w = (bounds[2] - bounds[0]).max(1e-3);
        let frame_h = (bounds[3] - bounds[1]).max(1e-3);
        let box_aspect = box_w / box_h;
        let frame_aspect = frame_w / frame_h;
        let delta = (box_aspect / frame_aspect - 1.0).abs();
        total += 1;
        if delta < 0.25 {
            matched += 1;
        }
        println!(
            "{slot:3} {} panel={:4} frame#{} {name:?} {layers} layers | box {box_w:7.1}x{box_h:7.1} aspect {box_aspect:5.2} | frame {frame_w:7.1}x{frame_h:7.1} aspect {frame_aspect:5.2} | delta {:+.0}%",
            pair.model.rsplit(['/', '\\']).next().unwrap_or(""),
            layer.panel,
            layer.panel,
            delta * 100.0
        );
    }
    println!("\naspect within 25% for {matched}/{total} model layers");
}

fn load_frames(dir: &Path, model: &str, tag: &str) -> Option<Vec<(String, usize, [f32; 4])>> {
    let name = model.rsplit(['/', '\\']).next()?;
    let bytes = std::fs::read(dir.join(name)).ok()?;
    let v55 = formats::mbs::parse_bytes(&bytes, tag).ok()?;
    let quad = formats::mbs::convert(&v55).ok()?;
    let mut out = Vec::with_capacity(quad.keyframe.len());
    for cell in &quad.keyframe {
        let formats::mbs::quad::Pad::Filled(frame) = cell else {
            continue;
        };
        let mut low = (f32::MAX, f32::MAX);
        let mut high = (f32::MIN, f32::MIN);
        let mut drawn = 0usize;
        for cell in &frame.layer {
            let formats::mbs::quad::Pad::Filled(layer) = cell else {
                continue;
            };
            drawn += 1;
            for point in layer.dstquad.chunks_exact(2) {
                low = (low.0.min(point[0]), low.1.min(point[1]));
                high = (high.0.max(point[0]), high.1.max(point[1]));
            }
        }
        if drawn == 0 || low.0 > high.0 {
            out.push((frame.name.clone(), 0, [0.0, 0.0, 0.0, 0.0]));
            continue;
        }
        out.push((frame.name.clone(), drawn, [low.0, low.1, high.0, high.1]));
    }
    Some(out)
}
