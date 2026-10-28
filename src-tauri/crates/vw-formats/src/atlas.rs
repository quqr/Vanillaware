//! 图集：`.mbs` 的同目录同名 `.ftx` / `.ftp` 容器，按 `tex_id` 位置索引（ADR-0007）。
//!
//! `s4.tex` 的下标就是容器里**可解码帧**的顺序号：PHP `FTEX_swit_unicorn.php` 也是按同一顺序
//! 落盘成 `<stem>.<i>.nvt`，再由 `clut2png` 转成 `<stem>.<i>.png`。
//!
//! 按需解码：只解 `tex_id` 指定的那一帧，不整包展开——预览器每帧只用到少数几张。

use std::path::{Path, PathBuf};

use crate::{ftx, DecodedTexture, Error, Result};

/// 一个 `.mbs` 对应的纹理容器。
pub struct Atlas {
    path: PathBuf,
    info: Vec<ftx::TextureInfo>,
}

impl Atlas {
    /// 打开 `.mbs` 旁边的纹理容器；没有同名容器时返回 `None`。
    ///
    /// 只读结构与尺寸，不解码像素。
    pub fn open(mbs_path: &Path) -> Result<Option<Atlas>> {
        let Some(path) = sibling_container(mbs_path)? else {
            return Ok(None);
        };
        let info = ftx::probe_file(&path)?;
        Ok(Some(Atlas { path, info }))
    }

    /// 可解码帧数，即 `tex_id` 的合法上界。
    pub fn len(&self) -> usize {
        self.info.len()
    }

    /// 是否一张纹理都没有。
    pub fn is_empty(&self) -> bool {
        self.info.is_empty()
    }

    /// 容器路径（错误信息里用）。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 所有帧的元信息（不含像素），供 UI 列表用。
    pub fn info(&self) -> &[ftx::TextureInfo] {
        &self.info
    }

    /// 解码 `tex_id` 指向的那一帧。
    ///
    /// 越界**报错**而不是夹取：预览宁可显示「图集缺该帧」，也不能静默画错像素。
    pub fn decode(&self, tex_id: i64) -> Result<DecodedTexture> {
        if tex_id < 0 || tex_id as usize >= self.info.len() {
            return Err(Error::BadData(format!(
                "tex_id {tex_id} 超出图集范围 0..{}（{}）",
                self.info.len(),
                self.path.display()
            )));
        }
        ftx::decode_file_frame(&self.path, tex_id as usize)
    }
}

/// `.mbs` → 同目录的纹理容器。
///
/// 语料里两种命名并存（实测 `000` 树：357 个同名、198 个带数字后缀、2 个两者皆有、165 个无纹理）：
/// 1. **同名**：`Fighter_M.mbs` ↔ `Fighter_M.ftx`；
/// 2. **同名 + 数字后缀**：`Fighter_M.mbs` ↔ `Fighter_M00.ftx`（导出时命名的变体号）。
///
/// 同名优先；否则取「`<stem>` + 纯数字」的唯一候选。候选多于一个时**报错而不猜**
/// （例如 `Eltlinde_F` 同时有 `00` 与 `10` 两个容器）——预览宁可报缺，也不画错像素。
pub fn sibling_container(mbs_path: &Path) -> Result<Option<PathBuf>> {
    let stem = mbs_path
        .file_stem()
        .ok_or_else(|| Error::BadData("mbs 没有文件名".to_string()))?;
    let Some(dir) = mbs_path.parent() else {
        return Ok(None);
    };
    let stem = stem.to_string_lossy();

    for extension in ["ftx", "ftp"] {
        let candidate = dir.join(stem.as_ref()).with_extension(extension);
        if candidate.is_file() {
            return Ok(Some(candidate));
        }
    }

    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(None);
    };
    let mut candidates: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let Some(extension) = path.extension() else {
                return false;
            };
            let extension = extension.to_string_lossy();
            if extension != "ftx" && extension != "ftp" {
                return false;
            }
            path.file_stem()
                .map(|other| other.to_string_lossy().into_owned())
                .and_then(|other| other.strip_prefix(stem.as_ref()).map(str::to_string))
                .is_some_and(|suffix| {
                    !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit())
                })
        })
        .collect();
    candidates.sort();

    match candidates.len() {
        0 => Ok(None),
        1 => Ok(candidates.pop()),
        _ => Err(Error::Unsupported {
            what: "atlas",
            detail: format!(
                "{} 有 {} 个候选纹理容器，无法确定归属：{:?}",
                mbs_path.display(),
                candidates.len(),
                candidates
                    .iter()
                    .filter_map(|path| path.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
            ),
        }),
    }
}
