//! 文件系统层：目录读取、格式注册表与内存索引。
//!
//! 这个 crate 只做文件系统访问与**注册表过滤**，不解析任何文件内容——它连 `formats`
//! 都不依赖，想在这里解码都做不到。

pub mod dto;
pub mod format;
pub mod fslist;
pub mod game;
pub mod index;

pub use dto::*;
pub use format::{Format, Kind, FORMATS};
pub use game::{detect as detect_game, GameId, GameProfileDto};
pub use fslist::{absolutize, file_meta, list_dir, normalize, read_head, FsError};
pub use index::{build as build_index, search as search_index, IndexEntry, SEARCH_LIMIT};
