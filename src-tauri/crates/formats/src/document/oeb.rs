//! ```text
//! ```
//! ```text
//! +0x52 f32 scaleX   +0x56 f32 scaleY
//! +0x5a f32 x        +0x5e f32 y        +0x62 f32 z
//! ```

use super::arena::DocArena;
use super::bundle::{self, Spec};
use super::rdr;
use super::text;

pub static TYPES: &[(u32, &str)] = &[
    (4, "arrange_root"),
    (26, "group"),
    (27, "entry"),
    (28, "slot"),
];

const SLOT_TYPE: u32 = 28;
const SLOT_BODY: usize = 228;

pub const SPEC: Spec = Spec {
    magic: b"OEB\0",
    title: "OEB",
    types: TYPES,
};

pub struct Slot {
    pub offset: usize,
    pub caption: String,
    pub label: String,
    pub scale: [f32; 2],
    pub position: [f32; 3],
}

pub fn build(bytes: &[u8]) -> DocArena {
    bundle::build(bytes, &SPEC)
}

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

pub fn resources(bytes: &[u8]) -> Vec<String> {
    bundle::parse(bytes, &SPEC)
        .map(|parsed| parsed.resources)
        .unwrap_or_default()
}

pub fn decode_slot(raw: &[u8]) -> String {
    text::sjis_slot(raw)
}
