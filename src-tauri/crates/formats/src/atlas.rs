use std::path::{Path, PathBuf};

use crate::{ftx, DecodedTexture, Error, Result};

pub struct Atlas {
    path: PathBuf,
    info: Vec<ftx::TextureInfo>,
}

impl Atlas {
    pub fn open(mbs_path: &Path) -> Result<Option<Atlas>> {
        let Some(path) = sibling_container(mbs_path)? else {
            return Ok(None);
        };
        let info = ftx::probe_file(&path)?;
        Ok(Some(Atlas { path, info }))
    }

    pub fn len(&self) -> usize {
        self.info.len()
    }

    pub fn is_empty(&self) -> bool {
        self.info.is_empty()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn info(&self) -> &[ftx::TextureInfo] {
        &self.info
    }

    pub fn decode(&self, tex_id: i64) -> Result<DecodedTexture> {
        if tex_id < 0 || tex_id as usize >= self.info.len() {
            return Err(Error::BadData(format!(
                "tex_id {tex_id} is outside the atlas range 0..{} ({})",
                self.info.len(),
                self.path.display()
            )));
        }
        ftx::decode_file_frame(&self.path, tex_id as usize)
    }
}

pub fn sibling_container(mbs_path: &Path) -> Result<Option<PathBuf>> {
    let stem = mbs_path
        .file_stem()
        .ok_or_else(|| Error::BadData("mbs has no file name".to_string()))?;
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
                "{} has {} candidate texture containers, cannot pick one: {:?}",
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
