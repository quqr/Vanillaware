use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use vfs::Mount;

fn corpora() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(extra) = std::env::var("CPK_CORPUS") {
        paths.push(PathBuf::from(extra));
    }
    paths.push(PathBuf::from(r"F:\Codes\13\dlc\1\ROBO_DLC_CN.CPK"));
    paths.push(PathBuf::from(r"F:\Codes\13\main\1\ROBO_CN.CPK"));
    paths.retain(|path| path.is_file());
    paths
}

#[test]
#[ignore = "walks the real game archives, run with --ignored"]
fn a_real_archive_lists_and_reads_as_a_directory() {
    let archives = corpora();
    if archives.is_empty() {
        eprintln!("skipped: no CPK corpus found, set CPK_CORPUS");
        return;
    }

    let mount = Mount::new(None);
    for cpk in &archives {
        let root = cpk.to_string_lossy().into_owned();
        let listing = mount.list(&root).expect("list archive root");
        assert!(!listing.dirs.is_empty(), "{root} has no directories");

        let first = &listing.dirs[0];
        let nested = mount.list(&first.path).expect("list nested directory");
        assert_eq!(nested.parent.as_deref(), Some(root.as_str()));
        assert!(
            !nested.files.is_empty(),
            "{} has no catalogued files",
            first.path
        );

        let mut read = 0;
        for file in nested.files.iter().take(3) {
            let data = mount.read(&file.path).expect("read entry");
            assert_eq!(data.len() as u64, file.size, "{}", file.path);
            read += 1;
        }
        eprintln!(
            "{} dirs={} files={} hidden={} read={read}",
            cpk.file_name().unwrap().to_string_lossy(),
            listing.dirs.len(),
            listing.files.len(),
            listing.hidden_files
        );
    }
}

#[test]
#[ignore = "walks the real game archives, run with --ignored"]
fn indexing_a_directory_opens_its_archives() {
    let archives = corpora();
    if archives.is_empty() {
        eprintln!("skipped: no CPK corpus found, set CPK_CORPUS");
        return;
    }

    let mount = Mount::new(None);
    let cancel = AtomicBool::new(false);
    for cpk in &archives {
        let directory = cpk.parent().expect("parent");
        let entries =
            vfs::build_index(&mount, Path::new(directory), &cancel, |_| {}).expect("index");
        let inner = entries
            .iter()
            .filter(|entry| entry.path.contains(".CPK\\") || entry.path.contains(".cpk\\"))
            .count();
        let archive = mount.archive(&cpk.to_string_lossy()).expect("archive");
        eprintln!(
            "{} indexed={} inner={} toc={}",
            cpk.file_name().unwrap().to_string_lossy(),
            entries.len(),
            inner,
            archive.entries().len()
        );
        assert!(
            inner > 0,
            "no archive entries reached the index for {}",
            cpk.display()
        );
    }
}
