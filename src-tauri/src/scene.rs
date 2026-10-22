//! 预览场景的 DTO 组装：把 `vw-formats` 的 Quad / 图集摊平成前端能直接画的结构。
//!
//! 这里只做「模型 → 视图」的搬运，不含任何格式判断：字段含义全部由 `vw-formats` 定义，
//! 前端拿到的是**已经解好引用**的图层（`tex_id` 指向 `atlas` 下标、`attach` 指向 keyframe/hitbox 下标）。
//!
//! 矩阵故意保留 16 项（空数组表示单位矩阵）：Canvas 端按行主序直接喂给 `setTransform` 的前四项。

use std::path::Path;

use serde::Serialize;
use vw_formats::atlas::Atlas;
use vw_formats::mbs::quad::{
    Animation, Attribute, Attach, Bone, Hitbox, HitboxLayer, Keyframe, KeyframeLayer, Pad, Quad,
    TimelineEntry,
};
use vw_formats::mbs::v55::FogQuad;
use vw_formats::Result;

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

/// 复用一个 `.mbs` 的解析结果：Quad + 图集元信息。
pub struct LoadedMbs {
    pub quad: Quad,
    pub atlas: Option<Atlas>,
}

/// 解析 + 转换 + 打开图集。
pub fn load(mbs_path: &Path, tag: &str) -> Result<LoadedMbs> {
    let v55 = vw_formats::mbs::parse_file(mbs_path, tag)?;
    let quad = vw_formats::mbs::convert(&v55)?;
    let atlas = Atlas::open(mbs_path)?;
    Ok(LoadedMbs { quad, atlas })
}

impl LoadedMbs {
    /// 摊平成前端 DTO。
    pub fn scene(&self, mbs_path: &Path, tag: &str) -> SceneDto {
        let atlas = self
            .atlas
            .as_ref()
            .map(|atlas| {
                atlas
                    .info()
                    .iter()
                    .enumerate()
                    .map(|(index, info)| AtlasFrameDto {
                        tex_id: index as i64,
                        width: info.width,
                        height: info.height,
                        format: info.format,
                        format_name: info.format_name(),
                        channels: info.channels,
                    })
                    .collect()
            })
            .unwrap_or_default();

        SceneDto {
            mbs_path: mbs_path.display().to_string(),
            tag: tag.to_string(),
            atlas,
            keyframe: self.quad.keyframe.iter().map(keyframe_dto).collect(),
            hitbox: self.quad.hitbox.iter().map(hitbox_dto).collect(),
            animations: self
                .quad
                .animation
                .iter()
                .enumerate()
                .filter_map(|(index, cell)| match cell {
                    Pad::Filled(animation) => Some(animation_dto(index as i64, animation)),
                    Pad::Empty(_) => None,
                })
                .collect(),
            skeletons: self
                .quad
                .skeleton
                .iter()
                .filter_map(|cell| match cell {
                    Pad::Filled(skeleton) => Some(SkeletonDto {
                        name: skeleton.name.clone(),
                        bones: skeleton.bone.iter().map(bone_dto).collect(),
                    }),
                    Pad::Empty(_) => None,
                })
                .collect(),
            self_check: vw_formats::mbs::self_check(&self.quad),
        }
    }
}

fn keyframe_dto(cell: &Pad<Keyframe>) -> Option<KeyframeDto> {
    match cell {
        Pad::Empty(_) => None,
        Pad::Filled(keyframe) => Some(KeyframeDto {
            name: keyframe.name.clone(),
            flags: keyframe.debug.clone(),
            layers: keyframe.layer.iter().map(layer_dto).collect(),
        }),
    }
}

fn layer_dto(cell: &Pad<KeyframeLayer>) -> Option<LayerDto> {
    match cell {
        Pad::Empty(_) => None,
        Pad::Filled(layer) => Some(LayerDto {
            flags: layer.debug.0.clone(),
            dst: layer.dstquad.clone(),
            src: layer.srcquad.clone(),
            tex_id: layer.tex_id,
            blend_id: layer.blend_id,
            fog: layer.fogquad.as_ref().map(fog_dto),
            colorize: layer.colorize.clone(),
            attribute: layer.attribute.as_ref().map(attribute_dto),
        }),
    }
}

/// 单色雾色展开成 4 份，前端只取 `[0]` 也不会错。
fn fog_dto(fog: &FogQuad) -> Vec<String> {
    match fog {
        FogQuad::One(color) => vec![color.clone(); 4],
        FogQuad::Four(colors) => colors.to_vec(),
    }
}

fn attribute_dto(attribute: &Attribute) -> Vec<String> {
    match attribute {
        Attribute::One(name) => vec![name.clone()],
        Attribute::Many(names) => names.clone(),
    }
}

fn hitbox_dto(cell: &Pad<Hitbox>) -> Option<HitboxDto> {
    match cell {
        Pad::Empty(_) => None,
        Pad::Filled(hitbox) => Some(HitboxDto {
            name: hitbox.name.clone(),
            layers: hitbox
                .layer
                .iter()
                .map(|cell| match cell {
                    Pad::Empty(_) => None,
                    Pad::Filled(layer) => Some(hitbox_layer_dto(layer)),
                })
                .collect(),
        }),
    }
}

fn hitbox_layer_dto(layer: &HitboxLayer) -> HitboxLayerDto {
    HitboxLayerDto {
        flags: layer.debug.clone(),
        quad: layer.hitquad.clone(),
    }
}

fn animation_dto(id: i64, animation: &Animation) -> AnimationDto {
    AnimationDto {
        id,
        name: animation.name.clone(),
        loop_id: animation.loop_id,
        frames: animation.timeline.iter().map(frame_dto).collect(),
    }
}

fn frame_dto(entry: &TimelineEntry) -> FrameDto {
    FrameDto {
        time: entry.time,
        flags: entry.debug.clone(),
        attach: entry.attach.as_ref().map(attach_dto),
        color: entry.color.clone(),
        matrix: entry.matrix.clone(),
    }
}

fn attach_dto(attach: &Attach) -> AttachDto {
    AttachDto {
        kind: attach.kind.clone(),
        id: attach.id,
    }
}

fn bone_dto(bone: &Bone) -> AttachDto {
    attach_dto(&bone.attach)
}
