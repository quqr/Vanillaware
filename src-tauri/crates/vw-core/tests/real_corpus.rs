//! 真实语料的冒烟验证。默认不跑：语料只在本机存在。
//!
//! ```text
//! cargo test -p vw-core --test real_corpus -- --ignored --nocapture
//! ```

use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use vw_core::{build_index, list_dir, Kind};

const CORPUS: &str = r"F:\Codes\UNICORN OVERLORD\extract";

#[test]
#[ignore = "需要本机 UNICORN OVERLORD 解包语料"]
fn listing_returns_only_native_resources() {
    let root = Path::new(CORPUS);
    if !root.is_dir() {
        eprintln!("跳过：语料不存在 {}", root.display());
        return;
    }

    let t = Instant::now();
    let listing = list_dir(root).unwrap();
    eprintln!("list_dir 根目录耗时：{:?}", t.elapsed());
    eprintln!(
        "子目录 {} / 收录 {} / 未收录 {} / 警告 {}",
        listing.dirs.len(),
        listing.files.len(),
        listing.hidden_files,
        listing.warnings.len()
    );

    assert!(!listing.dirs.is_empty(), "根下应有子目录");
    for file in &listing.files {
        assert!(
            matches!(file.kind, Kind::Texture | Kind::Animation | Kind::Binary),
            "{} 的类别不在原生资源范围内",
            file.name
        );
        assert!(!matches!(
            file.ext.as_str(),
            "png" | "json" | "txt" | "quad"
        ));
    }
}

#[test]
#[ignore = "需要本机 UNICORN OVERLORD 解包语料"]
fn content_directory_reports_textures_and_animations() {
    let dir = Path::new(CORPUS).join("000").join("GUI");
    if !dir.is_dir() {
        eprintln!("跳过：{} 不存在", dir.display());
        return;
    }

    let listing = list_dir(&dir).unwrap();
    let textures = listing
        .files
        .iter()
        .filter(|f| f.kind == Kind::Texture)
        .count();
    let animations = listing
        .files
        .iter()
        .filter(|f| f.kind == Kind::Animation)
        .count();
    eprintln!(
        "{}：纹理 {} / 动画 {} / 未收录 {}",
        dir.display(),
        textures,
        animations,
        listing.hidden_files
    );

    assert!(textures > 0, "GUI 目录应有纹理");
    assert!(animations > 0, "GUI 目录应有动画");
    assert_eq!(
        textures + animations,
        listing.files.len(),
        "不应混入其他类别"
    );
}

#[test]
#[ignore = "需要本机 UNICORN OVERLORD 解包语料"]
fn derived_output_directory_indexes_to_nothing() {
    let dir = Path::new(CORPUS).join("Main-Output").join("quads");
    if !dir.is_dir() {
        eprintln!("跳过：{} 不存在", dir.display());
        return;
    }

    let cancel = AtomicBool::new(false);
    let t = Instant::now();
    let entries = build_index(&dir, &cancel, |_| {}).unwrap();
    eprintln!("索引 {} 耗时：{:?}", dir.display(), t.elapsed());
    eprintln!("{} 下的索引条目：{}", dir.display(), entries.len());
    assert!(entries.is_empty(), "只含 .quad.json 的目录不应收录任何资源");
}
