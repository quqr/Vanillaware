//! ```text
//! cargo test -p catalog --test real_corpus -- --ignored --nocapture
//! ```

use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use catalog::{build_index, list_dir, Kind};

const CORPUS: &str = r"F:\Codes\UNICORN OVERLORD\extract";

#[test]
#[ignore = "requires the local UNICORN OVERLORD extraction"]
fn listing_returns_only_native_resources() {
    let root = Path::new(CORPUS);
    if !root.is_dir() {
        eprintln!("skipped: corpus missing {}", root.display());
        return;
    }

    let t = Instant::now();
    let listing = list_dir(root).unwrap();
    eprintln!("list_dir root took {:?}", t.elapsed());
    eprintln!(
        "dirs {} / catalogued {} / skipped {} / warnings {}",
        listing.dirs.len(),
        listing.files.len(),
        listing.hidden_files,
        listing.warnings.len()
    );

    assert!(!listing.dirs.is_empty(), "root must contain subdirectories");
    for file in &listing.files {
        assert!(
            !file.kind.icon().is_empty(),
            "{} has a kind outside the native resource set",
            file.name
        );
        assert!(!matches!(
            file.ext.as_str(),
            "png" | "json" | "txt" | "quad"
        ));
    }
}

#[test]
#[ignore = "requires the local UNICORN OVERLORD extraction"]
fn content_directory_reports_textures_and_animations() {
    let dir = Path::new(CORPUS).join("000").join("GUI");
    if !dir.is_dir() {
        eprintln!("skipped: {} does not exist", dir.display());
        return;
    }

    let listing = list_dir(&dir).unwrap();
    let textures = listing
        .files
        .iter()
        .filter(|f| f.kind == Kind::Texture)
        .count();
    let animations = listing
        .files
        .iter()
        .filter(|f| f.kind == Kind::Animation)
        .count();
    eprintln!(
        "{}: textures {} / animations {} / skipped {}",
        dir.display(),
        textures,
        animations,
        listing.hidden_files
    );

    assert!(textures > 0, "GUI must contain textures");
    assert!(animations > 0, "GUI must contain animations");
    assert!(
        textures + animations <= listing.files.len(),
        "texture and animation counts must not exceed the total"
    );
}

#[test]
#[ignore = "requires the local UNICORN OVERLORD extraction"]
fn derived_output_directory_indexes_to_nothing() {
    let dir = Path::new(CORPUS).join("Main-Output").join("quads");
    if !dir.is_dir() {
        eprintln!("skipped: {} does not exist", dir.display());
        return;
    }

    let cancel = AtomicBool::new(false);
    let t = Instant::now();
    let entries = build_index(&dir, &cancel, |_| {}).unwrap();
    eprintln!("index {} took {:?}", dir.display(), t.elapsed());
    eprintln!("index entries under {}: {}", dir.display(), entries.len());
    assert!(
        entries.is_empty(),
        "a directory of .quad.json files must catalogue nothing"
    );
}
