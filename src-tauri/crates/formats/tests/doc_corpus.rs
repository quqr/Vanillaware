//! ```text
//! cargo test -p formats --test doc_corpus -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};

use formats::document;

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
        .filter(|path| {
            path.extension()
                .is_some_and(|value| value.eq_ignore_ascii_case(ext))
        })
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

fn check_ok(arena: &document::DocArena, check: &str) -> Option<bool> {
    arena
        .doc(0, String::new(), String::new(), String::new(), 0, 0)
        .checks
        .iter()
        .find(|item| item.id == check)
        .map(|item| item.ok)
}

fn assert_corpus(ext: &str, ids: &[&str]) {
    let root = corpus("CORPUS_13", r"F:\Codes\13\13rim ex\EX DATA");
    let mut files = Vec::new();
    walk(&root, ext, &mut files);
    if files.is_empty() {
        eprintln!("skipped: no .{ext} under {}", root.display());
        return;
    }

    let mut failed = 0;
    for path in &files {
        let arena = document::parse_file(path, None).expect("doc parse");
        for id in ids {
            if check_ok(&arena, id) == Some(false) {
                failed += 1;
                eprintln!("{} failed {}: {}", path.display(), id, path.display());
                break;
            }
        }
    }
    eprintln!(".{ext}: {} files, {failed} failed", files.len());
    assert_eq!(failed, 0);
}

#[test]
#[ignore = "requires the local 13 Sentinels corpus"]
fn fms_corpus_is_closed() {
    assert_corpus("fms", &["fms.trailer", "fms.payload"]);
}

#[test]
#[ignore = "requires the local 13 Sentinels corpus"]
fn rsc_corpus_is_closed() {
    assert_corpus("rsc", &["rsc.table", "rsc.payloadStart"]);
}

#[test]
#[ignore = "requires the local 13 Sentinels corpus"]
fn otm_corpus_is_closed() {
    assert_corpus(
        "otm",
        &[
            "otm.magic",
            "otm.size",
            "otm.offB",
            "otm.offC",
            "otm.sections",
        ],
    );
}

#[test]
#[ignore = "requires the local 13 Sentinels corpus"]
fn mpd_corpus_is_closed() {
    assert_corpus(
        "mpd",
        &["mpd.magic", "mpd.size", "mpd.payload", "mpd.offset"],
    );
}

#[test]
#[ignore = "requires the local 13 Sentinels corpus"]
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
#[ignore = "requires the local 13 Sentinels corpus"]
fn fmd_mesh_scene_has_geometry() {
    let path = PathBuf::from(r"F:\Codes\13\13rim ex\EX DATA\BattleStageModel\BT_STAGE00.fmd");
    if !path.is_file() {
        eprintln!("skipped: {} does not exist", path.display());
        return;
    }
    let bytes = std::fs::read(path).unwrap();
    let scene = document::fmd::mesh_scene(&bytes).expect("mesh scene");

    let meshes: Vec<_> = scene
        .nodes
        .iter()
        .filter_map(|node| node.mesh.as_ref())
        .collect();
    let triangles: usize = meshes.iter().map(|mesh| mesh.indices.len() / 3).sum();
    let with_uv = meshes.iter().filter(|mesh| !mesh.uvs.is_empty()).count();
    let uv_range = meshes
        .iter()
        .flat_map(|mesh| mesh.uvs.iter())
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), value| {
            (lo.min(*value), hi.max(*value))
        });
    let ranges: Vec<_> = meshes
        .iter()
        .flat_map(|mesh| mesh.sub_ranges.iter())
        .collect();
    let textured = ranges.iter().filter(|range| range.page >= 0).count();
    let collision = ranges
        .iter()
        .filter(|range| range.page < 0 && range.rgba[3] == 0)
        .count();
    let untextured = ranges.len() - textured - collision;
    eprintln!(
        "fmd mesh: {} meshes, {} triangles, {} with UV, UV range {:?}, {} sections (textured {} / untextured {} / collision {}), bounds {:?}, truncated {}",
        meshes.len(),
        triangles,
        with_uv,
        uv_range,
        ranges.len(),
        textured,
        untextured,
        collision,
        scene.bounds,
        scene.truncated
    );

    assert!(!meshes.is_empty());
    assert!(triangles > 0);
    assert!(with_uv > 0, "the sample FMD must yield UVs");
    assert!(uv_range.0.is_finite() && uv_range.1.is_finite());
    assert!(scene.bounds.iter().all(|value| value.is_finite()));
    assert!(meshes.iter().all(|mesh| mesh.positions.len() % 3 == 0));
    assert!(meshes.iter().all(|mesh| mesh.uvs.len() % 2 == 0));
    assert!(
        textured > 0,
        "the sample FMD must have a textured draw section"
    );
    assert_eq!(
        collision, 0,
        "collision materials must not enter draw sections"
    );
    assert!(ranges.iter().all(|range| (-1..=3).contains(&range.tex_ref)));
}

#[test]
#[ignore = "requires the local 13 Sentinels corpus"]
fn fmd_corpus_is_closed() {
    assert_corpus("fmd", &["fmd.closed", "fmd.versionGate"]);
}

#[test]
#[ignore = "requires the local 13 Sentinels corpus"]
fn fmd_mesh_count_matches_python() {
    let root = corpus("CORPUS_13", r"F:\Codes\13\13rim ex\EX DATA");
    let mut files = Vec::new();
    walk(&root, "fmd", &mut files);
    if files.is_empty() {
        eprintln!("skipped: no .fmd under {}", root.display());
        return;
    }

    let mut meshes = 0usize;
    let mut declared = 0u64;
    let mut parsed = 0usize;
    for path in &files {
        let arena = document::parse_file(path, None).expect("fmd parse");
        meshes += arena.count_field_value("nonEmpty", "true");
        parsed += arena.count_field_value("nonEmpty", "true")
            + arena.count_field_value("nonEmpty", "false");
        declared += arena.sum_field("meshCount");
    }
    eprintln!(
        ".fmd: {} files, declared {declared} meshes, parsed {parsed}, non-empty {meshes}",
        files.len()
    );

    assert_eq!(
        declared, parsed as u64,
        "declared and parsed mesh counts differ"
    );
    if let Ok(expected) = std::env::var("FMD_MESHES") {
        assert_eq!(meshes, expected.parse::<usize>().unwrap());
    }
    assert!(meshes > 0);
}

#[test]
#[ignore = "requires the local 13 Sentinels corpus"]
fn fsb_corpus_is_closed() {
    assert_corpus("fsb", &["fsb.closed", "fsb.sequence", "fsb.version"]);
}

#[test]
#[ignore = "requires the local UO corpus"]
fn asb_corpus_parses_to_eof() {
    let dir = corpus(
        "CORPUS_ASB",
        r"F:\Codes\UNICORN OVERLORD\extract\000\Script",
    );
    let files = files(&dir, "asb");
    if files.is_empty() {
        eprintln!("skipped: no .asb under {}", dir.display());
        return;
    }

    let mut failed = 0usize;
    let mut broken_refs = 0usize;
    let mut enums = 0u64;
    let mut functions = 0u64;
    let mut globals = 0u64;
    let mut strings = 0u64;
    for path in &files {
        match document::parse_file(path, None) {
            Ok(arena) if check_ok(&arena, "asb.eof") == Some(true) => {
                enums += arena.sum_field("enums");
                functions += arena.sum_field("functions") + arena.sum_field("globalFuncs");
                globals += arena.sum_field("globals");
                strings += arena.sum_field("strings");
                if check_ok(&arena, "asb.refs") != Some(true) {
                    broken_refs += 1;
                    eprintln!("dangling operands: {}", path.display());
                }
            }
            Ok(_) => {
                failed += 1;
                eprintln!("did not reach EOF: {}", path.display());
            }
            Err(error) => {
                failed += 1;
                eprintln!("parse failed {}: {error}", path.display());
            }
        }
    }
    eprintln!(
        ".asb: {} files, {failed} failed, {broken_refs} dangling; enum {enums} / func {functions} / global {globals} / string {strings}",
        files.len()
    );
    assert_eq!(failed, 0);
    assert_eq!(broken_refs, 0);
}

#[test]
#[ignore = "requires the local UO corpus"]
fn asb_disasm_sample_matches_golden() {
    let path = PathBuf::from(r"F:\Codes\UNICORN OVERLORD\extract\000\Script\CaptureCharacter.asb");
    if !path.is_file() {
        eprintln!("skipped: {} does not exist", path.display());
        return;
    }
    let bytes = std::fs::read(path).unwrap();
    let asm = document::asb::disassemble_file(&bytes, Some(0)).expect("disasm");
    let ops: Vec<&str> = asm.lines.iter().map(|line| line.op.as_str()).collect();
    assert_eq!(
        ops,
        [
            "SUSPEND", "SetV1", "CpyVtoR4", "JLowZ", "SUSPEND", "SUSPEND", "PshC4", "CALLSYS",
            "JMP", "SUSPEND", "RET",
        ]
    );
    assert!(asm.lines.iter().any(|line| line
        .note
        .as_deref()
        .is_some_and(|note| note.contains("Wait"))));

    let pseudo = document::asb::pseudo_file(&bytes, Some(0)).expect("pseudo");
    assert!(pseudo.text.contains("Wait(100)"));
    assert!(pseudo.text.contains("goto L1"));
}

#[test]
#[ignore = "requires the local UO corpus"]
fn bbs_layout_sample_is_nonempty() {
    let dir = corpus(
        "CORPUS_BBS",
        r"F:\Codes\UNICORN OVERLORD\extract\000\BattleBG",
    );
    let files = files(&dir, "bbs");
    let Some(path) = files.first() else {
        eprintln!("skipped: no .bbs under {}", dir.display());
        return;
    };
    let bytes = std::fs::read(path).unwrap();
    let layout = document::bbs::layout(&bytes);
    assert!(layout.bbox.iter().all(|value| value.is_finite()));
    assert!(!layout.objects.is_empty());
    assert!(!layout.mbs_names.is_empty());
}

#[test]
#[ignore = "requires the local 13 Sentinels corpus"]
fn fms_text_sample_is_nonempty() {
    let path = PathBuf::from(r"F:\Codes\13\13rim ex\EX DATA\Fms\ScriptText.fms");
    if !path.is_file() {
        eprintln!("skipped: {} does not exist", path.display());
        return;
    }
    let bytes = std::fs::read(path).unwrap();
    let table = document::fms::text_table(&bytes);
    assert_eq!(table.count as usize, table.strings.len());
    assert!(!table.strings.is_empty());
    assert!(table.strings.iter().any(|text| !text.is_empty()));
}

#[test]
#[ignore = "requires the local UO corpus"]
fn bbs_corpus_is_closed() {
    let dir = corpus(
        "CORPUS_BBS",
        r"F:\Codes\UNICORN OVERLORD\extract\000\BattleBG",
    );
    let files = files(&dir, "bbs");
    if files.is_empty() {
        eprintln!("skipped: {} has no .bbs", dir.display());
        return;
    }

    let mut failed = 0;
    for path in &files {
        let arena = document::parse_file(path, None).expect("bbs parse");
        if check_ok(&arena, "bbs.closed") != Some(true) {
            failed += 1;
            eprintln!("not closed: {}", path.display());
        }
    }
    eprintln!("bbs: {} files, {failed} failed", files.len());
    assert_eq!(failed, 0);
}

#[test]
#[ignore = "requires the local 13 Sentinels corpus"]
fn osb_resources_sample() {
    let dir = corpus("CORPUS_OSB", r"F:\Codes\13\13rim ex\EX DATA\Stage");
    let files = files(&dir, "osb");
    let Some(path) = files.first() else { return };
    let bytes = std::fs::read(path).unwrap();
    let resources = document::osb::resources(&bytes);
    eprintln!("{}: {} resources", path.display(), resources.len());
    for item in resources.iter().take(20) {
        eprintln!("  {item}");
    }
}

#[test]
#[ignore = "requires the local 13 Sentinels corpus"]
fn osb_scene_sample_has_geometry() {
    let path = PathBuf::from(r"F:\Codes\13\13rim ex\EX DATA\Stage\98siki_2syume.osb");
    if !path.is_file() {
        eprintln!("skipped: {} does not exist", path.display());
        return;
    }
    let bytes = std::fs::read(&path).unwrap();
    let scene = document::osb::scene(&bytes, &path);

    let meshes: Vec<_> = scene
        .nodes
        .iter()
        .filter_map(|node| node.mesh.as_ref())
        .collect();
    let triangles: usize = meshes.iter().map(|mesh| mesh.indices.len() / 3).sum();
    let references: usize = scene
        .nodes
        .iter()
        .filter(|node| node.reference.is_some())
        .count();
    let resolved = scene
        .references
        .iter()
        .filter(|reference| reference.status == "resolved")
        .count();
    eprintln!(
        "osb scene: {} nodes, {} meshes, {} triangles, {} references ({} resolvable), bounds {:?}, truncated {}",
        scene.nodes.len(),
        meshes.len(),
        triangles,
        references,
        resolved,
        scene.bounds,
        scene.truncated
    );

    assert!(scene.nodes.len() > 1);
    assert!(!scene.references.is_empty());
    assert!(
        resolved > 0,
        "the sample OSB references must resolve by relative path"
    );
    assert!(!meshes.is_empty());
    assert!(triangles > 0);
    assert!(scene.bounds.iter().all(|value| value.is_finite()));
    assert!(meshes.iter().all(|mesh| mesh.positions.len() % 3 == 0));
    assert!(scene
        .nodes
        .iter()
        .all(|node| node.transform.iter().all(|value| value.is_finite())));
}

#[test]
#[ignore = "requires the local 13 Sentinels corpus"]
fn rsc_graph_sample_has_entries() {
    let samples = [
        r"F:\Codes\13\13rim ex\EX DATA\BattleScript\area1_wave1.rsc",
        r"F:\Codes\13\13rim ex\EX DATA\RoundScript\Amiguchi00_30.rsc",
    ];
    for sample in samples {
        let path = PathBuf::from(sample);
        if !path.is_file() {
            eprintln!("skipped: {} does not exist", path.display());
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        let graph = document::rsc::graph(&bytes, &path);

        let entries = graph
            .nodes
            .iter()
            .filter(|node| node.kind == "entry")
            .count();
        let scripts = graph
            .nodes
            .iter()
            .filter(|node| node.kind == "script")
            .count();
        let externals = graph
            .nodes
            .iter()
            .filter(|node| node.kind == "external")
            .count();
        let loads = graph
            .edges
            .iter()
            .filter(|edge| edge.label == "loads")
            .count();
        eprintln!(
            "{}: {} nodes (entry {entries}, script {scripts}, external {externals}), {} edges (loads {loads}), truncated {}",
            path.file_name().unwrap().to_string_lossy(),
            graph.nodes.len(),
            graph.edges.len(),
            graph.truncated
        );

        assert_eq!(graph.nodes[0].kind, "file");
        assert!(graph.nodes[0].status.is_none());
        assert!(entries > 0);
        assert!(scripts > 0);
        assert_eq!(graph.edges.len(), entries + scripts + externals);
        assert_eq!(
            externals,
            1 + loads,
            "external = paired .fms plus payload assets"
        );
        assert!(graph
            .nodes
            .iter()
            .filter(|node| node.kind == "external")
            .all(|node| node.status.is_some()));
        assert!(graph
            .edges
            .iter()
            .all(|edge| (edge.from as usize) < graph.nodes.len()));
        assert!(graph
            .edges
            .iter()
            .all(|edge| (edge.to as usize) < graph.nodes.len()));

        if loads > 0 {
            let resolved = graph
                .nodes
                .iter()
                .filter(|node| {
                    node.kind == "external" && node.status.as_deref() == Some("resolved")
                })
                .count();
            assert_eq!(
                resolved,
                1 + loads,
                "every reference must resolve in the corpus"
            );
            assert!(graph
                .nodes
                .iter()
                .any(|node| node.label.eq_ignore_ascii_case("GUI/WARPCIRCLE_MONE.MBS")));
        }
    }
}

#[test]
#[ignore = "requires the local 13 Sentinels corpus"]
fn osb_corpus_is_closed() {
    let dir = corpus("CORPUS_OSB", r"F:\Codes\13\13rim ex\EX DATA\Stage");
    let files = files(&dir, "osb");
    if files.is_empty() {
        eprintln!("skipped: {} has no .osb", dir.display());
        return;
    }

    let mut failed = 0;
    for path in &files {
        let arena = document::parse_file(path, None).expect("osb parse");
        if check_ok(&arena, "osb.closed") != Some(true) {
            failed += 1;
            eprintln!("not closed: {}", path.display());
        }
    }
    eprintln!("osb: {} files, {failed} failed", files.len());
    assert_eq!(failed, 0);
}

#[test]
#[ignore = "requires the local 13 Sentinels corpus"]
fn scb_corpus_is_closed() {
    let roots = [
        corpus("CORPUS_13", r"F:\Codes\13\13rim ex\EX DATA"),
        PathBuf::from(r"F:\Codes\13\13rim dlc ex"),
        PathBuf::from(r"F:\Codes\13\13rim dlc2 ex"),
    ];

    let mut scb = Vec::new();
    for root in &roots {
        walk(root, "scb", &mut scb);
    }
    if scb.is_empty() {
        eprintln!("skipped: no .scb under any of the three roots");
        return;
    }

    let mut failed = 0;
    for path in &scb {
        let arena = document::parse_file(path, None).expect("scb parse");
        if check_ok(&arena, "scb.eof") != Some(true) {
            failed += 1;
            eprintln!("not closed: {}", path.display());
        }
    }
    eprintln!("scb: {} files, {failed} failed", scb.len());
    assert_eq!(failed, 0);
}

#[test]
#[ignore = "requires the local 13 Sentinels corpus"]
fn scb_tree_has_named_nodes() {
    let path = PathBuf::from(r"F:\Codes\13\13rim dlc ex\Script\Animation.scb");
    if !path.is_file() {
        eprintln!("skipped: {} does not exist", path.display());
        return;
    }

    let arena = document::parse_file(&path, None).expect("scb parse");
    let root_page = arena.page(0, 0, 0, 100).expect("root page");
    let root_id = root_page
        .items
        .iter()
        .find(|node| node.name == "ROOT")
        .map(|node| node.id)
        .expect("ROOT group");

    let page = arena.page(0, root_id, 0, 100).expect("ROOT page");

    let names: Vec<String> = page.items.iter().map(|node| node.name.clone()).collect();
    eprintln!("first 12 nodes: {:#?}", &names[..names.len().min(12)]);

    assert!(
        names.iter().any(|name| name.starts_with("@MONSTER")),
        "a @MONSTER node must appear"
    );
    assert!(
        names.iter().any(|name| name.starts_with("@TITLE")),
        "a @TITLE node must appear"
    );
}

#[test]
#[ignore = "requires the local UNICORN OVERLORD extraction"]
fn nnd_container_invariants_hold_for_all_files() {
    let root = corpus(
        "UO_EXTRACT",
        r"F:/Codes/UNICORN OVERLORD/extract/000/NodeNetworkData",
    );
    let mut list = Vec::new();
    walk(&root, "nnd", &mut list);
    assert!(!list.is_empty(), "the corpus is empty: {}", root.display());

    let (mut failed, mut with_units, mut with_tail, mut units) = (0, 0, 0, 0u64);
    for path in &list {
        let arena = document::parse_file(path, None).expect("nnd parse");
        for check in arena.checks() {
            if !check.ok {
                failed += 1;
                eprintln!(
                    "failed {}: {} - {:?}",
                    check.id,
                    path.display(),
                    check.detail
                );
            }
        }
        let page = arena.page(0, arena.root(), 0, 100).expect("root page");
        if page
            .items
            .iter()
            .any(|node| node.name.starts_with("Units ("))
        {
            with_units += 1;
        }
        if page
            .items
            .iter()
            .any(|node| node.name.starts_with("Tail ("))
        {
            with_tail += 1;
        }
        units += page
            .items
            .iter()
            .filter(|node| node.name.starts_with("Units ("))
            .count() as u64;
    }

    eprintln!(
        "nnd: {} files, {failed} failed, {with_units} with payload, {with_tail} with tail, {units} payload units",
        list.len()
    );
    assert_eq!(failed, 0);
    assert_eq!(
        with_units,
        list.len(),
        "every file must carry a payload section"
    );
}
