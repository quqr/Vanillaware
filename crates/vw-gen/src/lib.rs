//! ksc 0.11 生成物的容器 crate。
//!
//! `src/gen/` 下的文件全部由 `tools/ksc_gen.py` 从 `spec/ksy/*.ksy` 生成，
//! 任何手改都会在下次生成时被覆盖。要改变解析行为，请改 `.ksy`。

// ksc 生成物不适用项目风格规则，统一豁免，避免生成器格式变化牵动构建警告。
// rustc 侧同样豁免：生成器会产出 `unused_parens`、未使用到的 `_r`/`_prc` 绑定等。
#[allow(clippy::all)]
#[allow(unused_parens)]
#[allow(unused_variables)]
#[allow(dead_code)]
pub mod gen;
