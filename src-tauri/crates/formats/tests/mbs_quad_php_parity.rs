//! ```text
//! $env:MBS_ORACLE_DIR="<oracle_dir>"
//! $env:MBS_TAG="swi_unic"
//! cargo test -p formats --test mbs_quad_php_parity -- --nocapture
//! ```

use std::path::PathBuf;

use formats::mbs::{self, verify};

#[test]
fn quad_matches_php_oracle() {
    let Ok(dir) = std::env::var("MBS_ORACLE_DIR") else {
        eprintln!("skip: MBS_ORACLE_DIR is not set");
        return;
    };
    let dir = PathBuf::from(dir);
    if !dir.is_dir() {
        eprintln!("skip: {} is not a directory", dir.display());
        return;
    }

    let tag = std::env::var("MBS_TAG").unwrap_or_else(|_| "swi_unic".to_string());
    let mut samples: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("read oracle dir")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "mbs"))
        .collect();
    samples.sort();

    let mut checked = 0usize;
    let mut mismatched = 0usize;
    let mut lines = Vec::new();

    for mbs_file in &samples {
        let oracle = PathBuf::from(format!("{}.v55.quad", mbs_file.display()));
        if !oracle.is_file() {
            continue;
        }

        let v55 = mbs::parse_file(mbs_file, &tag).expect("parse mbs");
        let ours = mbs::convert(&v55).expect("convert V55 to Quad");
        let ours = serde_json::to_value(&ours).expect("serialise Quad");
        let raw = std::fs::read(&oracle).expect("read the PHP reference");
        let theirs: serde_json::Value = serde_json::from_slice(&raw).expect("parse the PHP reference JSON");

        let mut diffs = Vec::new();
        verify::diff_values(&theirs, &ours, "", &mut diffs);
        checked += 1;

        if !diffs.is_empty() {
            mismatched += 1;
            lines.push(format!("{}: {} differences", mbs_file.display(), diffs.len()));
            for diff in diffs.iter().take(10) {
                lines.push(format!("    {diff}"));
            }
        }
    }

    if checked == 0 {
        eprintln!("skip: no <x>.mbs paired with <x>.mbs.v55.quad");
        return;
    }
    eprintln!("compared {checked} files, {mismatched} differ");
    assert!(
        lines.is_empty(),
        "differs from the PHP reference:\n{}",
        lines.join("\n")
    );
}
