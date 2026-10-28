//! Switch 平台 FTEX：`FTEX` 容器 + `FTX0` 块 + `.tex`(NVT)。
//!
//! 结构与 `SwitchFtexParser.cs` 逐字段等价；像素处理顺序也保持一致：
//! 先按格式解码成像素，再做 Tegra X1 块置换，最后裁掉 `ceilPow2(h)` 带来的补行。
//!
//! 不支持的像素格式按旧实现的行为**跳过**（不报错、不占帧索引）。

use kaitai::{BytesReader, KStruct};
use schema::gen::ftex_switch::{FtexSwitch, FtexSwitch_FtxBlock, FtexSwitch_TextureEntry};

use super::ceil_pow2;
use crate::codec::{bc, tegra_x1};
use crate::{DecodedTexture, Error, Result};

pub const FMT_BC3: u16 = 0x44;
pub const FMT_BC4: u16 = 0x49;
pub const FMT_BC7: u16 = 0x4d;

/// 该格式的输出通道数；不支持的格式返回 `None`。
pub fn channels_of(format: u16) -> Option<u8> {
    match format {
        FMT_BC3 | FMT_BC7 => Some(4),
        FMT_BC4 => Some(1),
        _ => None,
    }
}

pub fn decode(data: &[u8]) -> Result<Vec<DecodedTexture>> {
    let io = BytesReader::from(data);
    let root = <FtexSwitch as KStruct>::read_into::<_, FtexSwitch>(&io, None, None)?;

    let entries = root.entries();
    let blocks = root.blocks();
    let mut out = Vec::with_capacity(blocks.len());

    for (index, block) in blocks.iter().enumerate() {
        if let Some(texture) = decode_block(block, texture_name(&entries, index))? {
            out.push(texture);
        }
    }

    Ok(out)
}

/// 只解码第 `index` 个**可解码**的帧（跳过不支持的格式与 0 尺寸块）。
pub fn decode_frame(data: &[u8], index: usize) -> Result<DecodedTexture> {
    let io = BytesReader::from(data);
    let root = <FtexSwitch as KStruct>::read_into::<_, FtexSwitch>(&io, None, None)?;

    let entries = root.entries();
    let blocks = root.blocks();
    let mut seen = 0usize;

    for (block_index, block) in blocks.iter().enumerate() {
        if channels_of(*block.nvt().format()).is_none() {
            continue;
        }

        if seen == index {
            return decode_block(block, texture_name(&entries, block_index))?
                .ok_or_else(|| Error::BadData(format!("frame {index} is empty")));
        }

        seen += 1;
    }

    Err(Error::BadData(format!(
        "frame index {index} out of range (only {seen} decodable frames)"
    )))
}

fn texture_name(entries: &[kaitai::OptRc<FtexSwitch_TextureEntry>], index: usize) -> String {
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

fn decode_block(block: &FtexSwitch_FtxBlock, name: String) -> Result<Option<DecodedTexture>> {
    let nvt = block.nvt();
    let width = *nvt.width() as usize;
    let height = *nvt.height() as usize;
    let format = *nvt.format();

    if width == 0 || height == 0 {
        return Ok(None);
    }

    let Some(channels) = channels_of(format) else {
        return Ok(None);
    };

    let pixels = nvt.data();
    let padded_h = ceil_pow2(height);

    let data = match format {
        FMT_BC3 => {
            let raw = bc::decode_bc3(&pixels);
            let swizzled = tegra_x1::swizzle_16b(&raw, width as i32, padded_h as i32);
            crop(&swizzled, width, height, padded_h, 4)
        }
        FMT_BC7 => {
            // texture2ddecoder 输出 u32（0xAARRGGBB），这里转成 RGBA8 字节序。
            // 与 PHP 参考一致：解码覆盖补行后的全部块（数据不足时按实际长度截断），
            // 缺失的补行块在重排后保持 0。
            // 压缩数据每 16 字节一个 4x4 块：每像素行的压缩字节数为 `(width/4)*16`，
            // 由此换算解码像素行数，与 PHP `bptc_texture::bc7()` 解码全部块的行为一致。
            let bw = (width / 4).max(1);
            let dec_h = (pixels.len() / 16 / bw * 4).min(padded_h);
            let mut packed = vec![0u32; width * dec_h];
            texture2ddecoder::decode_bc7(&pixels, width, dec_h, &mut packed)
                .map_err(|e| Error::BadData(format!("BC7 decode failed: {e}")))?;

            let mut image = Vec::with_capacity(packed.len() * 4);
            for px in packed {
                image.push(((px >> 16) & 0xFF) as u8);
                image.push(((px >> 8) & 0xFF) as u8);
                image.push((px & 0xFF) as u8);
                image.push(((px >> 24) & 0xFF) as u8);
            }

            // PHP 的 bptc_texture::bc7() 输出**块连续序列**（每块 4x4 像素行主序
            // 连续，共 64 字节），swizzle 按此顺序消费；而 texture2ddecoder 输出
            // 行主序整图。这里重排成块序列，对齐 PHP 数据流。
            let raw = pack_blocks(&image, width, dec_h);

            let swizzled = tegra_x1::swizzle_16b(&raw, width as i32, padded_h as i32);
            crop(&swizzled, width, height, padded_h, 4)
        }
        FMT_BC4 => {
            let mono = bc::decode_bc4(&pixels);
            let swizzled = tegra_x1::swizzle_8b(&mono, width as i32, padded_h as i32);
            crop(&swizzled, width, height, padded_h, 1)
        }
        _ => unreachable!("channels_of already filtered"),
    };

    Ok(Some(DecodedTexture {
        name,
        width: width as u32,
        height: height as u32,
        format,
        channels,
        data,
    }))
}

/// 行主序 RGBA 图 → 块连续序列：块 `(bx, by)` 的 4 行像素各 16 字节依次排列，
/// 对齐 PHP `bptc_texture::bc7()` 的输出语义。
fn pack_blocks(image: &[u8], width: usize, height: usize) -> Vec<u8> {
    let bw = width / 4;
    let bh = height / 4;
    let mut out = vec![0u8; bw * bh * 64];

    for by in 0..bh {
        for bx in 0..bw {
            for row in 0..4 {
                let src = ((by * 4 + row) * width + bx * 4) * 4;
                let dst = ((by * bw + bx) * 4 + row) * 16;
                out[dst..dst + 16].copy_from_slice(&image[src..src + 16]);
            }
        }
    }

    out
}

/// 去掉补到 2 的幂高度带来的多余行。
fn crop(padded: &[u8], width: usize, height: usize, padded_h: usize, channels: usize) -> Vec<u8> {
    let stride = width * channels;
    let needed = stride * height;

    if padded.len() >= needed && padded_h >= height {
        return padded[..needed].to_vec();
    }

    // 数据不足时补零，避免下游 panic（异常文件才会走到这里）。
    let mut out = vec![0u8; needed];
    let copy = padded.len().min(needed);
    out[..copy].copy_from_slice(&padded[..copy]);
    out
}
