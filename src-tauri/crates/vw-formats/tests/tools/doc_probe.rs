//! 语料抽样探针：`cargo run -p vw-formats --example doc_probe -- <file>...`
//!
//! 打印每个文件的节点数与全部校验项，用来在真实语料上复核解析闭合。

use std::path::Path;

fn main() {
    let mut failed = 0usize;
    for arg in std::env::args().skip(1) {
        let path = Path::new(&arg);
        match vw_formats::doc::parse_file(path, None) {
            Ok(arena) => {
                let checks: Vec<String> = arena
                    .checks()
                    .iter()
                    .map(|check| format!("{}={}", check.id, if check.ok { "ok" } else { "FAIL" }))
                    .collect();
                let bad = arena.checks().iter().filter(|check| !check.ok).count();
                failed += bad;
                println!(
                    "{}\tnodes={}\tbad={}\t[{}]",
                    arg,
                    arena.total_nodes(),
                    bad,
                    checks.join(" ")
                );
            }
            Err(err) => {
                failed += 1;
                println!("{arg}\tERROR {err}");
            }
        }
    }
    if failed > 0 {
        std::process::exit(1);
    }
}
