//! Quad ↔ 旧 C# golden（`.quad.json`）结构等价比对（判据见 ADR-0006）。
//!
//! `F:\Codes\UNICORN OVERLORD\extract\Main-Output\quads` 下的 722 个 golden 是旧 C# 工具的产物，
//! 与权威 PHP 存在**算法级**差异；闭集差异表由 [`vw_formats::mbs::validate::normalize_golden`]
//! 规范化抹平，**闭集外的任何差异判失败**。
//!
//! ```text
//! $env:VW_MBS_GOLDEN_DIR="F:\Codes\UNICORN OVERLORD\extract\Main-Output\quads"
//! $env:VW_MBS_MBS_ROOT="F:\Codes\UNICORN OVERLORD\extract"
//! $env:VW_MBS_TAG="swi_unic"
//! cargo test -p vw-formats --test mbs_golden_quad -- --nocapture
//! ```
//!
//! 默认**分层抽样**（每个目录按比例取，固定种子可复现，共约 32 个），因为全量 722 个要跑两分多钟；
//! `VW_MBS_FULL_VERIFY=1` 跑全量，`VW_MBS_GOLDEN_SAMPLE=<n>` 改样本量（`0` 等价于全量）。
//!
//! 未设置环境变量、或目录不存在时自动跳过（与 `gold_png.rs` 同样的「缺料即跳过」约定）。
//!
//! 语料事实：722 个 golden 产自 `extract\000` 树。`Interface` / `Interface_Button` / `Opening00`
//! 这三个 stem 在 `art book ex` 下还有内容不同的同名 `.mbs`，故按 stem 配对时**不猜归属**、
//! 显式记为「因重名跳过」；把 `VW_MBS_MBS_ROOT` 指向 `extract\000` 即可覆盖全部 722 个。

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use vw_formats::mbs::{self, validate};

/// 默认样本量：8 个目录分组各取 4 个左右（每组至少 2 个），debug 下约半分钟。
///
/// 抽样不是「验证强度」的一部分，只是让每次 `cargo test` 不必读 722 个 golden；
/// 发版前用 `VW_MBS_FULL_VERIFY=1` 跑全量（配合 `--release` 约两分钟）。
const DEFAULT_SAMPLE: usize = 32;

#[test]
fn quad_matches_csharp_golden() {
    let Ok(golden_dir) = std::env::var("VW_MBS_GOLDEN_DIR") else {
        eprintln!("skip: 未设置 VW_MBS_GOLDEN_DIR");
        return;
    };
    let Ok(mbs_root) = std::env::var("VW_MBS_MBS_ROOT") else {
        eprintln!("skip: 未设置 VW_MBS_MBS_ROOT");
        return;
    };
    let golden_dir = PathBuf::from(golden_dir);
    let mbs_root = PathBuf::from(mbs_root);
    if !golden_dir.is_dir() || !mbs_root.is_dir() {
        eprintln!("skip: golden 或 mbs 根目录不存在");
        return;
    }

    let tag = std::env::var("VW_MBS_TAG").unwrap_or_else(|_| "swi_unic".to_string());
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
        let v55 = mbs::parse_file(mbs_path, &tag).expect("解析 mbs");
        let quad = mbs::convert(&v55).expect("转换 V55 → Quad");
        let raw = std::fs::read(&pair.golden).expect("读取 golden");
        let golden: serde_json::Value = serde_json::from_slice(&raw).expect("解析 golden JSON");

        let report = validate::compare_golden(&quad, &golden).expect("比对");
        checked += 1;
        translation_count += report.translation_diffs.len();
        total_diffs += report.diffs.len();

        let issues = validate::self_check(&quad);
        if !issues.is_empty() {
            lines.push(format!(
                "{}: 自洽性检查 {} 条不成立",
                mbs_path.display(),
                issues.len()
            ));
            lines.extend(issues.iter().take(5).map(|issue| format!("    {issue}")));
        }
        if !report.is_pass() {
            lines.push(format!(
                "{}: {} 处差异",
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
        eprintln!("skip: 没有找到任何配对");
        return;
    }
    eprintln!(
        "比对 {checked}/{total_paired} 个文件：{total_diffs} 处判失败差异，\
         {translation_count} 处平移列差异（只报告），{} 个未找到 mbs，{} 个因重名跳过",
        unpaired.len(),
        skipped_ambiguous.len()
    );
    if !categories.is_empty() {
        eprintln!("差异分类（路径 → 次数 / 首例）：");
        for (key, (count, example)) in &categories {
            eprintln!("  {key}: {count}\n    例：{example}");
        }
    }
    if !unpaired.is_empty() {
        eprintln!(
            "未配对的 golden（前 10）：{:?}",
            &unpaired[..unpaired.len().min(10)]
        );
    }
    if !skipped_ambiguous.is_empty() {
        eprintln!("同 stem 有多个 .mbs、无法确定 golden 归属，已跳过：{skipped_ambiguous:?}");
    }
    assert!(
        lines.is_empty(),
        "与 C# golden 不一致：\n{}",
        lines.join("\n")
    );
}

/// 一个待比的 `<mbs, golden>` 对，`group` 是它所在目录名（分层抽样的层）。
struct Pair {
    group: String,
    mbs: PathBuf,
    golden: PathBuf,
}

/// 样本量：`VW_MBS_FULL_VERIFY=1` 或 `=0` 表示全量。
fn sample_size() -> usize {
    if std::env::var("VW_MBS_FULL_VERIFY").is_ok_and(|value| value == "1") {
        return 0;
    }
    match std::env::var("VW_MBS_GOLDEN_SAMPLE") {
        Ok(text) => text.parse().unwrap_or(DEFAULT_SAMPLE),
        Err(_) => DEFAULT_SAMPLE,
    }
}

/// 分层抽样：按目录分组，组内用固定种子的哈希排序后取前 `ceil(按比例)` 个，每组至少 2 个。
///
/// 固定种子的哈希保证同一份语料每次抽到同一批文件（可复现），
/// 分层保证 Chara / BattleBG / GUI / CharaFace 等各组都进样本，不会被单一类别淹没。
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

/// FNV-1a：只要固定、跨平台一致即可，不需要密码学强度。
fn stable_hash(group: &str, path: &Path) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in group.bytes().chain(path.to_string_lossy().bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// 分层用的组名：`.mbs` 所在目录名（`Chara` / `BattleBG` / `GUI` …）。
fn group_of(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .and_then(|relative| relative.parent())
        .and_then(|parent| parent.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".to_string())
}

/// 差异分类键：把所有下标折成 `[]`（如 `keyframe[].layer[].dstquad[]`）。
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

/// 递归收集 `.mbs`，按 stem 建索引。
///
/// 同一个 stem 出现在多个目录时**不猜归属**：从索引里剔除并记进 `ambiguous`，
/// 对应 golden 在报告里显式跳过（这类文件的 golden 无法确定是哪一个 `.mbs` 产出的）。
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

/// `<name>.quad.json` → `<name>`。
fn golden_stem(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_string_lossy().into_owned();
    Some(name.strip_suffix(".quad.json")?.to_string())
}
