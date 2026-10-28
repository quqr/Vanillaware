use std::path::Path;

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GameId {
    UnicornOverlord,
    ThirteenSentinels,
    Unknown,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GameProfileDto {
    pub id: GameId,
    pub label: &'static str,
    pub mbs_tag: Option<&'static str>,
    pub confidence: u8,
    pub evidence: Vec<String>,
}

const UO_DIRS: &[&str] = &["Script", "BattleBG"];
const TS_DIRS: &[&str] = &["Stage", "Cot", "Fms"];
const CONTAINERS: &[&str] = &["EX DATA", "000"];

pub fn detect(root: &Path) -> GameProfileDto {
    let mut uo = 0u32;
    let mut ts = 0u32;
    let mut evidence: Vec<String> = Vec::new();

    let mut cursor = Some(root);
    for depth in 0..3 {
        let Some(dir) = cursor else { break };
        scan_markers(dir, &mut uo, &mut ts, &mut evidence);
        if depth > 0 {
            scan_basename(dir, &mut uo, &mut ts, &mut evidence);
        }
        cursor = dir.parent();
    }

    for container in CONTAINERS {
        scan_markers(&root.join(container), &mut uo, &mut ts, &mut evidence);
    }
    for dir in [root.to_path_buf(), root.join("EX DATA"), root.join("000")] {
        scan_extensions(&dir, &mut uo, &mut ts, &mut evidence);
    }

    let id = if uo > ts && uo - ts >= 10 {
        GameId::UnicornOverlord
    } else if ts > uo && ts - uo >= 10 {
        GameId::ThirteenSentinels
    } else {
        GameId::Unknown
    };

    let total = uo + ts;
    let confidence = match id {
        GameId::Unknown => 0,
        GameId::UnicornOverlord => (uo * 100 / total.max(1)) as u8,
        GameId::ThirteenSentinels => (ts * 100 / total.max(1)) as u8,
    };

    evidence.truncate(8);
    GameProfileDto {
        id,
        label: match id {
            GameId::UnicornOverlord => "Unicorn Overlord",
            GameId::ThirteenSentinels => "13 Sentinels",
            GameId::Unknown => "Unknown",
        },
        mbs_tag: match id {
            GameId::UnicornOverlord => Some("swi_unic"),
            GameId::ThirteenSentinels => Some("swi_sent"),
            GameId::Unknown => None,
        },
        confidence,
        evidence,
    }
}

fn scan_markers(dir: &Path, uo: &mut u32, ts: &mut u32, evidence: &mut Vec<String>) {
    if !dir.is_dir() {
        return;
    }
    for marker in UO_DIRS {
        if dir.join(marker).is_dir() {
            *uo += 50;
            push_evidence(evidence, format!("{marker}/ exists under {}", dir.display()));
        }
    }
    for marker in TS_DIRS {
        if dir.join(marker).is_dir() {
            *ts += 50;
            push_evidence(evidence, format!("{marker}/ exists under {}", dir.display()));
        }
    }
}

fn scan_basename(dir: &Path, uo: &mut u32, ts: &mut u32, evidence: &mut Vec<String>) {
    let Some(name) = dir.file_name().and_then(|name| name.to_str()) else {
        return;
    };
    if UO_DIRS.iter().any(|marker| name.eq_ignore_ascii_case(marker)) {
        *uo += 20;
        push_evidence(evidence, format!("directory is named {name}"));
    }
    if TS_DIRS.iter().any(|marker| name.eq_ignore_ascii_case(marker)) {
        *ts += 20;
        push_evidence(evidence, format!("directory is named {name}"));
    }
}

fn scan_extensions(dir: &Path, uo: &mut u32, ts: &mut u32, evidence: &mut Vec<String>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };

    let mut uo_hit = false;
    let mut ts_hit = false;
    for entry in read.flatten().take(256) {
        if !entry.file_type().map(|file| file.is_file()).unwrap_or(false) {
            continue;
        }
        let ext = entry
            .path()
            .extension()
            .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        match ext.as_str() {
            "asb" | "bbs" => {
                *uo += 8;
                uo_hit = true;
            }
            "osb" | "ctb" | "rsc" | "fmd" | "fsb" => {
                *ts += 8;
                ts_hit = true;
            }
            _ => {}
        }
    }

    if uo_hit {
        push_evidence(evidence, format!("asb/bbs found under {}", dir.display()));
    }
    if ts_hit {
        push_evidence(evidence, format!("osb/ctb/rsc/fmd/fsb found under {}", dir.display()));
    }
}

fn push_evidence(evidence: &mut Vec<String>, item: String) {
    if evidence.len() < 8 && !evidence.contains(&item) {
        evidence.push(item);
    }
}
