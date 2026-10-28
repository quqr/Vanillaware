//! M0 门禁：用真实样本验证 .ksy 生成的解析器能读对结构字段。
//! 只验证「结构」，不涉及像素解码。

use kaitai::{BytesReader, KStruct};
use schema::gen::ftex_switch::FtexSwitch;

const SAMPLE: &str = r"F:\Codes\13\13rim ex\EX DATA\BattleEffect\Barrier00.ftx";

#[test]
fn parses_barrier00_header() {
    if !std::path::Path::new(SAMPLE).exists() {
        eprintln!("skip: sample not found at {SAMPLE}");
        return;
    }

    let io = BytesReader::open(SAMPLE).expect("open");
    let root = <FtexSwitch as KStruct>::read_into::<_, FtexSwitch>(&io, None, None).expect("parse FtexSwitch");

    assert_eq!(*root.version(), 0x0004_0140);
    assert_eq!(*root.header_size(), 0x80);
    assert_eq!(*root.texture_count(), 1);

    let entries = root.entries();
    assert_eq!(entries.len(), 1);
    let name = String::from_utf8_lossy(&entries[0].name()).trim_end_matches('\0').to_string();
    assert_eq!(name, "barrier00.nvt");

    let blocks = root.blocks();
    assert_eq!(blocks.len(), 1);
    let block = &blocks[0];
    assert_eq!(*block.nvt_size(), 0x0004_0100);
    assert_eq!(*block.nvt_offset(), 0x40);

    let nvt = block.nvt();
    assert_eq!(*nvt.format(), 0x4D); // BC7
    assert_eq!(*nvt.width(), 512);
    assert_eq!(*nvt.height(), 512);
    assert_eq!(*nvt.data_offset(), 0x100);
    assert_eq!(*nvt.data_size(), 0x40000);
    assert_eq!(nvt.data().len(), 0x40000);
}
