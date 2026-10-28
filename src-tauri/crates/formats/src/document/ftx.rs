//! FTEX 容器的结构视图：头 + 页目录。
//!
//! 页负载是 NVN 私有纹理块，这里只给区间，不做解码。

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;

const HEADER_SIZE: usize = 0x20;
const PAGE_STRIDE: usize = 0x30;
const MAX_PAGES: u32 = 256;

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("FTEX");

    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < HEADER_SIZE {
        builder.warn("文件不足 0x20 字节，无法读取 FTEX 头");
        builder.mark(root, DocMark::Error);
        return builder.finish();
    }

    let magic = rdr::ascii(&bytes[..4]);
    let data_size = rdr::u32_at(bytes, 4).unwrap_or(0);
    let header_size = rdr::u32_at(bytes, 8).unwrap_or(0);
    let page_count = rdr::u32_at(bytes, 0x0c).unwrap_or(0);

    let header = builder.group(root, Some("header"), "Header");
    builder.field(header, "magic", magic.clone());
    builder.field(header, "dataSize", format!("{data_size} (0x{data_size:x})"));
    builder.field(
        header,
        "headerSize",
        format!("{header_size} (0x{header_size:x})"),
    );
    builder.field(header, "pageCount", page_count.to_string());
    builder.field(header, "reserved", rdr::hex(&bytes[0x10..HEADER_SIZE]));

    builder.check(
        "ftx.magic",
        "魔数为 FTEX",
        magic == "FTEX",
        Some(format!("读到 {magic}")),
    );

    let expected = data_size as usize + header_size as usize + 0x10;
    builder.check(
        "ftx.payload",
        "dataSize + headerSize + 0x10 == 文件长",
        expected == bytes.len(),
        Some(format!("{expected:#x} vs {:#x}", bytes.len())),
    );

    let directory_end = HEADER_SIZE + page_count as usize * PAGE_STRIDE;
    builder.check(
        "ftx.directory",
        "headerSize 覆盖页目录",
        header_size as usize >= directory_end,
        Some(format!("headerSize {header_size:#x} / 目录末尾 {directory_end:#x}")),
    );

    let pages = builder.array(root, Some("pages"), format!("Pages ({page_count})"));
    let shown = page_count.min(MAX_PAGES);
    if page_count > MAX_PAGES {
        builder.truncate();
        builder.warn(format!("页目录只展示前 {MAX_PAGES} / {page_count} 页"));
    }

    for index in 0..shown {
        let base = HEADER_SIZE + index as usize * PAGE_STRIDE;
        let Some(name_bytes) = bytes.get(base..base + 0x20) else {
            builder.warn(format!("页 {index} 的目录项越界"));
            break;
        };
        let name = rdr::c_string(name_bytes);
        let key = index.to_string();
        let group = builder.group(pages, Some(key.as_str()), format!("Page {index}"));
        builder.field(group, "name", if name.is_empty() { "—".into() } else { name });
        builder.field(
            group,
            "u32@0x20",
            rdr::u32_at(bytes, base + 0x20)
                .map(|v| format!("0x{v:08X}"))
                .unwrap_or_else(|| "—".into()),
        );
        builder.field(
            group,
            "u16@0x24",
            rdr::u16_at(bytes, base + 0x24)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into()),
        );
        builder.field(
            group,
            "u16@0x26",
            rdr::u16_at(bytes, base + 0x26)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into()),
        );
        builder.field(
            group,
            "timestamp",
            rdr::hex(&bytes[base + 0x28..base + 0x2e]),
        );
        builder.bytes(group, Some("entry"), "目录项", base as u64, PAGE_STRIDE as u64);
    }

    builder.finish()
}
