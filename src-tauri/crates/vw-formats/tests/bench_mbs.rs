//! MBS 解析耗时（预览打开时的固定成本）。默认不跑：
//!
//! ```text
//! cargo test --release -p vw-formats --test bench_mbs -- --ignored --nocapture
//! ```

use std::time::Instant;

const GUI_DIR: &str = r"F:\Codes\UNICORN OVERLORD\extract\000\GUI";
const TAG: &str = "swi_unic";

#[test]
#[ignore = "需要本机 UNICORN OVERLORD 解包语料"]
fn parse_and_convert_cost() {
    let dir = std::path::Path::new(GUI_DIR);
    if !dir.is_dir() {
        eprintln!("跳过：{} 不存在", dir.display());
        return;
    }

    let mut samples: Vec<(u64, std::path::PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("mbs") {
            continue;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        samples.push((size, path));
    }
    samples.sort_by_key(|(size, _)| std::cmp::Reverse(*size));
    samples.truncate(3);

    for (size, path) in samples {
        let t = Instant::now();
        let v55 = match vw_formats::mbs::parse_file(&path, TAG) {
            Ok(v55) => v55,
            Err(e) => {
                eprintln!("{} 解析失败：{e}", path.display());
                continue;
            }
        };
        let parse = t.elapsed();

        let t = Instant::now();
        let quad = match vw_formats::mbs::convert(&v55) {
            Ok(quad) => quad,
            Err(e) => {
                eprintln!("{} 转换失败：{e}", path.display());
                continue;
            }
        };
        let convert = t.elapsed();

        eprintln!(
            "{}（{:.1} KB）：parse {:?} + convert {:?}",
            path.file_name().unwrap().to_string_lossy(),
            size as f64 / 1024.0,
            parse,
            convert
        );

        let _ = quad;
    }
}
