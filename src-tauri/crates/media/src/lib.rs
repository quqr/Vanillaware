//! 缩略图磁盘缓存、PNG 编码与音频解码。
//!
//! 目录布局：`<root>/<fnv1a64(小写路径)>/{meta.json, 0.png, 1.png …}`
//! 失效判定：`schema + size + mtime`；写入一律 `.tmp → rename` 原子替换。

pub mod audio;
pub mod cache;
pub mod content;
pub mod encode;
pub mod hash;
pub mod video;

pub use audio::{
    AudioError, Bundle as AudioBundle, Clip as AudioClip, Codec as AudioCodec,
    TrackInfo as AudioTrackInfo,
};
pub use cache::{CacheInfo, ThumbCache};
pub use content::{ContentDto, SectionDto, SeriesDto};
pub use video::{UsmIndex, VideoChunk, VideoFrame};
pub use formats::ftx::TextureInfo;
