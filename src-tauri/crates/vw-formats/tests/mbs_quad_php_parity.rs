//! Quad ↔ PHP `.mbs.v55.quad` 结构等价比对（判据见 ADR-0006）。
//!
//! 真值由权威 PHP（`rip-vanillaware/v55_to_quad.php`）产出，本测试只负责比较、不负责生成。
//! 生成脚本（仓库外）对每个 `.mbs.v55` 调 `vanilla(0, <v55>)`，产出 `<x>.mbs.v55.quad`。
//! 把 `VW_MBS_ORACLE_DIR` 指向该目录即可：
//!
//! ```text
//! $env:VW_MBS_ORACLE_DIR="<oracle_dir>"
//! $env:VW_MBS_TAG="swi_unic"
//! cargo test -p vw-formats --test mbs_quad_php_parity -- --nocapture
//! ```
//!
//! 未设置该变量、或目录不存在时自动跳过（与 `gold_png.rs` 同样的「缺料即跳过」约定）。

use std::path::PathBuf;

use vw_formats::mbs::{self, verify};

#[test]
fn quad_matches_php_oracle() {
    let Ok(dir) = std::env::var("VW_MBS_ORACLE_DIR") else {
        eprintln!("skip: 未设置 VW_MBS_ORACLE_DIR");
        return;
    };
    let dir = PathBuf::from(dir);
    if !dir.is_dir() {
        eprintln!("skip: {} 不是目录", dir.display());
        return;
    }

    let tag = std::env::var("VW_MBS_TAG").unwrap_or_else(|_| "swi_unic".to_string());
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

        let v55 = mbs::parse_file(mbs_file, &tag).expect("解析 mbs");
        let ours = mbs::convert(&v55).expect("转换 V55 → Quad");
        let ours = serde_json::to_value(&ours).expect("序列化 Quad");
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

    if checked == 0 {
        eprintln!("skip: 没有找到任何 <x>.mbs 与 <x>.mbs.v55.quad 配对");
        return;
    }
    eprintln!("比对 {checked} 个文件，{mismatched} 个有差异");
    assert!(lines.is_empty(), "与 PHP 真值不一致：\n{}", lines.join("\n"));
}
