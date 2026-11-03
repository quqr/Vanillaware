use std::path::{Path, PathBuf};

use kaitai::{BytesReader, KStruct};
use schema::gen::fms::Fms;

use crate::{Error, Result};

pub const POOL_DIR: &str = "Fms";
pub const POOL_FILE: &str = "ScriptText.fms";

const DEPTH: usize = 4;
const UTF8_ENCODING: u8 = 3;
const ASCII_ENCODING: u8 = 0;

pub struct TextPool {
    path: PathBuf,
    strings: Vec<String>,
}

impl TextPool {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        Self::parse(&bytes, path)
    }

    pub fn parse(bytes: &[u8], path: &Path) -> Result<Self> {
        if bytes.len() < 0x20 {
            return Err(Error::BadData(
                "text pool is shorter than 0x20 bytes".into(),
            ));
        }
        let header_size = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        if header_size < 0x19 {
            return Err(Error::BadData(format!("header size {header_size:#x}")));
        }
        let encoding = bytes[0x18];
        if encoding != UTF8_ENCODING && encoding != ASCII_ENCODING {
            return Err(Error::Unsupported {
                what: "text pool encoding",
                detail: format!("byte {encoding} at 0x18 is not UTF-8"),
            });
        }

        let io = BytesReader::from(bytes);
        let root = <Fms as KStruct>::read_into::<_, Fms>(&io, None, None)?;
        let handles = root.strings();
        let mut strings = Vec::with_capacity(handles.len());
        for handle in handles.iter() {
            strings.push(handle.value().clone());
        }

        Ok(Self {
            path: path.to_path_buf(),
            strings,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    pub fn get(&self, id: u32) -> Option<&str> {
        self.strings.get(id as usize).map(String::as_str)
    }
}

pub fn locate(script: &Path) -> Option<PathBuf> {
    let mut dir = script.parent();
    for _ in 0..DEPTH {
        let current = dir?;
        let candidate = current.join(POOL_DIR).join(POOL_FILE);
        if candidate.is_file() {
            return Some(candidate);
        }
        dir = current.parent();
    }
    None
}
