use std::path::{Path, PathBuf};
use std::process::ExitCode;

use archive::CpkArchive;

fn main() -> ExitCode {
    let paths: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    if paths.is_empty() {
        eprintln!("usage: cpk_probe <archive.cpk>...");
        return ExitCode::from(2);
    }

    let mut bad = 0u32;
    for path in &paths {
        match report(path) {
            Ok(failures) => bad += failures,
            Err(error) => {
                println!("{} FAIL {error}", path.display());
                bad += 1;
            }
        }
    }

    if bad > 0 {
        println!("bad={bad}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn report(path: &Path) -> Result<u32, archive::Error> {
    let archive = CpkArchive::open(path)?;
    let summary = archive.summary();

    println!(
        "{} entries={} dirs={} compressed={} total={:.2}GB toc={} content={} tocFromToc={} enc=header:{} toc:{} padding={}/{}",
        path.file_name().unwrap_or(path.as_os_str()).to_string_lossy(),
        summary.entries,
        summary.directories,
        summary.compressed,
        summary.total_extract as f64 / 1024f64.powi(3),
        summary.toc_offset,
        summary.content_offset,
        summary.content_base_is_toc,
        summary.header_encrypted,
        summary.toc_encrypted,
        summary.header_padding,
        summary.toc_padding
    );

    let mut bad = 0u32;
    if summary.entries as u64 != summary.declared_files {
        println!(
            "  [FAIL] entries={} declared={}",
            summary.entries, summary.declared_files
        );
        bad += 1;
    }
    if summary.out_of_range != 0 {
        println!("  [FAIL] out_of_range={}", summary.out_of_range);
        bad += 1;
    }
    if summary.header_padding != 0 || summary.toc_padding != 0 {
        println!(
            "  [FAIL] padding header={} toc={}",
            summary.header_padding, summary.toc_padding
        );
        bad += 1;
    }

    println!(
        "  version={} revision={} align={} magic={:?}",
        summary.version, summary.revision, summary.align, summary.magic
    );

    for dir in archive.child_dirs("") {
        let count = archive
            .entries()
            .iter()
            .filter(|entry| entry.dir == dir || entry.dir.starts_with(&format!("{dir}/")))
            .count();
        println!("  dir {dir} entries={count}");
    }

    Ok(bad)
}
