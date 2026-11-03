use std::path::PathBuf;

use formats::field;

fn corpus() -> Option<PathBuf> {
    let path = std::env::var("CORPUS_UFB")
        .unwrap_or_else(|_| r"F:\Codes\UNICORN OVERLORD\extract\000\FieldBG\Field.ufb".to_string());
    let path = PathBuf::from(path);
    if path.is_file() {
        Some(path)
    } else {
        None
    }
}

#[test]
#[ignore]
fn ufb_closes_on_the_corpus() {
    let Some(path) = corpus() else {
        eprintln!("no corpus, set CORPUS_UFB");
        return;
    };
    let bytes = std::fs::read(&path).expect("read ufb");
    let layout = field::parse(&bytes).expect("layout");

    for check in field::checks(&layout, bytes.len()) {
        assert!(
            check.ok,
            "{} ({}) failed: {}",
            check.id, check.what, check.detail
        );
    }

    assert_eq!(layout.counts.records, 14490);
    assert_eq!(layout.counts.records as usize, field::COLUMNS * field::ROWS);
    assert_eq!(layout.derived.layers, 123280);
    assert_eq!(layout.derived.panel_layers, 117146);
    assert_eq!(layout.derived.model_layers, 6134);
    assert_eq!(layout.counts.objects, 123291);
    assert_eq!(
        layout.counts.model_objects + layout.counts.panel_objects,
        layout.counts.objects
    );
    assert_eq!(
        layout.masks.iter().filter(|mask| mask.group >= 0).count(),
        179
    );
    assert_eq!(layout.samples.len(), 337);
    assert_eq!(layout.group_names.len(), 179);
    assert_eq!(layout.names.len(), 4);

    let ext_vertices = layout
        .records
        .iter()
        .map(|record| record.ext_count)
        .sum::<usize>()
        * 6;
    assert_eq!(ext_vertices, 1587606);

    let first = &layout.records[0];
    assert_eq!((first.column, first.row), (0, 0));
    let last = layout.records.last().expect("records");
    assert_eq!((last.column, last.row), (137, 104));
    assert_eq!(last.index, field::COLUMNS * field::ROWS - 1);
}

#[test]
#[ignore]
fn ufb_document_reports_every_check_as_passed() {
    let Some(path) = corpus() else {
        eprintln!("no corpus, set CORPUS_UFB");
        return;
    };
    let bytes = std::fs::read(&path).expect("read ufb");
    let arena = formats::document::parse_bytes(&bytes, "ufb", None).expect("doc");
    let failed: Vec<String> = arena
        .checks()
        .iter()
        .filter(|check| !check.ok)
        .map(|check| format!("{}: {:?}", check.id, check.detail))
        .collect();
    assert!(failed.is_empty(), "failed checks: {failed:?}");
    assert!(arena.checks().len() >= 8);
}

#[test]
#[ignore]
fn ucb_closes_on_the_corpus() {
    let Some(ufb) = corpus() else {
        eprintln!("no corpus, set CORPUS_UFB");
        return;
    };
    let ucb = ufb.with_file_name("Field.ucb");
    if !ucb.is_file() {
        eprintln!("no Field.ucb next to the ufb");
        return;
    }
    let bytes = std::fs::read(&ucb).expect("read ucb");
    let arena = formats::document::parse_bytes(&bytes, "ucb", None).expect("doc");
    let failed: Vec<String> = arena
        .checks()
        .iter()
        .filter(|check| !check.ok)
        .map(|check| format!("{}: {:?}", check.id, check.detail))
        .collect();
    assert!(failed.is_empty(), "failed checks: {failed:?}");
    let ids = arena
        .checks()
        .iter()
        .find(|check| check.id == "ucb.ids")
        .expect("ucb.ids");
    assert!(ids.ok, "{:?}", ids.detail);
}

#[test]
#[ignore]
fn ufb_masks_reproduce_the_ucb_grid_exactly() {
    let Some(ufb) = corpus() else {
        eprintln!("no corpus, set CORPUS_UFB");
        return;
    };
    let ucb = ufb.with_file_name("Field.ucb");
    if !ucb.is_file() {
        eprintln!("no Field.ucb next to the ufb");
        return;
    }
    let bytes = std::fs::read(&ufb).expect("read ufb");
    let layout = field::parse(&bytes).expect("layout");
    let grid = std::fs::read(&ucb).expect("read ucb");

    let matched = field::cross_check(&layout, &bytes, &grid).expect("ucb has the grid size");
    eprintln!(
        "{} / {} cells identical, {} painted, {} clipped, {} open, {} underlay, first {:?}",
        matched.same,
        matched.cells,
        matched.painted,
        matched.clipped,
        matched.open,
        matched.underlay,
        matched.first
    );
    assert_eq!(matched.clipped, 0, "no mask cell may fall outside the grid");
    assert_eq!(matched.underlay, 8, "eight masks are flagged 0x4700");
    assert_eq!(
        matched.same, matched.cells,
        "first mismatch: {:?}",
        matched.first
    );
}

#[test]
#[ignore]
fn a_placement_set_renders_one_window_without_borrowing_other_sets() {
    let Some(path) = corpus() else {
        eprintln!("no corpus, set CORPUS_UFB");
        return;
    };
    let bytes = std::fs::read(&path).expect("read ufb");
    let layout = field::parse(&bytes).expect("layout");
    let dir = path.parent().expect("directory");

    let sets = field::maps(&layout);
    assert_eq!(sets.len(), 25, "the corpus holds 25 placement sets");

    let target = sets
        .iter()
        .find(|entry| entry.name == "AC00_00P")
        .expect("AC00_00P is present");
    let x0 = field::default_window(&layout, &target.name, 1920.0);
    let load = |pair: &field::Pair| field::read_pair(dir, pair);
    let scene = field::compose_window(&layout, &target.name, x0, 1920.0, &load, 1920)
        .expect("the window renders");

    assert_eq!(scene.map, "AC00_00P");
    assert_eq!(scene.layers, scene.drawn + scene.models + scene.missing);
    assert!(scene.drawn > 0, "nothing was drawn");
    assert_eq!(scene.missing, 0, "every panel resolved");
    assert_eq!(
        scene.raster.width, 1920,
        "1 px = 1 world unit across the window"
    );
    assert!(
        (1..=3840).contains(&scene.raster.height),
        "the window is bounded by the same scale"
    );

    let others = layout
        .pairs
        .iter()
        .enumerate()
        .filter(|(_, pair)| field::family(pair).as_deref() != Some("AC00"))
        .count();
    assert_eq!(
        others,
        39 - 11,
        "AC00 owns 11 pairs, the rest belong elsewhere"
    );
}

#[test]
#[ignore]
fn ucb_shades_by_area_and_by_class_without_moving_the_grid() {
    let Some(ufb) = corpus() else {
        eprintln!("no corpus, set CORPUS_UFB");
        return;
    };
    let ucb = ufb.with_file_name("Field.ucb");
    if !ucb.is_file() {
        eprintln!("no Field.ucb next to the ufb");
        return;
    }
    let bytes = std::fs::read(&ucb).expect("read ucb");

    let area = formats::content::ucb_at(&bytes, formats::content::UcbMode::Area);
    let class = formats::content::ucb_at(&bytes, formats::content::UcbMode::Class);
    let area_map = area.color.as_ref().expect("area image");
    let class_map = class.color.as_ref().expect("class image");

    assert_eq!(
        (area_map.width, area_map.height),
        (class_map.width, class_map.height)
    );
    assert!(
        area_map.legend.contains("area"),
        "area legend names the area id"
    );
    assert!(
        class_map.legend.contains("terrain class"),
        "class legend names the class"
    );
    assert_ne!(
        area_map.rgba, class_map.rgba,
        "the two shadings must not produce the same picture"
    );
}

#[test]
#[ignore]
fn the_authored_box_is_the_quad_rotated_into_the_world() {
    use std::collections::HashMap;

    let Some(path) = corpus() else {
        eprintln!("no corpus, set CORPUS_UFB");
        return;
    };
    let bytes = std::fs::read(&path).expect("read ufb");
    let layout = field::parse(&bytes).expect("layout");
    let dir = path.parent().expect("directory");

    let mut cache: HashMap<usize, Vec<field::Panel>> = HashMap::new();
    let mut total = 0usize;
    let mut matched = 0usize;
    let mut worst: Vec<(f32, String, usize, usize)> = Vec::new();
    let (mut under, mut over) = (0f32, 0f32);

    for record in &layout.records {
        for layer in &record.layers {
            if layer.pair < 0 || layer.flags & 4 == 0 {
                continue;
            }
            let slot = layer.pair as usize;
            let Some(pair) = layout.pairs.get(slot) else {
                continue;
            };
            let panels = cache.entry(slot).or_insert_with(|| {
                field::read_pair(dir, pair)
                    .map(|(model, _)| field::panels(&model))
                    .unwrap_or_default()
            });
            let Some(panel) = panels.get(layer.panel as usize) else {
                continue;
            };

            let quad = field::quad_corners(layer, panel);
            let mut low = f32::MAX;
            let mut high = f32::MIN;
            for corner in &quad {
                low = low.min(corner.0);
                high = high.max(corner.0);
            }
            let drawn = high - low;
            let authored = (layer.max[0] - layer.min[0]).abs();

            total += 1;
            let drift = drawn - authored;
            if drift > over {
                over = drift;
            }
            if drift < under {
                under = drift;
            }
            let slack = 1.5 + 0.01 * authored;
            if drift.abs() <= slack {
                matched += 1;
            } else if worst.len() < 8 {
                worst.push((
                    drift,
                    pair.model.clone(),
                    layer.panel as usize,
                    layer.roll as usize,
                ));
            }
        }
    }

    let rate = matched as f32 / total as f32;
    eprintln!(
        "{matched}/{total} panel layers keep the authored box width ({rate:.4}), drift {under:+.1}..{over:+.1}"
    );
    for entry in &worst {
        eprintln!(
            "   drift {:+.1} on {} panel {} roll {}",
            entry.0, entry.1, entry.2, entry.3
        );
    }
    assert!(total > 100_000, "the corpus carries {total} panel layers");
    assert!(
        rate > 0.94,
        "the drawn quad must reproduce the authored box width ({rate:.4})"
    );
}
