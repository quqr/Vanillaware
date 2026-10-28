use std::path::Path;
use formats::document::bst;

fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).expect("read failed");
        let stage = bst::parse(&bytes);
        let name = Path::new(&path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        println!(
            "{name}: records={} plates={} terrain={} end={:#x}/{:#x} {}",
            stage.records.len(),
            stage.plates.len(),
            stage.quads.len(),
            stage.end,
            bytes.len(),
            match stage.unknown_tag {
                Some((at, tag)) => format!("unknown tag 0x{tag:02x} @0x{at:x}"),
                None => "closed".to_string(),
            }
        );
    }
}
