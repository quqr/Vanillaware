use std::path::Path;

use formats::document::resolve::*;

#[test]
fn resolves_same_directory_first() {
    let dir = std::env::temp_dir().join("resolver-test");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("Target.mbs");
    std::fs::write(&file, b"x").unwrap();

    let (path, ok) = Resolver::new(&dir).resolve("Target.mbs");
    assert!(ok);
    assert_eq!(path, file);
}

#[test]
fn missing_name_reports_false_without_panicking() {
    let (_, ok) = Resolver::new(Path::new(".")).resolve("no-such-file.mbs");
    assert!(!ok);
}
