//! 预览场景的 DTO：前端直接消费的「已解引用」图层结构。
//!
//! 矩阵故意保留 16 项（空数组表示单位矩阵）：前端按行主序直接用它变换顶点。

use serde::Serialize;

/// 预览所需的一切。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneDto {
    /// `.mbs` 绝对路径。
    pub mbs_path: String,
    /// 解析用的平台标签。
    pub tag: String,
    /// 图集帧元信息，下标即 `texId`。
    pub atlas: Vec<AtlasFrameDto>,
    /// `keyframe[s6]`，空位为 `null`（前端按 s6 下标取）。
    pub keyframe: Vec<Option<KeyframeDto>>,
    /// `hitbox[s6]`，空位为 `null`。
    pub hitbox: Vec<Option<HitboxDto>>,
    /// 动画列表（`animation[sa]` 中的实填项）。
    pub animations: Vec<AnimationDto>,
    /// 骨骼列表（`skeleton[s9]` 中的实填项）。
    pub skeletons: Vec<SkeletonDto>,
    /// 自洽性检查结果；非空即说明本文件的转换结果有结构问题。
    pub self_check: Vec<String>,
}

/// 图集一帧（不含像素）。
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

/// 一个判定框帧的全部图层。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyframeDto {
    pub name: String,
    pub flags: String,
    /// 稠密图层数组，空位为 `null`（与 Quad 的 `0` 占位同义）。
    pub layers: Vec<Option<LayerDto>>,
}

/// 一个图层。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerDto {
    /// s4 的 flags 串（`debug[0]`）。
    pub flags: String,
    /// 目标四边形，8 个 float。
    pub dst: Vec<f32>,
    /// 源四边形（UV），8 个 float；无纹理时为 `null`。
    pub src: Option<Vec<f32>>,
    pub tex_id: Option<i64>,
    pub blend_id: i64,
    /// 雾色：0 或 4 个 `#rrggbbaa`（单色时重复 4 次），便于着色器统一取。
    pub fog: Option<Vec<String>>,
    /// `COLOR_%x` 着色标记。
    pub colorize: Option<String>,
    /// `bitflag_attr` 的产物，统一成数组。
    pub attribute: Option<Vec<String>>,
}

/// 一个命中框帧。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HitboxDto {
    pub name: String,
    pub layers: Vec<Option<HitboxLayerDto>>,
}

/// 命中框图层。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HitboxLayerDto {
    pub flags: String,
    pub quad: Vec<f32>,
}

/// 一根骨的整条时间线。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationDto {
    /// 在 `animation` 数组里的下标（同时是 `attach.id`）。
    pub id: i64,
    pub name: String,
    /// 循环起点（时间线内顺序下标）；`-1` 表示不循环。
    pub loop_id: i64,
    pub frames: Vec<FrameDto>,
}

/// 时间线一帧。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameDto {
    /// 时长（帧）。
    pub time: i64,
    pub flags: String,
    /// 要挂载的 s6 下标与类型（`slot` / `keyframe` / `hitbox`）。
    pub attach: Option<AttachDto>,
    /// s7 的雾色；`null` 表示 `#ffffffff`（不染色）。
    pub color: Option<String>,
    /// 16 个 f64（行主序）；空数组表示单位矩阵。
    pub matrix: Vec<f64>,
}

/// 挂载点。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachDto {
    pub kind: String,
    pub id: i64,
}

/// 一个 s9 的骨列表。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkeletonDto {
    pub name: String,
    pub bones: Vec<AttachDto>,
}
