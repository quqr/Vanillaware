use std::fs;
use std::sync::atomic::AtomicBool;

use catalog::{build_index, file_meta, list_dir, search_index, Kind, SEARCH_LIMIT};

fn touch(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, b"x").unwrap();
}

#[test]
fn listing_keeps_catalogued_files_and_counts_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    touch(&root.join("Hero.ftx"));
    touch(&root.join("Hero.mbs"));
    touch(&root.join("Enemy.asb"));
    touch(&root.join("Hero.png"));
    touch(&root.join("Hero.quad.json"));
    touch(&root.join("readme.txt"));

    let listing = list_dir(root).unwrap();
    let names: Vec<&str> = listing.files.iter().map(|f| f.name.as_str()).collect();

    assert_eq!(names, vec!["Enemy.asb", "Hero.ftx", "Hero.mbs"]);
    assert_eq!(listing.hidden_files, 3);
    assert!(listing.warnings.is_empty());
}

#[test]
fn listing_reports_every_subdirectory_even_when_empty() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    fs::create_dir_all(root.join("empty")).unwrap();
    fs::create_dir_all(root.join("only_png")).unwrap();
    touch(&root.join("only_png").join("a.png"));

    let listing = list_dir(root).unwrap();
    let dirs: Vec<&str> = listing.dirs.iter().map(|d| d.name.as_str()).collect();

    assert_eq!(dirs, vec!["empty", "only_png"]);
    assert!(listing.files.is_empty());
}

#[test]
fn kind_and_viewers_come_from_the_registry() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    touch(&root.join("Hero.ftx"));
    touch(&root.join("Hero.mbs"));

    let listing = list_dir(root).unwrap();
    let ftx = listing.files.iter().find(|f| f.ext == "ftx").unwrap();
    let mbs = listing.files.iter().find(|f| f.ext == "mbs").unwrap();

    assert_eq!(ftx.kind, Kind::Texture);
    assert_eq!(ftx.viewers, ["atlas", "doc", "hex"]);
    assert!(ftx.has_thumb);

    assert_eq!(mbs.kind, Kind::Animation);
    assert_eq!(mbs.viewers, ["animation", "doc", "hex"]);
    assert!(!mbs.has_thumb);
}

#[test]
fn related_files_share_base_name_within_the_directory() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    touch(&root.join("Hero.ftx"));
    touch(&root.join("Hero.mbs"));
    touch(&root.join("Hero.png"));
    touch(&root.join("Other.mbs"));

    let meta = file_meta(&root.join("Hero.mbs")).unwrap();
    let names: Vec<&str> = meta.related.iter().map(|r| r.name.as_str()).collect();

    assert_eq!(names, vec!["Hero.ftx"]);
}

#[test]
fn file_meta_rejects_uncatalogued_paths() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shot.png");
    touch(&path);

    assert!(file_meta(&path).is_err());
}

#[test]
fn index_covers_the_whole_tree_but_only_catalogued_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    touch(&root.join("a").join("Hero.mbs"));
    touch(&root.join("b").join("deep").join("Hero.asb"));
    touch(&root.join("b").join("deep").join("Hero.png"));

    let cancel = AtomicBool::new(false);
    let entries = build_index(root, &cancel, |_| {}).unwrap();

    assert_eq!(entries.len(), 2);

    let hit = search_index(&entries, "hero", SEARCH_LIMIT);
    assert_eq!(hit.total, 2);
    assert!(!hit.truncated);
}

#[test]
fn cancelled_index_returns_what_it_has() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    for i in 0..8 {
        touch(&root.join(format!("f{i}.asb")));
    }

    let cancel = AtomicBool::new(true);
    let entries = build_index(root, &cancel, |_| {}).unwrap();
    assert!(entries.is_empty());
}
