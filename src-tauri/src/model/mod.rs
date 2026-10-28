//! 跨进程数据结构（DTO）。
//!
//! 这一层只有 `struct` 与 `serde` 属性，没有任何解析或 IO 逻辑：
//! 字段含义由 `formats` / `catalog` 定义，这里只负责「怎么传给前端」。

pub mod animation;
pub mod audio;
pub mod scene;
pub mod texture;
pub mod video;

pub use animation::ConvertDto;
pub use audio::{AudioBundleDto, AudioClipDto, AudioCodecCountDto, AudioTrackDto};
pub use scene::{
    AnimationDto, AtlasFrameDto, AttachDto, FrameDto, HitboxDto, HitboxLayerDto, KeyframeDto,
    LayerDto, SceneDto, SkeletonDto,
};
pub use texture::{FrameListDto, FrameMetaDto};
pub use video::UsmVideoDto;
