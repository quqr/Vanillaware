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
        "{} / {} cells identical, {} painted, {} clipped, {} open, {} dormant, first {:?}",
        matched.same,
        matched.cells,
        matched.painted,
        matched.clipped,
        matched.open,
        matched.dormant,
        matched.first
    );
    assert_eq!(matched.clipped, 0, "no mask cell may fall outside the grid");
    assert_eq!(matched.dormant, 8, "eight masks are flagged 0x4700");
    assert_eq!(
        matched.same, matched.cells,
        "first mismatch: {:?}",
        matched.first
    );
}
