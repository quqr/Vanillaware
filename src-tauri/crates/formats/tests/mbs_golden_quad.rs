//! ```text
//! $env:MBS_GOLDEN_DIR="F:\Codes\UNICORN OVERLORD\extract\Main-Output\quads"
//! $env:MBS_ROOT="F:\Codes\UNICORN OVERLORD\extract"
//! $env:MBS_TAG="swi_unic"
//! cargo test -p formats --test mbs_golden_quad -- --nocapture
//! ```

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use formats::mbs::{self, validate};

const DEFAULT_SAMPLE: usize = 32;

#[test]
fn quad_matches_csharp_golden() {
    let Ok(golden_dir) = std::env::var("MBS_GOLDEN_DIR") else {
        eprintln!("skip: MBS_GOLDEN_DIR is not set");
        return;
    };
    let Ok(mbs_root) = std::env::var("MBS_ROOT") else {
        eprintln!("skip: MBS_ROOT is not set");
        return;
    };
    let golden_dir = PathBuf::from(golden_dir);
    let mbs_root = PathBuf::from(mbs_root);
    if !golden_dir.is_dir() || !mbs_root.is_dir() {
        eprintln!("skip: the golden or mbs root directory does not exist");
        return;
    }

    let tag = std::env::var("MBS_TAG").unwrap_or_else(|_| "swi_unic".to_string());
    let mut mbs_by_stem: HashMap<String, PathBuf> = HashMap::new();
    let mut ambiguous: HashSet<String> = HashSet::new();
    collect_mbs(&mbs_root, &mut mbs_by_stem, &mut ambiguous);

    let mut goldens: Vec<PathBuf> = std::fs::read_dir(&golden_dir)
        .expect("read golden dir")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().ends_with(".quad.json"))
        })
        .collect();
    goldens.sort();

    let mut pairs: Vec<Pair> = Vec::new();
    let mut unpaired = Vec::new();
    let mut skipped_ambiguous = Vec::new();
    for golden_path in &goldens {
        let Some(stem) = golden_stem(golden_path) else {
            continue;
        };
        let Some(mbs_path) = mbs_by_stem.get(&stem) else {
            if ambiguous.contains(&stem) {
                skipped_ambiguous.push(stem);
            } else {
                unpaired.push(stem);
            }
            continue;
        };
        pairs.push(Pair {
            group: group_of(mbs_path, &mbs_root),
            mbs: mbs_path.clone(),
            golden: golden_path.clone(),
        });
    }

    let total_paired = pairs.len();
    let sample = sample_size();
    if sample != 0 && sample < pairs.len() {
        pairs = stratified_sample(pairs, sample);
    }

    let mut checked = 0usize;
    let mut lines = Vec::new();
    let mut categories: BTreeMap<String, (usize, String)> = BTreeMap::new();
    let mut translation_count = 0usize;
    let mut total_diffs = 0usize;

    for pair in &pairs {
        let mbs_path = &pair.mbs;
        let v55 = mbs::parse_file(mbs_path, &tag).expect("parse mbs");
        let quad = mbs::convert(&v55).expect("convert V55 to Quad");
        let raw = std::fs::read(&pair.golden).expect("read golden");
        let golden: serde_json::Value =
            serde_json::from_slice(&raw).expect("parse the golden JSON");

        let report = validate::compare_golden(&quad, &golden).expect("compare");
        checked += 1;
        translation_count += report.translation_diffs.len();
        total_diffs += report.diffs.len();

        let issues = validate::self_check(&quad);
        if !issues.is_empty() {
            lines.push(format!(
                "{}: {} self-consistency checks do not hold",
                mbs_path.display(),
                issues.len()
            ));
            lines.extend(issues.iter().take(5).map(|issue| format!("    {issue}")));
        }
        if !report.is_pass() {
            lines.push(format!(
                "{}: {} differences",
                mbs_path.display(),
                report.diffs.len()
            ));
            for diff in &report.diffs {
                let key = diff_category(&diff.path);
                let entry = categories.entry(key).or_insert((0, diff.to_string()));
                entry.0 += 1;
            }
            lines.extend(
                report
                    .diffs
                    .iter()
                    .take(3)
                    .map(|diff| format!("    {diff}")),
            );
        }
    }

    if checked == 0 {
        eprintln!("skip: no pair found");
        return;
    }
    eprintln!(
        "compared {checked}/{total_paired} files: {total_diffs} failing differences, \
         {translation_count} translation-column differences (reported only), {} mbs not found, {} skipped as ambiguous",
        unpaired.len(),
        skipped_ambiguous.len()
    );
    if !categories.is_empty() {
        eprintln!("difference classes (path -> count / first example):");
        for (key, (count, example)) in &categories {
            eprintln!("  {key}: {count}\n    e.g. {example}");
        }
    }
    if !unpaired.is_empty() {
        eprintln!(
            "unpaired goldens (first 10): {:?}",
            &unpaired[..unpaired.len().min(10)]
        );
    }
    if !skipped_ambiguous.is_empty() {
        eprintln!("several .mbs share the stem so the golden cannot be attributed, skipped: {skipped_ambiguous:?}");
    }
    assert!(
        lines.is_empty(),
        "differs from the C# golden://n{}",
        lines.join("\n")
    );
}

struct Pair {
    group: String,
    mbs: PathBuf,
    golden: PathBuf,
}

fn sample_size() -> usize {
    if std::env::var("MBS_FULL_VERIFY").is_ok_and(|value| value == "1") {
        return 0;
    }
    match std::env::var("MBS_GOLDEN_SAMPLE") {
        Ok(text) => text.parse().unwrap_or(DEFAULT_SAMPLE),
        Err(_) => DEFAULT_SAMPLE,
    }
}

fn stratified_sample(mut pairs: Vec<Pair>, sample: usize) -> Vec<Pair> {
    let total = pairs.len();
    let mut by_group: BTreeMap<String, Vec<Pair>> = BTreeMap::new();
    pairs.drain(..).for_each(|pair| {
        by_group.entry(pair.group.clone()).or_default().push(pair);
    });

    let mut picked = Vec::new();
    for (group, mut group_pairs) in by_group {
        group_pairs.sort_by_key(|pair| stable_hash(&group, &pair.mbs));
        let quota = (sample * group_pairs.len()).div_ceil(total);
        let take = quota.max(2).min(group_pairs.len());
        picked.extend(group_pairs.into_iter().take(take));
    }
    picked.sort_by(|left, right| left.mbs.cmp(&right.mbs));
    picked
}

fn stable_hash(group: &str, path: &Path) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in group.bytes().chain(path.to_string_lossy().bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn group_of(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .and_then(|relative| relative.parent())
        .and_then(|parent| parent.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".to_string())
}

fn diff_category(path: &str) -> String {
    let mut out = String::new();
    let mut inside_index = false;
    for ch in path.chars() {
        match ch {
            '[' => inside_index = true,
            ']' => {
                inside_index = false;
                if !out.ends_with("[]") {
                    out.push_str("[]");
                }
            }
            _ if inside_index => {}
            _ => out.push(ch),
        }
    }
    out
}

fn collect_mbs(dir: &Path, index: &mut HashMap<String, PathBuf>, ambiguous: &mut HashSet<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut children: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    children.sort();
    for path in children {
        if path.is_dir() {
            collect_mbs(&path, index, ambiguous);
        } else if path.extension().is_some_and(|ext| ext == "mbs") {
            let Some(stem) = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
            else {
                continue;
            };
            if ambiguous.contains(&stem) {
                continue;
            }
            if index.insert(stem.clone(), path).is_some() {
                index.remove(&stem);
                ambiguous.insert(stem);
            }
        }
    }
}

fn golden_stem(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_string_lossy().into_owned();
    Some(name.strip_suffix(".quad.json")?.to_string())
}
