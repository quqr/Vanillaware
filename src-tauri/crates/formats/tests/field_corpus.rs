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
fn every_ground_piece_is_one_atlas_cell_pinned_inside_its_record() {
    let Some(path) = corpus() else {
        eprintln!("no corpus, set CORPUS_UFB");
        return;
    };
    let bytes = std::fs::read(&path).expect("read ufb");
    let layout = field::parse(&bytes).expect("layout");

    let mut records = 0usize;
    let mut blocks = 0usize;
    let mut pieces = 0usize;
    let mut square = 0usize;
    let mut one_cell = 0usize;
    let mut centred = 0usize;
    let mut inside = 0usize;
    let mut on_cell = 0usize;
    let mut closed = 0usize;

    for record in &layout.records {
        if record.groups.is_empty() {
            continue;
        }
        records += 1;
        for block in &record.groups {
            blocks += 1;
            let end = block.at + field::GROUP_HEAD + block.count as usize * field::GROUP_ENTRY;
            let next = record.at + record.size;
            if end <= next {
                closed += 1;
            }
            let quads = field::ground_quads(&bytes, block);
            assert_eq!(quads.len(), block.count as usize);
            for quad in quads {
                pieces += 1;
                let (mut lx, mut ly, mut hx, mut hy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
                for corner in &quad.corners {
                    lx = lx.min(corner.0);
                    ly = ly.min(corner.1);
                    hx = hx.max(corner.0);
                    hy = hy.max(corner.1);
                }
                if (hx - lx - field::TERRAIN_CELL).abs() < 1e-3
                    && (hy - ly - field::TERRAIN_CELL).abs() < 1e-3
                {
                    square += 1;
                }
                let (mut u0, mut v0, mut u1, mut v1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
                for uv in &quad.uv {
                    u0 = u0.min(uv.0);
                    v0 = v0.min(uv.1);
                    u1 = u1.max(uv.0);
                    v1 = v1.max(uv.1);
                }
                if (u1 - u0 - 0.125).abs() < 1e-4
                    && (v1 - v0 - 0.25).abs() < 1e-4
                    && (0.0..=1.0).contains(&u0)
                    && (0.0..=1.0).contains(&v1)
                {
                    one_cell += 1;
                }
                if ((lx + hx) * 0.5).abs() < 1e-3 && ((ly + hy) * 0.5).abs() < 1e-3 {
                    centred += 1;
                }
                if quad.at.0 >= record.min[0]
                    && quad.at.0 <= record.max[0]
                    && quad.at.1 >= record.min[2]
                    && quad.at.1 <= record.max[2]
                {
                    inside += 1;
                }
                let (col, row) = (
                    (quad.at.0 - record.min[0] - field::TERRAIN_CELL * 0.5) / field::TERRAIN_CELL,
                    (quad.at.1 - record.min[2] - field::TERRAIN_CELL * 0.5) / field::TERRAIN_CELL,
                );
                if (col - col.round()).abs() < 0.01 && (row - row.round()).abs() < 0.01 {
                    on_cell += 1;
                }
            }
        }
    }

    eprintln!(
        "{records} records carry {blocks} group blocks and {pieces} placed pieces: {square} are exactly one 128-unit cell, {one_cell} cover exactly one atlas cell, {centred} are centred on their own origin, {inside} sit inside their record, {on_cell} land on a cell centre, {closed} blocks close inside their record"
    );
    assert_eq!(
        pieces, layout.derived.group_entries as usize,
        "every entry is one quad"
    );
    assert!(
        pieces > 190_000,
        "the corpus carries {pieces} placed pieces"
    );
    assert_eq!(
        square, pieces,
        "every placed piece must be one 128-unit cell"
    );
    assert_eq!(
        one_cell, pieces,
        "every placed piece must cover one atlas cell"
    );
    assert!(
        centred as f32 / pieces as f32 > 0.9999,
        "a placed piece is authored about its own origin ({centred}/{pieces})"
    );
    assert_eq!(
        inside, pieces,
        "a piece may not leave the record it belongs to"
    );
    assert_eq!(
        closed, blocks,
        "every group block must close inside its record"
    );
    assert!(
        on_cell * 5 > pieces * 2,
        "about half pin to a cell centre, the rest sit a fraction off ({on_cell}/{pieces})"
    );
}

#[test]
#[ignore]
fn every_ground_piece_carries_one_colour_pair() {
    use std::collections::BTreeSet;

    let Some(path) = corpus() else {
        eprintln!("no corpus, set CORPUS_UFB");
        return;
    };
    let bytes = std::fs::read(&path).expect("read ufb");
    let layout = field::parse(&bytes).expect("layout");

    let mut pieces = 0usize;
    let mut flat = 0usize;
    let mut opaque_tint = 0usize;
    let mut no_fog = 0usize;
    let mut tints: BTreeSet<u32> = BTreeSet::new();
    let mut fogs: BTreeSet<u32> = BTreeSet::new();

    for record in &layout.records {
        for block in &record.groups {
            let count = block.count as usize;
            let colours = block.at + field::GROUP_HEAD + count * field::GROUP_TAIL;
            for (index, quad) in field::ground_quads(&bytes, block).into_iter().enumerate() {
                pieces += 1;
                let at = colours + index * field::GROUP_COLOUR;
                let tint = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
                let fog = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap());
                let mut same = 0;
                for vertex in 1..4 {
                    let copy = at + vertex * 8;
                    if bytes[copy..copy + 8] == bytes[at..at + 8] {
                        same += 1;
                    }
                }
                if same == 3 {
                    flat += 1;
                }
                assert_eq!(quad.tint, tint, "the reader takes the first copy");
                assert_eq!(quad.fog, fog, "the reader takes the second word");
                if tint >> 24 == 0xff {
                    opaque_tint += 1;
                }
                if fog & 0x00ff_ffff == 0x00ff_ffff && fog >> 24 == 0 {
                    no_fog += 1;
                }
                tints.insert(tint);
                fogs.insert(fog);
            }
        }
    }

    eprintln!(
        "{pieces} ground pieces: {flat} write the same colour pair for all four vertices; {opaque_tint} tints are opaque; {no_fog} are the white no-fog marker; {} tints and {} fogs in use",
        tints.len(),
        fogs.len()
    );
    assert!(
        pieces > 190_000,
        "the corpus carries {pieces} placed pieces"
    );
    assert_eq!(
        flat, pieces,
        "a colour pair is flat across the four vertices it is written for"
    );
    assert_eq!(
        opaque_tint, pieces,
        "the tint is an RGBA colour with alpha 255"
    );
    assert!(
        tints.len() <= 8,
        "the tint is a palette, {} values seen",
        tints.len()
    );
    assert!(
        fogs.len() <= 64,
        "the fog is a palette, {} values seen",
        fogs.len()
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

#[test]
#[ignore]
fn playback_reproduces_a_full_render_frame_by_frame() {
    let Some(path) = corpus() else {
        eprintln!("no corpus, set CORPUS_UFB");
        return;
    };
    let dir = path.parent().expect("directory").to_path_buf();
    let bytes = std::fs::read(&path).expect("read ufb");
    let layout = field::parse(&bytes).expect("layout");
    let load = |pair: &field::Pair| field::read_pair(&dir, pair);

    let (scope, _, x0) = field::default_scope(&layout, 1920.0).expect("a layer set");
    let started = std::time::Instant::now();
    let play = field::Playback::prepare(
        &bytes,
        &layout,
        &scope,
        x0,
        1920.0,
        &load,
        1200,
        field::Render::default(),
    )
    .expect("prepared");
    let prepare = started.elapsed().as_secs_f64() * 1000.0;
    assert!(play.moving > 0, "the window should carry moving props");

    let mut worst = 0i32;
    let mut frames_ms = Vec::new();
    for step in 0..4u64 {
        let clock = step * 15_000;
        let started = std::time::Instant::now();
        let fast = play.frame(clock);
        frames_ms.push(started.elapsed().as_secs_f64() * 1000.0);
        let exact = field::compose_window_with(
            &bytes,
            &layout,
            &scope,
            x0,
            1920.0,
            &load,
            1200,
            field::Render {
                clock,
                terrain: false,
            },
        )
        .expect("exact");
        assert_eq!(fast.raster.width, exact.raster.width);
        assert_eq!(fast.raster.height, exact.raster.height);
        for (a, b) in fast.raster.rgba.iter().zip(exact.raster.rgba.iter()) {
            worst = worst.max((*a as i32 - *b as i32).abs());
        }
    }
    eprintln!(
        "playback of {scope}: {prepare:.0} ms to prepare, {} bands over {} moving props, {:.1} MB, frames {frames_ms:?}, worst channel {worst}",
        play.moving,
        play.moving,
        play.footprint() as f64 / (1024.0 * 1024.0)
    );
    assert!(
        worst <= 8,
        "a playback frame must match the full render up to rounding (worst {worst})"
    );
    assert!(
        play.footprint() < 200 << 20,
        "the band set must stay small ({})",
        play.footprint()
    );
}
