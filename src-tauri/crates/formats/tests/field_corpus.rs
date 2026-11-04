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
    let scene = field::compose_window(&bytes, &layout, &target.name, x0, 1920.0, &load, 1920)
        .expect("the window renders");

    assert_eq!(scene.map, "AC00_00P");
    assert_eq!(
        scene.layers,
        scene.drawn + scene.models + scene.hollow + scene.missing
    );
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
fn every_prop_layer_lands_inside_its_authored_box() {
    use std::collections::HashMap;

    let Some(path) = corpus() else {
        eprintln!("no corpus, set CORPUS_UFB");
        return;
    };
    let bytes = std::fs::read(&path).expect("read ufb");
    let layout = field::parse(&bytes).expect("layout");
    let dir = path.parent().expect("directory");

    let mut cache: HashMap<usize, Vec<Vec<field::Piece>>> = HashMap::new();
    let (mut art, mut hollow, mut resolved, mut quads) = (0usize, 0usize, 0usize, 0usize);
    let mut inside = 0usize;
    let mut widths: Vec<f32> = Vec::new();
    let mut out_of_range = 0usize;

    for record in &layout.records {
        for layer in &record.layers {
            if layer.pair < 0 || layer.flags & 4 != 0 {
                continue;
            }
            let Some(pair) = layout.pairs.get(layer.pair as usize) else {
                continue;
            };
            if pair.is_panel() {
                continue;
            }
            let props = cache.entry(layer.pair as usize).or_insert_with(|| {
                field::read_pair(dir, pair)
                    .and_then(|(model, _)| formats::mbs::parse_bytes(&model, field::MBS_TAG).ok())
                    .and_then(|v55| formats::mbs::convert(&v55).ok())
                    .map(|quad| field::props(&quad))
                    .unwrap_or_default()
            });
            resolved += 1;
            let Some(pieces) = props.get(layer.panel as usize) else {
                out_of_range += 1;
                continue;
            };
            if pieces.is_empty() {
                hollow += 1;
                continue;
            }
            let Some(bounds) = field::model_bounds(layer, pieces) else {
                continue;
            };
            art += 1;
            quads += pieces.len();
            let centre_x = (bounds[0] + bounds[2]) * 0.5;
            let centre_y = (bounds[1] + bounds[3]) * 0.5;
            if centre_x >= layer.min[0] - 1.0
                && centre_x <= layer.max[0] + 1.0
                && centre_y >= layer.min[1] - 1.0
                && centre_y <= layer.max[1] + 1.0
            {
                inside += 1;
            }
            let authored = (layer.max[0] - layer.min[0]).abs();
            if authored > 1.0 {
                widths.push((bounds[2] - bounds[0]) / authored);
            }
        }
    }

    widths.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let median = widths.get(widths.len() / 2).copied().unwrap_or(0.0);
    let centred = inside as f32 / art as f32;

    eprintln!(
        "{resolved} prop layers: {art} carry art ({quads} quads), {hollow} name a prop with no art, {out_of_range} name a prop that is out of range | {inside}/{art} stay inside the authored box ({centred:.3}) | median width vs box {median:.3}"
    );
    assert_eq!(
        out_of_range, 0,
        "every prop index must fall inside the .mbs"
    );
    assert!(art > 5_000, "the corpus carries {art} props with art");
    assert!(
        centred > 0.98,
        "the rest pose must sit in the authored box ({centred:.3})"
    );
    assert!(
        (median - 1.0).abs() < 0.02,
        "the authored box is the rest pose at 1 world unit per model unit ({median:.3})"
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

#[test]
#[ignore]
fn every_ground_tile_is_a_closed_fan_on_the_records_grid() {
    let Some(path) = corpus() else {
        eprintln!("no corpus, set CORPUS_UFB");
        return;
    };
    let bytes = std::fs::read(&path).expect("read ufb");
    let layout = field::parse(&bytes).expect("layout");

    let (mut records, mut blocks) = (0usize, 0usize);
    let (mut closed, mut square, mut centred, mut inside_uv, mut on_grid, mut in_cell) =
        (0, 0, 0, 0, 0, 0);
    for record in &layout.records {
        if record.ext_count == 0 {
            continue;
        }
        records += 1;
        let (cols, rows) = field::ext_grid(record);
        if cols * rows == record.ext_count {
            on_grid += 1;
        }
        for index in 0..record.ext_count {
            let verts = field::vertices(&bytes, record.ext_at + index * field::EXT, 6);
            if verts.len() < 6 {
                break;
            }
            blocks += 1;
            let tail = &verts[5];
            if tail.x == verts[1].x
                && tail.y == verts[1].y
                && tail.u == verts[1].u
                && tail.v == verts[1].v
            {
                closed += 1;
            }
            let mut low = (f32::MAX, f32::MAX);
            let mut high = (f32::MIN, f32::MIN);
            for vertex in &verts[1..5] {
                low = (low.0.min(vertex.x), low.1.min(vertex.y));
                high = (high.0.max(vertex.x), high.1.max(vertex.y));
            }
            if (high.0 - low.0 - field::TERRAIN_CELL).abs() < 0.5
                && (high.1 - low.1 - field::TERRAIN_CELL).abs() < 0.5
            {
                square += 1;
            }
            let centre = &verts[0];
            if centre.x >= low.0 - 0.5
                && centre.x <= high.0 + 0.5
                && centre.y >= low.1 - 0.5
                && centre.y <= high.1 + 0.5
            {
                centred += 1;
            }
            if verts
                .iter()
                .all(|v| (0.0..=1.0).contains(&v.u) && (0.0..=1.0).contains(&v.v))
            {
                inside_uv += 1;
            }
            if low.0 >= -field::TERRAIN_CELL * 0.5 - 0.5
                && high.0 <= field::TERRAIN_CELL * 0.5 + 0.5
                && low.1 >= -field::TERRAIN_CELL * 0.5 - 0.5
                && high.1 <= field::TERRAIN_CELL * 0.5 + 0.5
            {
                in_cell += 1;
            }
        }
    }

    eprintln!(
        "{records} records carry {blocks} atlas tiles: {closed} closed on v1, {square} fill their 128-unit cell, {in_cell} stay inside it, {centred} keep their centre inside, {inside_uv} stay in uv 0..1, {on_grid} records have exactly one tile per grid cell"
    );
    assert!(blocks > 260_000, "the corpus carries {blocks} tiles");
    assert_eq!(closed, blocks, "every fan must repeat its first corner");
    assert!(
        in_cell * 10 > blocks * 7,
        "most tiles fit one cell, the rest are pieces that span more ({in_cell}/{blocks})"
    );
    assert_eq!(inside_uv, blocks, "every uv must stay in the atlas");
    assert!(
        square * 10 > blocks * 7,
        "most tiles fill their cell, the rest are inset pieces ({square}/{blocks})"
    );
    assert!(
        centred as f32 / blocks as f32 > 0.99,
        "a bent tile still keeps its centre inside its corners"
    );
}

#[test]
#[ignore]
fn props_move_when_the_engine_clock_advances() {
    use std::collections::HashMap;

    let Some(path) = corpus() else {
        eprintln!("no corpus, set CORPUS_UFB");
        return;
    };
    let dir = path.parent().expect("directory").to_path_buf();
    let bytes = std::fs::read(&path).expect("read ufb");
    let layout = field::parse(&bytes).expect("layout");

    let mut seen: HashMap<&str, (usize, usize)> = HashMap::new();
    let mut files = 0usize;
    for pair in &layout.pairs {
        if pair.is_panel() {
            continue;
        }
        let name = pair.model.rsplit(['/', '\\']).next().unwrap_or("");
        let Ok(data) = std::fs::read(dir.join(name)) else {
            continue;
        };
        let Ok(v55) = formats::mbs::parse_bytes(&data, field::MBS_TAG) else {
            continue;
        };
        let Ok(quad) = formats::mbs::convert(&v55) else {
            continue;
        };
        let rest = field::props_at(&quad, 0);
        let later = field::props_at(&quad, 200_000);
        let mut moving = 0usize;
        for (left, right) in rest.iter().zip(later.iter()) {
            if left.len() != right.len()
                || left.iter().zip(right.iter()).any(|(a, b)| {
                    (a.dst[0].0 - b.dst[0].0).abs() > 0.01 || (a.dst[0].1 - b.dst[0].1).abs() > 0.01
                })
            {
                moving += 1;
            }
        }
        if !rest.is_empty() {
            files += 1;
            seen.insert(name, (moving, rest.len()));
        }
    }

    let moving: usize = seen.values().map(|(moving, _)| moving).sum();
    let total: usize = seen.values().map(|(_, props)| props).sum();
    eprintln!("{files} .mbs files: {moving} of {total} props move within 3 s");
    assert!(files > 5, "the corpus pairs {files} .mbs files");
    assert!(
        moving * 5 > total,
        "one prop in five should be animated ({moving}/{total})"
    );
}
