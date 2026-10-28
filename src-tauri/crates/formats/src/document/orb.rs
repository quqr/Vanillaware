//! ```text
//! ```

use super::arena::DocArena;
use super::bundle::{self, Spec};
use super::rdr;

pub static TYPES: &[(u32, &str)] = &[(1, "round_tree"), (29, "round"), (30, "camera")];

const CAMERA_TYPE: u32 = 30;

pub const SPEC: Spec = Spec {
    magic: b"ORB\0",
    title: "ORB",
    types: TYPES,
};

pub struct Camera {
    pub offset: usize,
    pub stage: String,
    pub label: String,
    pub position: [f32; 3],
}

pub fn build(bytes: &[u8]) -> DocArena {
    bundle::build(bytes, &SPEC)
}

pub fn cameras(bytes: &[u8]) -> Vec<Camera> {
    let Some(parsed) = bundle::parse(bytes, &SPEC) else {
        return Vec::new();
    };
    parsed
        .nodes
        .iter()
        .filter(|node| node.type_id == CAMERA_TYPE)
        .map(|node| {
            let body = &bytes[node.body_at..node.body_at + node.body_size];
            Camera {
                offset: node.at,
                stage: node.caption.clone(),
                label: node.label.clone(),
                position: [
                    rdr::f32_at(body, 0x52).unwrap_or(0.0),
                    rdr::f32_at(body, 0x56).unwrap_or(0.0),
                    rdr::f32_at(body, 0x5a).unwrap_or(0.0),
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
