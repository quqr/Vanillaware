use catalog::game::*;
use std::fs;

#[test]
fn detects_unicorn_overlord_by_markers() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("Script")).unwrap();
    fs::write(dir.path().join("a.asb"), b"x").unwrap();

    let profile = detect(dir.path());
    assert_eq!(profile.id, GameId::UnicornOverlord);
    assert_eq!(profile.mbs_tag, Some("swi_unic"));
}

#[test]
fn detects_thirteen_sentinels_by_markers() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("Stage")).unwrap();
    fs::write(dir.path().join("s.osb"), b"x").unwrap();

    let profile = detect(dir.path());
    assert_eq!(profile.id, GameId::ThirteenSentinels);
    assert_eq!(profile.mbs_tag, Some("swi_sent"));
}

#[test]
fn detects_leaf_directory_through_ancestor() {
    let dir = tempfile::tempdir().unwrap();
    let leaf = dir.path().join("EX DATA").join("Stage");
    fs::create_dir_all(&leaf).unwrap();
    fs::write(leaf.join("s.osb"), b"x").unwrap();

    assert_eq!(detect(&leaf).id, GameId::ThirteenSentinels);
}

#[test]
fn empty_directory_is_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let profile = detect(dir.path());
    assert_eq!(profile.id, GameId::Unknown);
    assert_eq!(profile.mbs_tag, None);
    assert_eq!(profile.confidence, 0);
}
