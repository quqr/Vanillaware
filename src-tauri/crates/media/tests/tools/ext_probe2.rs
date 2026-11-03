use std::collections::HashMap;

use formats::field;

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let layout = field::parse(&bytes).unwrap();
    let dir = std::path::Path::new(&path).parent().unwrap();

    let mut panels: HashMap<usize, usize> = HashMap::new();
    for (slot, pair) in layout.pairs.iter().enumerate() {
        let count = if pair.is_panel() {
            std::fs::read(dir.join(pair.model.rsplit(['/', '\\']).next().unwrap()))
                .ok()
                .map(|model| field::panels(&model).len())
                .unwrap_or(0)
        } else {
            0
        };
        panels.insert(slot, count);
    }

    for (slot, pair) in layout.pairs.iter().enumerate() {
        println!(
            "{slot:2} {:<26} {:<26} panels {:5} panel? {}",
            pair.model.rsplit(['/', '\\']).next().unwrap_or(""),
            pair.texture.rsplit(['/', '\\']).next().unwrap_or(""),
            panels.get(&slot).copied().unwrap_or(0),
            pair.is_panel()
        );
    }

    let mut stats: HashMap<(&str, bool), usize> = HashMap::new();
    let mut ext_hist: HashMap<u8, usize> = HashMap::new();
    for record in &layout.records {
        *ext_hist.entry(record.ext_count as u8).or_insert(0) += 1;
        for layer in &record.layers {
            let key_name = if layer.pair < 0 {
                "none"
            } else if layout.pairs[layer.pair as usize].is_panel() {
                "mpd"
            } else {
                "mbs"
            };
            let in_range = layer
                .pair
                .ge(&0)
                .then(|| {
                    panels
                        .get(&(layer.pair as usize))
                        .map(|count| (layer.panel as usize) < *count && *count > 0)
                        .unwrap_or(false)
                })
                .unwrap_or(false);
            let uses_ext = (layer.ext as usize) < record.ext_count && layer.ext != 0;
            *stats.entry((key_name, in_range)).or_insert(0) += 1;
            if uses_ext {
                *stats.entry(("ext!=0", in_range)).or_insert(0) += 1;
            }
            let box_w = (layer.max[0] - layer.min[0]).abs();
            if (box_w - 128.0).abs() < 0.5 && layer.ext == 0 {
                *stats.entry(("128 & ext0", in_range)).or_insert(0) += 1;
            }
        }
    }
    let mut list: Vec<_> = stats.into_iter().collect();
    list.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    for ((name, in_range), count) in list.iter().take(20) {
        println!("{name:12} panel-in-range {in_range:5} -> {count}");
    }
    let mut hist: Vec<_> = ext_hist.into_iter().collect();
    hist.sort_by_key(|(count, _)| *count);
    println!("ext_count histogram {hist:?}");
}
