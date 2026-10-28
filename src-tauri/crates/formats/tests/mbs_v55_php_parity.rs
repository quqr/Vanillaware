//! Structural parity against the PHP `.mbs.v55` output (see ADR-0006).
//!
//! The reference values come from the upstream PHP implementation; this test only
//! compares. Point `MBS_ORACLE_DIR` at the generated directory:
//!
//! ```text
//! $env:MBS_ORACLE_DIR="<out_dir>"
//! cargo test -p formats --test mbs_v55_php_parity
//! ```
//!
//! Skips when the variable is unset or the directory is missing, like `gold_png.rs`.

use std::path::PathBuf;

use formats::mbs::{self, verify};

#[test]
fn v55_matches_php_oracle() {
    let Ok(dir) = std::env::var("MBS_ORACLE_DIR") else {
        eprintln!("skip: 未设置 MBS_ORACLE_DIR");
        return;
    };
    let dir = PathBuf::from(dir);
    if !dir.is_dir() {
        eprintln!("skip: {} 不是目录", dir.display());
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
    assert!(!samples.is_empty(), "{} 里没有 .mbs 样本", dir.display());

    let mut checked = 0usize;
    let mut mismatched = 0usize;
    let mut lines = Vec::new();

    for mbs_file in &samples {
        let oracle = PathBuf::from(format!("{}.v55", mbs_file.display()));
        if !oracle.is_file() {
            continue;
        }

        let ours = mbs::parse_file(mbs_file, &tag).expect("解析 mbs");
        let ours = serde_json::to_value(&ours).expect("序列化 V55");
        let raw = std::fs::read(&oracle).expect("读取 PHP 真值");
        let theirs: serde_json::Value = serde_json::from_slice(&raw).expect("解析 PHP 真值 JSON");

        let mut diffs = Vec::new();
        verify::diff_values(&theirs, &ours, "", &mut diffs);
        checked += 1;

        if !diffs.is_empty() {
            mismatched += 1;
            lines.push(format!("{}: {} 处差异", mbs_file.display(), diffs.len()));
            for diff in diffs.iter().take(10) {
                lines.push(format!("    {diff}"));
            }
        }
    }

    assert!(checked > 0, "没有找到任何 <x>.mbs 与 <x>.mbs.v55 配对");
    eprintln!("比对 {checked} 个文件，{mismatched} 个有差异");
    assert!(
        lines.is_empty(),
        "与 PHP 真值不一致：\n{}",
        lines.join("\n")
    );
}
