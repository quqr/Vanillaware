//! MBS 动画族格式：分节表驱动的解析、V55 模型与 Quad 转换。
//!
//! 决策见 `docs/adr/0004` ~ `docs/adr/0009`；术语见 `CONTEXT.md`。

pub mod idtag;
pub mod matrix;
pub mod parse;
pub mod profile;
pub mod quad;
pub mod v55;
pub mod validate;
pub mod verify;

pub use parse::{parse_bytes, parse_file};
pub use quad::{convert, Quad};
pub use v55::V55;
pub use validate::{compare_golden, self_check, Report};
