use std::collections::HashMap;
use std::path::Path;

use formats::field;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(source) = args.next() else {
        println!("usage: ufb_models <Field.ufb> [record] [stage]");
        return;
    };
    let record: usize = args
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(6970);
    let stage = args.next();

    let path = Path::new(&source);
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            println!("read failed: {error}");
            return;
        }
    };
    let Some(layout) = field::parse(&bytes) else {
        println!("parse failed");
        return;
    };
    let dir = path.parent().unwrap_or(Path::new("."));
    let Some(rec) = layout.records.get(record) else {
        println!("no record {record}");
        return;
    };

    let mut cache: HashMap<usize, Vec<Vec<field::Piece>>> = HashMap::new();
    let mut names: HashMap<usize, Vec<String>> = HashMap::new();
    let mut shows = 0usize;
    for (slot, layer) in rec.layers.iter().enumerate() {
        if layer.flags & 4 != 0 || layer.pair < 0 {
            continue;
        }
        let pair = &layout.pairs[layer.pair as usize];
        if pair.is_panel() {
            continue;
        }
        if let Some(want) = &stage {
            if !field::matches(want, pair) {
                continue;
            }
        }
        let key = layer.pair as usize;
        let list = cache.entry(key).or_insert_with(|| {
            field::read_pair(dir, pair)
                .and_then(|(model, _)| formats::mbs::parse_bytes(&model, field::MBS_TAG).ok())
                .and_then(|v55| formats::mbs::convert(&v55).ok())
                .map(|quad| {
                    names.insert(
                        key,
                        quad.skeleton
                            .iter()
                            .map(|cell| match cell {
                                formats::mbs::quad::Pad::Filled(skel) => skel.name.clone(),
                                formats::mbs::quad::Pad::Empty(_) => "—".to_string(),
                            })
                            .collect(),
                    );
                    field::props(&quad)
                })
                .unwrap_or_default()
        });
        let Some(pieces) = list.get(layer.panel as usize) else {
            println!(
                "{slot:3} {} prop {} is out of range",
                pair.model, layer.panel
            );
            continue;
        };
        let name = names
            .get(&key)
            .and_then(|list| list.get(layer.panel as usize))
            .cloned()
            .unwrap_or_default();
        let bounds = field::model_bounds(layer, pieces);
        let box_w = (layer.max[0] - layer.min[0]).abs();
        let box_h = (layer.max[1] - layer.min[1]).abs();
        match bounds {
            Some(bounds) => println!(
                "{slot:3} {:<24} prop {:3} {name:<18} {:2} quads | box {box_w:7.1}x{box_h:7.1} | pose {:7.1}x{:7.1} at ({:.0}, {:.0})",
                pair.model.rsplit(['/', '\\']).next().unwrap_or(""),
                layer.panel,
                pieces.len(),
                bounds[2] - bounds[0],
                bounds[3] - bounds[1],
                (bounds[0] + bounds[2]) * 0.5,
                (bounds[1] + bounds[3]) * 0.5,
            ),
            None => println!(
                "{slot:3} {:<24} prop {:3} {name:<18} no art (a collision wall)",
                pair.model.rsplit(['/', '\\']).next().unwrap_or(""),
                layer.panel
            ),
        }
        shows += 1;
    }
    println!("\n{shows} prop layers in record {record}");
}
