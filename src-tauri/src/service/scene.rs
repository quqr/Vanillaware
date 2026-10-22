//! 预览场景服务：把 `.mbs` 解析成 Quad，再摊平成前端 DTO。
//!
//! 这里只做「模型 → 视图」的搬运，不含任何格式判断：字段含义全部由 `vw-formats` 定义。
//! 解析结果由 [`super::parse::ParseCache`] 缓存，播放与结构两个查看器共用同一份。

use std::path::Path;

use vw_formats::Result;
use vw_formats::atlas::Atlas;
use vw_formats::mbs::quad::{
    Animation, Attach, Attribute, Bone, Hitbox, HitboxLayer, Keyframe, KeyframeLayer, Pad, Quad,
    TimelineEntry,
};
use vw_formats::mbs::v55::FogQuad;

use crate::model::scene::{
    AnimationDto, AtlasFrameDto, AttachDto, FrameDto, HitboxDto, HitboxLayerDto, KeyframeDto,
    LayerDto, SceneDto, SkeletonDto,
};

/// 一个 `.mbs` 的解析结果：Quad + 图集元信息。
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
