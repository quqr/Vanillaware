use std::path::PathBuf;

use vfs::{split, Mount, Split};

fn asset() -> PathBuf {
    std::env::var("CRIFS_ASSETS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"F:\Codes\CriFsV2Lib\CriFsV2Lib.Tests\Assets"))
        .join("SampleData.cpk")
}

#[test]
fn split_finds_the_first_archive_component() {
    let cpk = asset();
    let text = cpk.to_string_lossy();

    match split(&text) {
        Split::Archive { cpk: found, inner } => {
            assert_eq!(found, text);
            assert_eq!(inner, "");
        }
        Split::Disk(_) => panic!("the archive root was not recognised"),
    }

    let nested = format!("{text}\\SampleData\\Text-Compressed.txt");
    match split(&nested) {
        Split::Archive { cpk: found, inner } => {
            assert_eq!(found, text);
            assert_eq!(inner, "SampleData/Text-Compressed.txt".replace('/', "\\"));
        }
        Split::Disk(_) => panic!("the nested path was not recognised"),
    }

    match split(r"F:\Codes\Vanillaware\README.md") {
        Split::Disk(path) => assert!(path.ends_with("README.md")),
        Split::Archive { .. } => panic!("a plain path must not be treated as an archive"),
    }
}

#[test]
fn archive_root_keeps_unregistered_extensions_out_of_the_listing() {
    let mount = Mount::new(None);
    let cpk = asset().to_string_lossy().into_owned();

    let listing = mount.list(&cpk).expect("list");
    assert!(listing.dirs.is_empty());
    assert!(
        listing.files.is_empty(),
        "the sample archive only holds flac, jpg and txt, none of which are catalogued"
    );
    assert_eq!(listing.hidden_files, 3);
    assert!(listing.parent.is_some());
}

#[test]
fn entries_read_through_the_mount() {
    let mount = Mount::new(None);
    let cpk = asset().to_string_lossy().into_owned();

    let entry = format!("{cpk}\\Text-Compressed.txt");
    let data = mount.read(&entry).expect("read entry");
    assert_eq!(data.len(), 3592);

    let whole = mount.read(&cpk).expect("read archive");
    assert_eq!(whole.len(), 174824);
    assert_eq!(&whole[..4], b"CPK ");

    let stat = mount.stat(&entry).expect("stat entry");
    assert!(!stat.is_dir);
    assert_eq!(stat.size, 3592);

    let root = mount.stat(&cpk).expect("stat root");
    assert!(root.is_dir);
    assert_eq!(root.size, 174824);

    assert!(mount.stat(&format!("{cpk}\\missing.bin")).is_err());
}

#[test]
fn peers_and_walk_stay_inside_the_archive() {
    let mount = Mount::new(None);
    let cpk = asset().to_string_lossy().into_owned();

    let mut peers = mount.peers(&format!("{cpk}\\Text-Compressed.txt"));
    peers.sort();
    assert_eq!(peers.len(), 3);
    assert_eq!(peers[2], "Text-Compressed.txt");

    let names = mount.walk_names(&cpk, 1);
    assert_eq!(names.len(), 3);
}

#[test]
fn a_cpk_on_disk_is_promoted_to_a_directory_entry() {
    let temporary = tempfile::tempdir().expect("temp dir");
    let copied = temporary.path().join("SampleData.cpk");
    std::fs::copy(asset(), &copied).expect("copy fixture");

    let mount = Mount::new(None);
    let listing = mount
        .list(&temporary.path().to_string_lossy())
        .expect("list");

    assert!(
        listing.files.is_empty(),
        "the archive must not stay in the file list"
    );
    assert_eq!(listing.dirs.len(), 1);
    assert_eq!(listing.dirs[0].name, "SampleData.cpk");
    assert_eq!(
        listing.dirs[0].kind,
        Some(catalog::Kind::Archive),
        "the mount point must be marked as an archive"
    );

    let inside = format!("{}\\Text-Compressed.txt", listing.dirs[0].path);
    assert_eq!(mount.read(&inside).expect("read").len(), 3592);
}

#[test]
fn index_walks_into_mounted_archives() {
    let temporary = tempfile::tempdir().expect("temp dir");
    let copied = temporary.path().join("SampleData.cpk");
    std::fs::copy(asset(), &copied).expect("copy fixture");

    let mount = Mount::new(None);
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let entries = vfs::build_index(&mount, temporary.path(), &cancel, |_| {}).expect("index");

    assert_eq!(
        entries.len(),
        1,
        "only the archive itself is catalogued, its payload is not"
    );
    assert!(entries[0].path.ends_with("SampleData.cpk"));
}

#[test]
fn indexing_an_archive_root_walks_its_table_of_contents() {
    let mount = Mount::new(None);
    let cpk = asset().to_string_lossy().into_owned();

    assert!(vfs::is_archive_path(&cpk));
    assert!(!vfs::is_archive_path("F:/Codes/Vanillaware/README.md"));

    let entries = vfs::index_archive(&mount, &cpk).expect("index archive");
    assert_eq!(
        entries.len(),
        1,
        "the sample payload is not catalogued, so only the archive itself is indexed"
    );
    assert!(entries[0].path.ends_with("SampleData.cpk"));
}
