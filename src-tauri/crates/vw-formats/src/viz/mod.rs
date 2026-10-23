//! 与格式无关的可视化图元层（ADR-0019）。
//!
//! FMD / OSB / RSC 各自投影到同一套场景 → 节点 → 网格/引用，前端只认这一层。

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VizReferenceDto {
    pub path: String,
    pub ext: String,
    /// model / texture / animation / audio / script / text
    pub role: String,
    /// resolved / missing
    pub status: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VizSubRangeDto {
    pub start: u32,
    /// 索引数量（triangle count × 3）。
    pub count: u32,
    pub material: i32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VizMeshDto {
    pub positions: Vec<f32>,
    pub indices: Vec<u32>,
    pub bounds: [f32; 6],
    pub sub_ranges: Vec<VizSubRangeDto>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VizNodeDto {
    pub id: u32,
    pub parent: Option<u32>,
    pub name: String,
    /// group / mesh / sprite / light / camera / reference
    pub kind: String,
    /// 列主序 4×4；首版为屏幕对齐的单位矩阵。
    pub transform: [f32; 16],
    pub mesh: Option<VizMeshDto>,
    pub reference: Option<VizReferenceDto>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VizSceneDto {
    /// [minX, minY, minZ, maxX, maxY, maxZ]
    pub bounds: [f32; 6],
    pub nodes: Vec<VizNodeDto>,
    pub references: Vec<VizReferenceDto>,
    /// 首版为避免 IPC 爆炸设置的顶点/索引上限是否触发了截断。
    pub truncated: bool,
}

pub fn identity() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ]
}
