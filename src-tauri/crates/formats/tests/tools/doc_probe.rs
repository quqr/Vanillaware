use std::path::Path;

fn main() {
    let mut failed = 0usize;
    for arg in std::env::args().skip(1) {
        let path = Path::new(&arg);
        match formats::document::parse_file(path, None) {
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
