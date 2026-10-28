//! 跨文件引用图：合成样本单测 + 真实语料解析率（默认忽略）。
//!
//! ```text
//! cargo test -p formats --test doc_graph -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};

use formats::document;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("graph-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn slot(bytes: &mut [u8], at: usize, text: &str) {
    let raw = text.as_bytes();
    bytes[at..at + raw.len()].copy_from_slice(raw);
}

/// 最小 UBBS：两张名表各 1 条。
fn synthetic_bbs() -> Vec<u8> {
    let mut bytes = vec![0u8; 0x400];
    bytes[..4].copy_from_slice(b"UBBS");
    bytes[0x20..0x22].copy_from_slice(&1u16.to_le_bytes()); // mbs 数
    bytes[0x22..0x24].copy_from_slice(&1u16.to_le_bytes()); // ftx 数
    for (index, offset) in [0x380u64, 0x3a0].iter().enumerate() {
        let at = 0x338 + index * 8;
        bytes[at..at + 8].copy_from_slice(&offset.to_le_bytes());
    }
    slot(&mut bytes, 0x380, "Target.mbs\0");
    slot(&mut bytes, 0x3a0, "Target.ftx\0");
    bytes
}

#[test]
fn bbs_graph_resolves_sibling_files() {
    let dir = scratch("bbs");
    std::fs::write(dir.join("Target.mbs"), b"x").unwrap();
    let path = dir.join("Demo.bbs");
    std::fs::write(&path, synthetic_bbs()).unwrap();

    let graph = document::bbs::graph(&synthetic_bbs(), &path);
    assert_eq!(graph.nodes.len(), 3, "文件 + mbs + ftx");
    assert_eq!(graph.edges.len(), 2);

    let mbs = graph
        .nodes
        .iter()
        .find(|node| node.label == "Target.mbs")
        .expect("mbs 节点");
    assert_eq!(mbs.status.as_deref(), Some("resolved"), "同目录应命中");

    let ftx = graph
        .nodes
        .iter()
        .find(|node| node.label == "Target.ftx")
        .expect("ftx 节点");
    assert_eq!(ftx.status.as_deref(), Some("missing"), "缺文件应标 missing");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn otm_graph_reports_missing_resource_without_panicking() {
    let dir = scratch("otm");
    let mut bytes = vec![0u8; 0x80];
    bytes[..4].copy_from_slice(b"OTBM");
    slot(&mut bytes, 0x10, "NoSuch.MBS\0");
    let path = dir.join("Demo.otm");
    std::fs::write(&path, &bytes).unwrap();

    let graph = document::otm::graph(&bytes, &path);
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.nodes[1].detail.as_deref(), Some("model"));
    assert_eq!(graph.nodes[1].status.as_deref(), Some("missing"));

    let _ = std::fs::remove_dir_all(&dir);
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

#[test]
#[ignore = "需要本机 UNICORN OVERLORD 语料"]
fn corpus_bbs_names_resolve() {
    let root = std::env::var("CORPUS_UO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"F:\Codes\UNICORN OVERLORD\extract\000\BattleBG"));

    let mut files = Vec::new();
    walk(&root, "bbs", &mut files);
    if files.is_empty() {
        eprintln!("跳过：{} 下没有 .bbs", root.display());
        return;
    }

    let mut total = 0usize;
    let mut resolved = 0usize;
    for path in &files {
        let bytes = std::fs::read(path).unwrap();
        let graph = document::bbs::graph(&bytes, path);
        for node in graph.nodes.iter().skip(1) {
            total += 1;
            if node.status.as_deref() == Some("resolved") {
                resolved += 1;
            }
        }
    }

    let ratio = resolved as f64 / total as f64;
    eprintln!("bbs 名表解析 {resolved}/{total} = {:.1}%", ratio * 100.0);
    assert!(total > 0);
    assert!(ratio >= 0.95, "同目录命中率应 ≥95%");
}

#[test]
#[ignore = "需要本机 13 Sentinels 语料"]
fn corpus_otm_header_resource_resolves() {
    let root = std::env::var("CORPUS_13")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"F:\Codes\13\13rim ex\EX DATA"));

    let mut files = Vec::new();
    walk(&root, "otm", &mut files);
    if files.is_empty() {
        eprintln!("跳过：{} 下没有 .otm", root.display());
        return;
    }

    let mut total = 0usize;
    let mut resolved = 0usize;
    for path in &files {
        let bytes = std::fs::read(path).unwrap();
        let graph = document::otm::graph(&bytes, path);
        if let Some(node) = graph.nodes.get(1) {
            total += 1;
            if node.status.as_deref() == Some("resolved") {
                resolved += 1;
            }
        }
    }

    let ratio = resolved as f64 / total as f64;
    eprintln!("otm 清单资源解析 {resolved}/{total} = {:.1}%", ratio * 100.0);
    assert!(total > 0);
    assert!(ratio >= 0.90, "有界索引应能找到兄弟目录里的资源");
}
