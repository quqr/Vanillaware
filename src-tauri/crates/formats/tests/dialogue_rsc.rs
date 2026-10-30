//! ```text
//! cargo test -p formats --test dialogue_rsc -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};

use formats::rsc::{self, TextPool};

fn corpus(env: &str, fallback: &str) -> PathBuf {
    std::env::var(env)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(fallback))
}

fn scripts(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(read) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in read.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("rsc"))
            {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

fn open(script: &Path) -> (rsc::Dialogue, String) {
    let bytes = std::fs::read(script).expect("read script");
    let pool = rsc::resolve::locate(script).map(|path| TextPool::load(&path).expect("read pool"));
    let text = pool
        .as_ref()
        .map(|pool| pool.get(15047).unwrap_or("").to_string());
    let dialogue = rsc::dialogue::parse(&bytes, pool.as_ref()).expect("parse script");
    (dialogue, text.unwrap_or_default())
}

#[test]
#[ignore = "requires the local 13 Sentinels corpus"]
fn ogata_opens_with_resolved_text() {
    let root = corpus(
        "CORPUS_13_ROUNDSCRIPT",
        r"F:\Codes\13\13rim ex\extract_data\RoundScript",
    );
    let script = root.join("Ogata00_00.rsc");
    if !script.is_file() {
        eprintln!("skipped: {} is missing", script.display());
        return;
    }

    let (dialogue, probe) = open(&script);
    assert_eq!(probe.chars().count(), 9);
    assert_eq!(probe.lines().count(), 2);
    assert_eq!(dialogue.stats.entries, 86);
    assert_eq!(dialogue.warnings, Vec::<String>::new());

    let entry = dialogue
        .entries
        .iter()
        .find(|entry| entry.name == "CLOUD_KISARAGI" && entry.group == 2011)
        .expect("entry CLOUD_KISARAGI/2011");
    assert_eq!(entry.offset, 0x4a34);
    assert_eq!(entry.commands, 13);

    let line = entry.lines.first().expect("first line");
    assert_eq!(line.opcode, 0x189);
    assert_eq!(line.text_id, 15047);
    assert_eq!(line.speaker.as_deref(), Some("PLAYER"));
    assert_eq!(line.voice, None);
    let text = line.text.as_deref().expect("resolved text");
    assert_eq!(text.chars().count(), 9);
    assert_eq!(text.lines().count(), 2);
}

#[test]
#[ignore = "requires the local 13 Sentinels corpus"]
fn corpus_totals_match_the_verified_baseline() {
    let root = corpus("CORPUS_13", r"F:\Codes\13\13rim ex\extract_data");
    let files = scripts(&root);
    if files.is_empty() {
        eprintln!("skipped: no .rsc under {}", root.display());
        return;
    }

    let mut entries = 0u32;
    let mut commands = 0u32;
    let mut text_commands = 0u32;
    let mut missing = 0u32;
    let mut failures = 0u32;
    let mut ids = std::collections::BTreeSet::new();

    for script in &files {
        let (dialogue, _) = open(script);
        entries += dialogue.stats.entries;
        commands += dialogue.stats.commands;
        text_commands += dialogue.stats.text_commands;
        missing += dialogue.stats.missing_text;
        for entry in &dialogue.entries {
            for line in &entry.lines {
                if line.text_id != u32::MAX {
                    ids.insert(line.text_id);
                }
            }
        }
        if !dialogue.warnings.is_empty() {
            failures += dialogue.warnings.len() as u32;
            for warning in &dialogue.warnings {
                eprintln!("{}: {warning}", script.display());
            }
        }
    }
    let unique = ids.len() as u32;

    eprintln!(
        "files {} · entries {entries} · commands {commands} · text {text_commands} · ids {unique} · missing {missing}",
        files.len()
    );

    assert_eq!(files.len(), 480);
    assert_eq!(entries, 6589);
    assert_eq!(commands, 133858);
    assert_eq!(text_commands, 23671);
    assert_eq!(unique, 21765);
    assert_eq!(missing, 0);
    assert_eq!(failures, 0);
}

fn pool_bytes(strings: &[&str]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"FMSB");
    let payload_size = 0u32;
    out.extend_from_slice(&payload_size.to_le_bytes());
    out.extend_from_slice(&0x20u32.to_le_bytes());
    out.extend_from_slice(&[0u8; 8]);
    out.extend_from_slice(&(strings.len() as u32).to_le_bytes());
    out.push(3);
    out.extend_from_slice(&[0u8; 7]);
    out.extend(std::iter::repeat_n(0u8, strings.len() * 8));
    for text in strings {
        out.extend_from_slice(text.as_bytes());
        out.push(0);
    }
    while out.len() % 16 != 0 {
        out.push(0);
    }
    out.extend_from_slice(b"FEOC");
    out.extend_from_slice(&[0u8; 12]);
    out
}

fn command(opcode: u32, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&opcode.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(body);
    out
}

fn text_command(text_id: u32, speaker: &str, voice: u32) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&3u32.to_le_bytes());
    body.extend_from_slice(&text_id.to_le_bytes());
    body.extend_from_slice(&1u32.to_le_bytes());
    body.extend_from_slice(speaker.as_bytes());
    body.push(0);
    while body.len() % 4 != 0 {
        body.push(0);
    }
    body.extend_from_slice(&3u32.to_le_bytes());
    body.extend_from_slice(&voice.to_le_bytes());
    command(0x189, &body)
}

fn script_bytes(commands: &[u8]) -> Vec<u8> {
    let entry = 0x10usize;
    let mut out = Vec::new();
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&0xdeadbeefu32.to_le_bytes());
    out.extend_from_slice(&[0u8; 8]);
    let mut name = [0u8; 0x40];
    name[..5].copy_from_slice(b"SCENE");
    out.extend_from_slice(&name);
    out.extend_from_slice(&7u32.to_le_bytes());
    out.extend_from_slice(&[0u8; 0x1c]);
    out.extend_from_slice(&42i16.to_le_bytes());
    out.extend_from_slice(&(-1i16).to_le_bytes());
    out.extend_from_slice(&[0u8; 0x38]);
    out.extend_from_slice(&(-1i32).to_le_bytes());
    out.extend_from_slice(&((entry + 0xb4) as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(commands);
    out
}

#[test]
fn synthetic_script_resolves_text_and_skips_noise() {
    let pool = TextPool::parse(&pool_bytes(&["zero", "one", "two"]), Path::new("mem.fms"))
        .expect("pool parses");
    assert_eq!(pool.len(), 3);
    assert_eq!(pool.get(1), Some("one"));
    assert_eq!(pool.get(9), None);

    let mut stream = text_command(1, "KISARAGI", u32::MAX);
    stream.extend(command(0x005, &[2, 0, 0, 0, 0x2d, 0, 0, 0]));
    stream.extend(text_command(2, "PLAYER", 5));
    let dialogue =
        rsc::dialogue::parse(&script_bytes(&stream), Some(&pool)).expect("script parses");

    assert_eq!(dialogue.stats.entries, 1);
    assert_eq!(dialogue.stats.commands, 3);
    assert_eq!(dialogue.stats.text_commands, 2);
    assert_eq!(dialogue.stats.unique_ids, 2);
    assert_eq!(dialogue.stats.missing_text, 0);
    assert!(dialogue.warnings.is_empty());

    let entry = &dialogue.entries[0];
    assert_eq!(entry.name, "SCENE");
    assert_eq!(entry.group, 42);
    assert_eq!(entry.commands, 3);
    assert_eq!(entry.lines.len(), 2);

    let first = &entry.lines[0];
    assert_eq!(first.speaker.as_deref(), Some("KISARAGI"));
    assert_eq!(first.text.as_deref(), Some("one"));
    assert_eq!(first.voice, None);
    assert_eq!(first.at, 0x10 + 0xb4);

    let second = &entry.lines[1];
    assert_eq!(second.text_id, 2);
    assert_eq!(second.text.as_deref(), Some("two"));
    assert_eq!(second.voice, Some(5));
}

#[test]
fn missing_pool_keeps_the_lines() {
    let mut stream = text_command(1, "KISARAGI", u32::MAX);
    stream.extend(text_command(2, "PLAYER", 0xff));
    let dialogue = rsc::dialogue::parse(&script_bytes(&stream), None).expect("script parses");

    assert_eq!(dialogue.stats.text_commands, 2);
    assert_eq!(dialogue.stats.missing_text, 2);
    assert_eq!(dialogue.pool_count, 0);
    assert_eq!(dialogue.entries[0].lines[1].text, None);
    assert_eq!(dialogue.entries[0].lines[1].voice, Some(0xff));
}
