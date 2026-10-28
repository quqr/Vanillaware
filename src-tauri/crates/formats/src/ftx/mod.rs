pub mod switch;

use std::path::Path;

use kaitai::{BytesReader, KStruct};
use schema::gen::ftex_index::{FtexIndex, FtexIndex_TextureEntry};

use crate::codec::fcmp;
use crate::{DecodedTexture, Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub format: u16,
    pub channels: u8,
}

impl TextureInfo {
    pub fn format_name(&self) -> &'static str {
        match self.format {
            switch::FMT_BC3 => "BC3",
            switch::FMT_BC4 => "BC4",
            switch::FMT_BC7 => "BC7",
            _ => "UNKNOWN",
        }
    }
}

pub fn probe_file(path: &Path) -> Result<Vec<TextureInfo>> {
    let bytes = std::fs::read(path)?;
    probe_bytes(&bytes)
}

pub fn probe_bytes(bytes: &[u8]) -> Result<Vec<TextureInfo>> {
    let data = unwrap_fcmp(bytes)?;

    if data.len() < 4 {
        return Err(Error::BadData("file shorter than 4 bytes".into()));
    }

    match &data[..4] {
        b"FTEX" => probe_switch(data.as_ref()),
        other => Err(Error::Unsupported {
            what: "ftx container",
            detail: format!("magic {:?}", String::from_utf8_lossy(other)),
        }),
    }
}

pub fn decode_file_frame(path: &Path, index: usize) -> Result<DecodedTexture> {
    let bytes = std::fs::read(path)?;
    decode_bytes_frame(&bytes, index)
}

pub fn decode_bytes_frame(bytes: &[u8], index: usize) -> Result<DecodedTexture> {
    let data = unwrap_fcmp(bytes)?;

    if data.len() < 4 {
        return Err(Error::BadData("file shorter than 4 bytes".into()));
    }

    match &data[..4] {
        b"FTEX" => switch::decode_frame(data.as_ref(), index),
        other => Err(Error::Unsupported {
            what: "ftx container",
            detail: format!("magic {:?}", String::from_utf8_lossy(other)),
        }),
    }
}

pub fn decode_file(path: &Path) -> Result<Vec<DecodedTexture>> {
    let bytes = std::fs::read(path)?;
    decode_bytes(&bytes)
}

pub fn decode_bytes(bytes: &[u8]) -> Result<Vec<DecodedTexture>> {
    let data = unwrap_fcmp(bytes)?;

    if data.len() < 4 {
        return Err(Error::BadData("file shorter than 4 bytes".into()));
    }

    match &data[..4] {
        b"FTEX" => switch::decode(data.as_ref()),
        other => Err(Error::Unsupported {
            what: "ftx container",
            detail: format!("magic {:?}", String::from_utf8_lossy(other)),
        }),
    }
}

fn unwrap_fcmp(bytes: &[u8]) -> Result<std::borrow::Cow<'_, [u8]>> {
    if !fcmp::is_fcmp(bytes) {
        return Ok(std::borrow::Cow::Borrowed(bytes));
    }

    let decoded = fcmp::decode(bytes)?;
    if decoded.is_empty() {
        return Err(Error::BadData(
            "FCMP header decoded to empty payload".into(),
        ));
    }

    Ok(std::borrow::Cow::Owned(decoded))
}

fn probe_switch(data: &[u8]) -> Result<Vec<TextureInfo>> {
    let io = BytesReader::from(data);
    let root = <FtexIndex as KStruct>::read_into::<_, FtexIndex>(&io, None, None)?;

    let entries = root.entries();
    let blocks = root.blocks();
    let mut out = Vec::with_capacity(blocks.len());

    for (index, block) in blocks.iter().enumerate() {
        let nvt = block.nvt();
        let width = *nvt.width();
        let height = *nvt.height();
        let format = *nvt.format();

        if width == 0 || height == 0 {
            continue;
        }

        let Some(channels) = switch::channels_of(format) else {
            continue;
        };

        out.push(TextureInfo {
            name: texture_name(&entries, index),
            width,
            height,
            format,
            channels,
        });
    }

    Ok(out)
}

fn texture_name(entries: &[kaitai::OptRc<FtexIndex_TextureEntry>], index: usize) -> String {
    entries
        .get(index)
        .map(|e| {
            String::from_utf8_lossy(&e.name())
                .trim_end_matches('\0')
                .to_string()
        })
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| format!("texture{index}"))
}

pub(crate) fn ceil_pow2(value: usize) -> usize {
    if value == 0 {
        return 1;
    }

    let mut ceil = 1usize;
    while ceil < value {
        ceil <<= 1;
    }
    ceil
}
