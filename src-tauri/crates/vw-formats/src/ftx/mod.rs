//! FTEX 纹理容器。目前实现 Switch（`FTEX` + `FTX0` + `.tex`），其余平台待有语料再补。

pub mod switch;

use std::path::Path;

use kaitai::{BytesReader, KStruct};
use vw_gen::gen::vw_ftx_index::{VwFtxIndex, VwFtxIndex_TextureEntry};

use crate::codec::fcmp;
use crate::{DecodedTexture, Error, Result};

/// 只描述结构的帧信息（不含像素）。用于帧数徽标与计数排序。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub format: u16,
    pub channels: u8,
}

impl TextureInfo {
    /// 人类可读的格式名，例如 `BC7`。
    pub fn format_name(&self) -> &'static str {
        match self.format {
            switch::FMT_BC3 => "BC3",
            switch::FMT_BC4 => "BC4",
            switch::FMT_BC7 => "BC7",
            _ => "UNKNOWN",
        }
    }
}

/// 只读结构，不读像素数据、不做 BC 解码。
///
/// 索引与 [`decode_file_frame`] 严格对齐：两者都跳过不支持的格式与 0 尺寸块。
pub fn probe_file(path: &Path) -> Result<Vec<TextureInfo>> {
    let bytes = std::fs::read(path)?;
    probe_bytes(&bytes)
}

/// 见 [`probe_file`]。
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

/// 只解码指定帧。缩略图必须走这个，禁止为了第 0 帧解码整个文件。
pub fn decode_file_frame(path: &Path, index: usize) -> Result<DecodedTexture> {
    let bytes = std::fs::read(path)?;
    decode_bytes_frame(&bytes, index)
}

/// 见 [`decode_file_frame`]。
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

/// 从文件读取并解码全部纹理。会自动处理 FCMP 压缩外壳。
pub fn decode_file(path: &Path) -> Result<Vec<DecodedTexture>> {
    let bytes = std::fs::read(path)?;
    decode_bytes(&bytes)
}

/// 从内存字节解码。会自动处理 FCMP 压缩外壳。
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

/// FCMP 解压外壳；未压缩时直接借用原切片。
fn unwrap_fcmp(bytes: &[u8]) -> Result<std::borrow::Cow<'_, [u8]>> {
    if !fcmp::is_fcmp(bytes) {
        return Ok(std::borrow::Cow::Borrowed(bytes));
    }

    let decoded = fcmp::decode(bytes)?;
    if decoded.is_empty() {
        return Err(Error::BadData("FCMP header decoded to empty payload".into()));
    }

    Ok(std::borrow::Cow::Owned(decoded))
}

fn probe_switch(data: &[u8]) -> Result<Vec<TextureInfo>> {
    let io = BytesReader::from(data);
    let root = <VwFtxIndex as KStruct>::read_into::<_, VwFtxIndex>(&io, None, None)?;

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

fn texture_name(entries: &[kaitai::OptRc<VwFtxIndex_TextureEntry>], index: usize) -> String {
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

/// 大于等于 `value` 的最小 2 的幂（对应 C# `ByteHelper.IntCeilPow2`）。
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
