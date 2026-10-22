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

fn walk(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, ext, out);
        } else if path
            .extension()
            .is_some_and(|value| value.eq_ignore_ascii_case(ext))
        {
            out.push(path);
        }
    }
}

fn check_ok(arena: &doc::DocArena, check: &str) -> Option<bool> {
    arena
        .doc(0, String::new(), String::new(), String::new(), 0, 0)
        .checks
        .iter()
        .find(|item| item.id == check)
        .map(|item| item.ok)
}

fn assert_corpus(ext: &str, ids: &[&str]) {
    let root = corpus("VW_CORPUS_13", r"F:\Codes\13\13rim ex\EX DATA");
    let mut files = Vec::new();
    walk(&root, ext, &mut files);
    if files.is_empty() {
        eprintln!("跳过：{} 下没有 .{ext}", root.display());
        return;
    }

    let mut failed = 0;
    for path in &files {
        let arena = doc::parse_file(path, None).expect("doc parse");
        for id in ids {
            if check_ok(&arena, id) == Some(false) {
                failed += 1;
                eprintln!("{} 未通过 {}: {}", path.display(), id, path.display());
                break;
            }
        }
    }
    eprintln!(".{ext}: {} 个，失败 {failed}", files.len());
    assert_eq!(failed, 0);
}

#[test]
#[ignore = "需要本机 13 Sentinels 语料"]
fn fms_corpus_is_closed() {
    assert_corpus("fms", &["fms.trailer", "fms.payload"]);
}

#[test]
#[ignore = "需要本机 13 Sentinels 语料"]
fn rsc_corpus_is_closed() {
    assert_corpus("rsc", &["rsc.table", "rsc.payloadStart"]);
}

#[test]
#[ignore = "需要本机 13 Sentinels 语料"]
fn otm_corpus_is_closed() {
    assert_corpus(
        "otm",
        &["otm.magic", "otm.size", "otm.offB", "otm.offC", "otm.sections"],
    );
}

#[test]
#[ignore = "需要本机 13 Sentinels 语料"]
fn mpd_corpus_is_closed() {
    assert_corpus("mpd", &["mpd.magic", "mpd.size", "mpd.payload", "mpd.offset"]);
}

#[test]
#[ignore = "需要本机 13 Sentinels 语料"]
fn ctb_corpus_is_closed() {
    assert_corpus(
        "ctb",
        &[
            "ctb.clips",
            "ctb.sec1",
            "ctb.sec2",
            "ctb.sec3",
            "ctb.sec4",
            "ctb.sec5",
            "ctb.sec6",
        ],
    );
}

#[test]
#[ignore = "需要本机 13 Sentinels 语料"]
fn fmd_corpus_is_closed() {
    assert_corpus("fmd", &["fmd.closed", "fmd.versionGate"]);
}

#[test]
#[ignore = "需要本机 13 Sentinels 语料"]
fn fmd_mesh_count_matches_python() {
    let root = corpus("VW_CORPUS_13", r"F:\Codes\13\13rim ex\EX DATA");
    let mut files = Vec::new();
    walk(&root, "fmd", &mut files);
    if files.is_empty() {
        eprintln!("跳过：{} 下没有 .fmd", root.display());
        return;
    }

    let mut meshes = 0usize;
    let mut declared = 0u64;
    let mut parsed = 0usize;
    for path in &files {
        let arena = doc::parse_file(path, None).expect("fmd parse");
        meshes += arena.count_field_value("nonEmpty", "true");
        parsed += arena.count_field_value("nonEmpty", "true")
            + arena.count_field_value("nonEmpty", "false");
        declared += arena.sum_field("meshCount");
    }
    eprintln!(
        ".fmd: {} 个文件，声明网格 {declared}，解析 {parsed}，非空 {meshes}",
        files.len()
    );

    assert_eq!(declared, parsed as u64, "声明网格数与解析网格数不一致");
    if let Ok(expected) = std::env::var("VW_FMD_MESHES") {
        assert_eq!(meshes, expected.parse::<usize>().unwrap());
    }
    assert!(meshes > 0);
}

#[test]
#[ignore = "需要本机 13 Sentinels 语料"]
fn fsb_corpus_is_closed() {
    assert_corpus("fsb", &["fsb.closed", "fsb.sequence", "fsb.version"]);
}

#[test]
#[ignore = "需要本机 UO 语料"]
fn asb_corpus_parses_to_eof() {
    let dir = corpus(
        "VW_CORPUS_ASB",
        r"F:\Codes\UNICORN OVERLORD\extract\000\Script",
    );
    let files = files(&dir, "asb");
    if files.is_empty() {
        eprintln!("跳过：{} 下没有 .asb", dir.display());
        return;
    }

    let mut failed = 0usize;
    let mut enums = 0u64;
    let mut functions = 0u64;
    let mut globals = 0u64;
    let mut strings = 0u64;
    for path in &files {
        match doc::parse_file(path, None) {
            Ok(arena) if check_ok(&arena, "asb.eof") == Some(true) => {
                enums += arena.sum_field("enums");
                functions += arena.sum_field("functions") + arena.sum_field("globalFuncs");
                globals += arena.sum_field("globals");
                strings += arena.sum_field("strings");
            }
            Ok(_) => {
                failed += 1;
                eprintln!("未到 EOF: {}", path.display());
            }
            Err(error) => {
                failed += 1;
                eprintln!("解析失败 {}: {error}", path.display());
            }
        }
    }
    eprintln!(
        ".asb: {} 个，失败 {failed}；enum {enums} / func {functions} / global {globals} / string {strings}",
        files.len()
    );
    assert_eq!(failed, 0);
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
        if check_ok(&arena, "bbs.closed") != Some(true) {
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
        if check_ok(&arena, "osb.closed") != Some(true) {
            failed += 1;
            eprintln!("未闭合: {}", path.display());
        }
    }
    eprintln!("osb: {} 个，失败 {failed}", files.len());
    assert_eq!(failed, 0);
}
