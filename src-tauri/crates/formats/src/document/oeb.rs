//! OEB —— 13 Sentinels 的「NPC 摆位」容器（`EX DATA/MobArrange/`，107 个）。
//!
//! 布局与 `.osb` 完全同族（见 [`super::bundle`]），只是节点类型编号不同：
//!
//! ```text
//! type 4   arrange_root   根
//! type 26  group          Caption 是分组名（如「通知」）
//! type 27  entry          Caption 是场景名，Label 是镜头/图层标识
//! type 28  slot           一个角色站位
//! ```
//!
//! 槽位（type 28）的 body 固定 228 字节，0x52 起是浮点：
//!
//! ```text
//! +0x52 f32 scaleX   +0x56 f32 scaleY
//! +0x5a f32 x        +0x5e f32 y        +0x62 f32 z
//! ```
//!
//! 实测 `Test_Mob_64` 有 46 个槽位，caption 是「女子3 (鞄) 待機」这类中文/日文描述，
//! label 是角色编号（`JK03B`），坐标与同场景的 `.osb` 舞台是同一套单位。

use super::arena::DocArena;
use super::bundle::{self, Spec};
use super::rdr;
use super::text;

/// 节点类型表。
pub static TYPES: &[(u32, &str)] = &[
    (4, "arrange_root"),
    (26, "group"),
    (27, "entry"),
    (28, "slot"),
];

const SLOT_TYPE: u32 = 28;
const SLOT_BODY: usize = 228;

/// 容器描述。
pub const SPEC: Spec = Spec {
    magic: b"OEB\0",
    title: "OEB",
    types: TYPES,
};

/// 一个角色站位。
pub struct Slot {
    pub offset: usize,
    pub caption: String,
    pub label: String,
    pub scale: [f32; 2],
    pub position: [f32; 3],
}

/// 文档树。
pub fn build(bytes: &[u8]) -> DocArena {
    bundle::build(bytes, &SPEC)
}

/// 角色站位列表（内容视图用）。
pub fn slots(bytes: &[u8]) -> Vec<Slot> {
    let Some(parsed) = bundle::parse(bytes, &SPEC) else {
        return Vec::new();
    };
    parsed
        .nodes
        .iter()
        .filter(|node| node.type_id == SLOT_TYPE && node.body_size >= SLOT_BODY - 0x52)
        .map(|node| {
            let body = &bytes[node.body_at..node.body_at + node.body_size];
            Slot {
                offset: node.at,
                caption: node.caption.clone(),
                label: node.label.clone(),
                scale: [
                    rdr::f32_at(body, 0x52).unwrap_or(0.0),
                    rdr::f32_at(body, 0x56).unwrap_or(0.0),
                ],
                position: [
                    rdr::f32_at(body, 0x5a).unwrap_or(0.0),
                    rdr::f32_at(body, 0x5e).unwrap_or(0.0),
                    rdr::f32_at(body, 0x62).unwrap_or(0.0),
                ],
            }
        })
        .collect()
}

/// 资源名（`.mbs` / `.ftx` 等）。
pub fn resources(bytes: &[u8]) -> Vec<String> {
    bundle::parse(bytes, &SPEC)
        .map(|parsed| parsed.resources)
        .unwrap_or_default()
}

/// 与 `text` 模块保持同一套 Shift-JIS 口径（供测试引用）。
pub fn decode_slot(raw: &[u8]) -> String {
    text::sjis_slot(raw)
}
