//! 真实语料的文档树闭合验证（默认忽略）。
//!
//! ```text
//! cargo test -p vw-formats --test doc_corpus -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};

use vw_formats::doc;

fn corpus(env: &str, fallback: &str) -> PathBuf {
    std::env::var(env)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(fallback))
}

fn files(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    read.flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|value| value.eq_ignore_ascii_case(ext)))
        .collect()
}

fn closed(arena: &doc::DocArena, check: &str) -> bool {
    arena
        .doc(0, String::new(), String::new(), String::new(), 0, 0)
        .checks
        .iter()
        .find(|item| item.id == check)
        .is_some_and(|item| item.ok)
}

#[test]
#[ignore = "需要本机 UO 语料"]
fn bbs_corpus_is_closed() {
    let dir = corpus(
        "VW_CORPUS_BBS",
        r"F:\Codes\UNICORN OVERLORD\extract\000\BattleBG",
    );
    let files = files(&dir, "bbs");
    if files.is_empty() {
        eprintln!("跳过：{} 不存在 .bbs", dir.display());
        return;
    }

    let mut failed = 0;
    for path in &files {
        let arena = doc::parse_file(path, None).expect("bbs parse");
        if !closed(&arena, "bbs.closed") {
            failed += 1;
            eprintln!("未闭合: {}", path.display());
        }
    }
    eprintln!("bbs: {} 个，失败 {failed}", files.len());
    assert_eq!(failed, 0);
}

#[test]
#[ignore = "需要本机 13 Sentinels 语料"]
fn osb_corpus_is_closed() {
    let dir = corpus("VW_CORPUS_OSB", r"F:\Codes\13\13rim ex\EX DATA\Stage");
    let files = files(&dir, "osb");
    if files.is_empty() {
        eprintln!("跳过：{} 不存在 .osb", dir.display());
        return;
    }

    let mut failed = 0;
    for path in &files {
        let arena = doc::parse_file(path, None).expect("osb parse");
        if !closed(&arena, "osb.closed") {
            failed += 1;
            eprintln!("未闭合: {}", path.display());
        }
    }
    eprintln!("osb: {} 个，失败 {failed}", files.len());
    assert_eq!(failed, 0);
}
