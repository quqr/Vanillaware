use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneDto {
    pub mbs_path: String,
    pub tag: String,
    pub atlas: Vec<AtlasFrameDto>,
    pub keyframe: Vec<Option<KeyframeDto>>,
    pub hitbox: Vec<Option<HitboxDto>>,
    pub animations: Vec<AnimationDto>,
    pub skeletons: Vec<SkeletonDto>,
    pub self_check: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasFrameDto {
    pub tex_id: i64,
    pub width: u32,
    pub height: u32,
    pub format: u16,
    pub format_name: &'static str,
    pub channels: u8,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyframeDto {
    pub name: String,
    pub flags: String,
    pub layers: Vec<Option<LayerDto>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerDto {
    pub flags: String,
    pub dst: Vec<f32>,
    pub src: Option<Vec<f32>>,
    pub tex_id: Option<i64>,
    pub blend_id: i64,
    pub fog: Option<Vec<String>>,
    pub colorize: Option<String>,
    pub attribute: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HitboxDto {
    pub name: String,
    pub layers: Vec<Option<HitboxLayerDto>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HitboxLayerDto {
    pub flags: String,
    pub quad: Vec<f32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationDto {
    pub id: i64,
    pub name: String,
    pub loop_id: i64,
    pub frames: Vec<FrameDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameDto {
    pub time: i64,
    pub flags: String,
    pub attach: Option<AttachDto>,
    pub color: Option<String>,
    pub matrix: Vec<f64>,
    pub matrix_mix: i64,
    pub quad_mix: i64,
    pub in_rate: i64,
    pub transform: Vec<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachDto {
    pub kind: String,
    pub id: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkeletonDto {
    pub name: String,
    pub bones: Vec<AttachDto>,
}
