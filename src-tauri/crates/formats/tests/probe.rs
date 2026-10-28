use std::path::Path;

const SAMPLE: &str = r"F:\Codes\13\13rim ex\EX DATA\BattleEffect\Barrier00.ftx";

#[test]
fn probe_matches_decode_for_single_frame_file() {
    if !Path::new(SAMPLE).exists() {
        eprintln!("skip: sample not found");
        return;
    }

    let infos = formats::ftx::probe_file(Path::new(SAMPLE)).expect("probe");
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].width, 512);
    assert_eq!(infos[0].height, 512);
    assert_eq!(infos[0].format, 0x4D);
    assert_eq!(infos[0].channels, 4);
    assert_eq!(infos[0].format_name(), "BC7");

    let frame = formats::ftx::decode_file_frame(Path::new(SAMPLE), 0).expect("decode frame");
    assert_eq!(frame.width, 512);
    assert_eq!(frame.height, 512);
    assert_eq!(frame.channels, 4);
    assert_eq!(frame.data.len(), 512 * 512 * 4);

    let all = formats::ftx::decode_file(Path::new(SAMPLE)).expect("decode all");
    assert_eq!(all.len(), infos.len());
    assert_eq!(
        all[0].data, frame.data,
        "decode_frame must equal decode_file()[0]"
    );
}

#[test]
fn probe_frame_count_matches_decode_for_many_files() {
    let root = Path::new(r"F:\Codes\13\13rim ex\EX DATA");
    if !root.exists() {
        eprintln!("skip: sample root not found");
        return;
    }

    let mut checked = 0usize;
    let mut mismatched = Vec::new();

    for entry in walk(Path::new(root), 12) {
        if !entry
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("ftx"))
        {
            continue;
        }

        let Ok(infos) = formats::ftx::probe_file(&entry) else {
            continue;
        };
        let Ok(frames) = formats::ftx::decode_file(&entry) else {
            continue;
        };

        if infos.len() != frames.len() {
            mismatched.push(format!(
                "{}: probe {} != decode {}",
                entry.display(),
                infos.len(),
                frames.len()
            ));
        }

        if let Some(last) = infos.len().checked_sub(1) {
            if let Ok(one) = formats::ftx::decode_file_frame(&entry, last) {
                if one.data != frames[last].data {
                    mismatched.push(format!("{}: frame {last} mismatch", entry.display()));
                }
            }
        }

        checked += 1;
    }

    assert!(checked > 0, "no samples checked");
    assert!(
        mismatched.is_empty(),
        "mismatch:\n{}",
        mismatched.join("\n")
    );
}

fn walk(root: &Path, limit: usize) -> Vec<std::path::PathBuf> {
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
            } else {
                out.push(path);
                if out.len() >= limit {
                    return out;
                }
            }
        }
    }

    out
}
