//! 业务服务层：不依赖 Tauri 命令宏，可被命令层与测试直接调用。
//!
//! - [`scene`]：`.mbs` 解析与预览场景组装；
//! - [`parse`]：解析结果 LRU；
//! - [`index`]：目录索引的后台构建与快照。

pub mod index;
pub mod parse;
pub mod scene;

pub use index::IndexService;
pub use parse::ParseCache;
pub use scene::{LoadedMbs, load as load_mbs};
