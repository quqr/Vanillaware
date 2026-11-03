//! Structural parity against the PHP `.mbs.v55` output (see ADR-0006).
//! The reference values come from the upstream PHP implementation; this test only
//! compares. Point `MBS_ORACLE_DIR` at the generated directory:
//! ```text
//! $env:MBS_ORACLE_DIR="<out_dir>"
//! cargo test -p formats --test mbs_v55_php_parity
//! ```
//! Skips when the variable is unset or the directory is missing, like `gold_png.rs`.

use std::path::PathBuf;

use formats::mbs::{self, verify};

#[test]
fn v55_matches_php_oracle() {
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
    assert!(
        !samples.is_empty(),
        "no .mbs sample under {}",
        dir.display()
    );

    let mut checked = 0usize;
    let mut mismatched = 0usize;
    let mut lines = Vec::new();

    for mbs_file in &samples {
        let oracle = PathBuf::from(format!("{}.v55", mbs_file.display()));
        if !oracle.is_file() {
            continue;
        }

        let ours = mbs::parse_file(mbs_file, &tag).expect("parse mbs");
        let ours = serde_json::to_value(&ours).expect("serialise V55");
        let raw = std::fs::read(&oracle).expect("read the PHP reference");
        let theirs: serde_json::Value =
            serde_json::from_slice(&raw).expect("parse the PHP reference JSON");

        let mut diffs = Vec::new();
        verify::diff_values(&theirs, &ours, "", &mut diffs);
        checked += 1;

        if !diffs.is_empty() {
            mismatched += 1;
            lines.push(format!(
                "{}: {} differences",
                mbs_file.display(),
                diffs.len()
            ));
            for diff in diffs.iter().take(10) {
                lines.push(format!("    {diff}"));
            }
        }
    }

    assert!(checked > 0, "no <x>.mbs paired with <x>.mbs.v55");
    eprintln!("compared {checked} files, {mismatched} differ");
    assert!(
        lines.is_empty(),
        "differs from the PHP reference:\n{}",
        lines.join("\n")
    );
}
