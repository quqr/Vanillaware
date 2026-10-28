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
    pub count: u32,
    pub material: i32,
    pub page: i32,
    pub rgba: [u8; 4],
    pub tex_ref: i32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VizMeshDto {
    pub positions: Vec<f32>,
    pub indices: Vec<u32>,
    pub uvs: Vec<f32>,
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
    pub transform: [f32; 16],
    pub mesh: Option<VizMeshDto>,
    pub mesh_ref: Option<u32>,
    pub reference: Option<VizReferenceDto>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VizSceneDto {
    /// [minX, minY, minZ, maxX, maxY, maxZ]
    pub bounds: [f32; 6],
    pub nodes: Vec<VizNodeDto>,
    pub references: Vec<VizReferenceDto>,
    pub truncated: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeDto {
    pub id: u32,
    pub label: String,
    /// file / entry / script / external
    pub kind: String,
    pub detail: Option<String>,
    pub path: Option<String>,
    pub offset: Option<u64>,
    pub status: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdgeDto {
    pub from: u32,
    pub to: u32,
    pub label: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileGraphDto {
    pub nodes: Vec<GraphNodeDto>,
    pub edges: Vec<GraphEdgeDto>,
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
