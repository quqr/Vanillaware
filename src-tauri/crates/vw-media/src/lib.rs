//! 缩略图磁盘缓存与 PNG 编码。
//!
//! 目录布局：`<root>/<fnv1a64(小写路径)>/{meta.json, 0.png, 1.png …}`
//! 失效判定：`schema + size + mtime`；写入一律 `.tmp → rename` 原子替换。

pub mod cache;
pub mod encode;
pub mod hash;

pub use cache::{CacheInfo, ThumbCache};
pub use vw_formats::ftx::TextureInfo;
