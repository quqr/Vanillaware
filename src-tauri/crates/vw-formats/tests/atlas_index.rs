//! 校验 `tex_id` 的下标口径：图集帧数必须等于 PHP 落盘的 `<stem>.<i>.png` 个数。
//!
//! PHP `FTEX_swit_unicorn.php` 按容器内**可解码帧**的顺序导出 `<stem>.<i>.<ext>`，
//! 预览器把 `s4.tex` 当作同一顺序的下标。口径一旦错位，预览会静默画出别的贴图，
//! 故这里用外部产物的数量做一次交叉核对（不比较像素，只比帧数与命名）。
//!
//! ```text
//! $env:VW_MBS_MBS_ROOT="F:\Codes\UNICORN OVERLORD\extract\000"
//! $env:VW_ATLAS_PNG_DIR="F:\Codes\UNICORN OVERLORD\extract\Main-Output\textures"
//! cargo test -p vw-formats --test atlas_index -- --nocapture
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use vw_formats::atlas::Atlas;

/// 样本量：每次只核 24 个 stem，够覆盖 BattleBG / Chara / GUI 三类尺寸分布。
const SAMPLE: usize = 24;

#[test]
fn atlas_frame_count_matches_png_numbering() {
    let Ok(mbs_root) = std::env::var("VW_MBS_MBS_ROOT") else {
        eprintln!("skip: 未设置 VW_MBS_MBS_ROOT");
        return;
    };
    let Ok(png_dir) = std::env::var("VW_ATLAS_PNG_DIR") else {
        eprintln!("skip: 未设置 VW_ATLAS_PNG_DIR");
        return;
    };
    let mbs_root = PathBuf::from(mbs_root);
    let png_dir = PathBuf::from(png_dir);
    if !mbs_root.is_dir() || !png_dir.is_dir() {
        eprintln!("skip: 目录不存在");
        return;
    }

    let expected = png_counts(&png_dir);
    let mut files: Vec<PathBuf> = Vec::new();
    collect_mbs(&mbs_root, &mut files);
    files.sort();
    files.sort_by_key(|path| stable_hash(&path.to_string_lossy()));

    let mut checked = 0usize;
    let mut lines = Vec::new();
    let mut skipped = Vec::new();
    for path in files.iter().take(SAMPLE) {
        let atlas = match Atlas::open(path) {
            Ok(Some(atlas)) => atlas,
            Ok(None) => continue,
            Err(e) => {
                // 例如 `Eltlinde_F` 同时有 `00`/`10` 两个容器：无法确定归属，如实登记为跳过。
                skipped.push(format!("{}: {e}", path.display()));
                continue;
            }
        };

        // png 以**容器**的 stem 命名（`Fighter_M.mbs` ↔ `Fighter_M00.ftx` ↔ `Fighter_M00.0.png`）。
        let Some(png_stem) = atlas
            .path()
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
        else {
            continue;
        };
        let Some(&png_count) = expected.get(&png_stem) else {
            lines.push(format!("{png_stem}: 容器存在但没有对应 png"));
            continue;
        };

        checked += 1;
        if atlas.len() != png_count {
            lines.push(format!(
                "{png_stem}: 容器 {} 帧，png {png_count} 个（{}）",
                atlas.len(),
                atlas.path().display()
            ));
        }
    }

    if checked == 0 {
        eprintln!("skip: 样本里没有可核对的配对");
        return;
    }
    eprintln!("核对 {checked} 个 stem 的图集帧数");
    if !skipped.is_empty() {
        eprintln!("跳过 {} 个（容器归属无法确定）：", skipped.len());
        for line in skipped.iter().take(5) {
            eprintln!("  {line}");
        }
    }
    assert!(
        lines.is_empty(),
        "图集下标口径不一致：\n{}",
        lines.join("\n")
    );
}

/// `<stem>.<i>.png` → `stem → 个数`。
fn png_counts(dir: &Path) -> HashMap<String, usize> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return HashMap::new();
    };
    let mut counts: HashMap<String, usize> = HashMap::new();
    for path in entries.flatten().map(|entry| entry.path()) {
        let Some(name) = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            continue;
        };
        let Some(rest) = name.strip_suffix(".png") else {
            continue;
        };
        let Some((stem, index)) = rest.rsplit_once('.') else {
            continue;
        };
        if index.parse::<usize>().is_ok() {
            *counts.entry(stem.to_string()).or_insert(0) += 1;
        }
    }
    counts
}

fn collect_mbs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.is_dir() {
            collect_mbs(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "mbs") {
            out.push(path);
        }
    }
}

/// FNV-1a：固定种子，保证每次抽到同一批文件。
fn stable_hash(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
