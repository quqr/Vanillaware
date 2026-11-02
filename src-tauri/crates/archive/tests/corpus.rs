mod common;

use archive::CpkArchive;

const SAMPLE_LIMIT: usize = 50;
const SAMPLE_MAX_BYTES: u64 = 64 * 1024 * 1024;

#[test]
#[ignore = "walks the real game archives, run with --ignored"]
fn every_real_archive_closes() {
    let archives = common::corpora();
    if archives.is_empty() {
        eprintln!("skipped: no CPK corpus found, set CPK_CORPUS");
        return;
    }

    let mut failures = 0;
    for path in &archives {
        let archive = match CpkArchive::open(path) {
            Ok(archive) => archive,
            Err(error) => {
                eprintln!("FAIL {} open: {error}", path.display());
                failures += 1;
                continue;
            }
        };

        let summary = archive.summary();
        eprintln!(
            "{} entries={} dirs={} compressed={}/{} extract={:.2}GB toc={} content={}",
            path.file_name().unwrap().to_string_lossy(),
            summary.entries,
            summary.directories,
            summary.compressed,
            summary.entries,
            summary.total_extract as f64 / 1024f64.powi(3),
            summary.toc_offset,
            summary.content_offset
        );

        if summary.entries as u64 != summary.declared_files {
            eprintln!(
                "FAIL {} entry count {} != header Files {}",
                path.display(),
                summary.entries,
                summary.declared_files
            );
            failures += 1;
        }
        if summary.out_of_range != 0 {
            eprintln!(
                "FAIL {} {} entries point outside the archive",
                path.display(),
                summary.out_of_range
            );
            failures += 1;
        }
        if summary.toc_padding != 0 || summary.header_padding != 0 {
            eprintln!(
                "FAIL {} unexpected padding header={} toc={}",
                path.display(),
                summary.header_padding,
                summary.toc_padding
            );
            failures += 1;
        }

        let stride = (archive.entries().len() / SAMPLE_LIMIT).max(1);
        let mut checked = 0;
        for index in (0..archive.entries().len()).step_by(stride) {
            let entry = &archive.entries()[index];
            if entry.extract_size > SAMPLE_MAX_BYTES {
                continue;
            }
            let data = match archive.read(index) {
                Ok(data) => data,
                Err(error) => {
                    eprintln!("FAIL {} {}: {error}", path.display(), entry.path());
                    failures += 1;
                    continue;
                }
            };
            if data.len() as u64 != entry.extract_size {
                eprintln!(
                    "FAIL {} {}: extracted {} != declared {}",
                    path.display(),
                    entry.path(),
                    data.len(),
                    entry.extract_size
                );
                failures += 1;
            }
            checked += 1;
        }
        eprintln!("  sampled {checked} entries");
    }

    assert_eq!(failures, 0, "{failures} corpus failures");
}
