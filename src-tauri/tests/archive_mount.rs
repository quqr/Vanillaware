use std::path::PathBuf;

use vanillaware_lib::state::AppState;

fn archive() -> Option<PathBuf> {
    let candidates = [
        std::env::var("CPK_CORPUS").map(PathBuf::from).ok(),
        Some(PathBuf::from(r"F:\Codes\13\dlc\1\ROBO_DLC_CN.CPK")),
    ];
    candidates.into_iter().flatten().find(|path| path.is_file())
}

#[test]
fn archive_entries_are_reachable_through_the_app_state() {
    let Some(cpk) = archive() else {
        eprintln!("skip: no CPK archive found, set CPK_CORPUS");
        return;
    };

    let spill = tempfile::tempdir().expect("temp dir");
    let thumbs = tempfile::tempdir().expect("temp dir");
    let state = AppState::new(thumbs.path().to_path_buf(), spill.path().to_path_buf());

    let root = cpk.to_string_lossy().into_owned();
    let listing = state.mount.list(&root).expect("list archive root");
    assert!(!listing.dirs.is_empty());

    let first = &listing.dirs[0];
    let nested = state.mount.list(&first.path).expect("list nested dir");
    assert!(
        !nested.files.is_empty(),
        "{} holds no catalogued files",
        first.path
    );

    let texture = nested
        .files
        .iter()
        .find(|file| file.ext == "ftx")
        .expect("no ftx in the first directory");

    let bytes = state.mount.read(&texture.path).expect("read ftx");
    assert_eq!(bytes.len() as u64, texture.size);

    let frames = formats::ftx::probe_bytes(&bytes).expect("probe ftx");
    assert!(!frames.is_empty());

    let stat = state.mount.stat(&texture.path).expect("stat");
    assert!(!stat.is_dir);
    assert_eq!(stat.size, texture.size);

    let meta = state.mount.file_meta(&texture.path).expect("meta");
    assert_eq!(meta.entry.name, texture.name);

    let real = state.mount.real(&texture.path).expect("materialize");
    assert!(real.is_file());
    assert_eq!(std::fs::read(&real).expect("read spill").len(), bytes.len());
}
