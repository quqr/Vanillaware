//! ORB —— 13 Sentinels 的「轮次镜头表」（`EX DATA/Round/`，15 个）。
//!
//! 与 `.osb` / `.oeb` 同族（见 [`super::bundle`]）：
//!
//! ```text
//! type 1   round_tree   根，Caption 是「ラウンドツリー」
//! type 29  round        一轮，Caption 是「ラウンド」
//! type 30  camera       一次镜头/舞台切换，Caption 是舞台名（「新校舎廊下」），Label 是序号
//! ```
//!
//! 资源池里装的是**完整资产路径**（`Stage\sinkosya_roka00.osb`、`Bg/*.mbs`、`Bg/*.ftx`），
//! 所以这个文件本身就是一张「这一轮用到哪些舞台与背景」的清单。
//!
//! 镜头节点（type 30）body 0x52 起是三个 f32 位置。

use super::arena::DocArena;
use super::bundle::{self, Spec};
use super::rdr;

/// 节点类型表。
pub static TYPES: &[(u32, &str)] = &[(1, "round_tree"), (29, "round"), (30, "camera")];

const CAMERA_TYPE: u32 = 30;

/// 容器描述。
pub const SPEC: Spec = Spec {
    magic: b"ORB\0",
    title: "ORB",
    types: TYPES,
};

/// 一次镜头。
pub struct Camera {
    pub offset: usize,
    pub stage: String,
    pub label: String,
    pub position: [f32; 3],
}

/// 文档树。
pub fn build(bytes: &[u8]) -> DocArena {
    bundle::build(bytes, &SPEC)
}

/// 镜头列表（内容视图用）。
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

/// 资源名（完整工程路径）。
pub fn resources(bytes: &[u8]) -> Vec<String> {
    bundle::parse(bytes, &SPEC)
        .map(|parsed| parsed.resources)
        .unwrap_or_default()
}
