//! BST 探针：打印记录数、闭合位置与未知标签位置。
use std::path::Path;
use formats::document::bst;

fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).expect("读取失败");
        let stage = bst::parse(&bytes);
        let name = Path::new(&path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        println!(
            "{name}: 记录={} 碰撞板={} 地形={} 结束={:#x}/{:#x} {}",
            stage.records.len(),
            stage.plates.len(),
            stage.quads.len(),
            stage.end,
            bytes.len(),
            match stage.unknown_tag {
                Some((at, tag)) => format!("未知标签 0x{tag:02x} @0x{at:x}"),
                None => "闭合".to_string(),
            }
        );
    }
}
