//! ACB 声音目录（`.acb`，CRI ADX2 的 cue sheet）。
//!
//! ACB 本体是一串 `@UTF` 表（根表 + 每类索引各一张：Cue / CueName /
//! Waveform / Synth / Track / Sequence / Aisac …），表后**可以**再附加一个
//! AFS2 音频包；本语料里 `se.acb`、`voice_*.acb` 都附加了，`bgm.acb` 也附加了。
//!
//! ```text
//! +0x00  "@UTF" 根表（1 行 × 96 列，CueTable 等列用 RawData 指向子表）
//! ...    子表若干，顺序排布
//! +off   "AFS2" 内嵌音频包（可选）
//! ```
//!
//! 表结构与列类型全部由 [`super::utf`] 解析；这里只补两件事：
//! **内嵌 AFS2 的定位**，以及「表区在音频包之前结束」的边界校验。

use super::arena::{Builder, DocArena, DocMark};
use super::awb;
use super::utf;

pub fn build(bytes: &[u8], title: &str) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root(title);
    builder.field(root, "size", bytes.len().to_string());

    let tables = utf::append(&mut builder, root, bytes);

    // 内嵌音频包：ACB 允许把 wave bank 直接追加在表后面。
    let bank_at = find(bytes, b"AFS2");
    match bank_at {
        Some(at) => {
            let group = builder.group(root, Some("audioBank"), "Audio Bank (AFS2)");
            builder.field(group, "offset", format!("0x{at:x}"));
            match awb::parse(bytes, at) {
                Some(bank) => {
                    builder.field(group, "entries", bank.entries.len().to_string());
                    builder.field(group, "bytes", bytes.len().saturating_sub(at).to_string());
                    builder.check(
                        "acb.bank",
                        "AFS2 落在表区之后",
                        bank_at.unwrap_or(0) > 0,
                        Some(format!("@{at:#x}")),
                    );
                }
                None => {
                    builder.warn(format!("AFS2 @0x{at:x} 头不闭合"));
                    builder.mark(group, DocMark::Warn);
                }
            }
        }
        None => {
            builder.field(root, "audioBank", "—（纯 cue sheet）");
        }
    }

    builder.check(
        "acb.tables",
        "至少一张 @UTF 表",
        tables > 0,
        Some(format!("{tables} 张")),
    );

    builder.finish()
}

fn find(bytes: &[u8], needle: &[u8]) -> Option<usize> {
    if bytes.len() < needle.len() {
        return None;
    }
    bytes
        .windows(needle.len())
        .position(|window| window == needle)
}
