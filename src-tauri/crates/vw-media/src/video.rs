//! USM（CRI Sofdec2）视频索引。
//!
//! `.usm` 最大两亿字节，整读进内存不现实，所以全程流式：
//! 先顺序扫块头建块表，再顺序扫 `@SFV` 载荷里的 AUD（`00 00 00 01 09`）建帧表。
//! 前端按 GOP 取字节，交给 WebCodecs 解码。
//!
//! 载荷不做任何 XOR 掩码：语料里 NAL 头（`67 64` = High）本身就是合法的，
//! 且整条流拼起来与 `cridecoder` 的抽取结果逐字节一致。

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use crate::audio::AudioError;

/// 块头长度：`magic(4) + size(4) + 0x18 参数`。
const CHUNK_HEADER: usize = 0x20;
/// 扫描窗口：块之间只需保留 3 字节，防止起始码跨块。
const CARRY: usize = 3;
const MAX_CHUNKS: usize = 2_000_000;
const MAX_FRAMES: usize = 4_000_000;

#[derive(Clone, Copy, Debug)]
pub struct VideoChunk {
    pub file_offset: u64,
    /// 载荷长度（已扣掉 0x20 块头）。
    pub payload: u32,
    /// 该块载荷在拼接流里的起始位置。
    pub stream_offset: u64,
    /// 块头里的帧号（大端 u32 @ +0x0C）。
    pub frame_no: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct VideoFrame {
    pub stream_offset: u64,
    pub size: u32,
    pub key: bool,
}

/// 一条 USM 的视频索引。
pub struct UsmIndex {
    pub width: u32,
    pub height: u32,
    /// `avc1.640028` 形式。
    pub codec: String,
    pub profile: u8,
    pub level: u8,
    /// 块头里的帧率 ×100（2997 = 29.97）。
    pub fps_centi: u32,
    /// SPS / PPS 原始字节（构造 avcC 用）。
    pub sps: Vec<u8>,
    pub pps: Vec<u8>,
    pub chunks: Vec<VideoChunk>,
    pub frames: Vec<VideoFrame>,
    /// 拼接流总长。
    pub stream_len: u64,
    /// `@SFA` 载荷的编码：adx / hca / none。
    pub audio: String,
}

impl UsmIndex {
    /// 关键帧序号（按帧表下标）。
    pub fn keyframes(&self) -> Vec<usize> {
        self.frames
            .iter()
            .enumerate()
            .filter(|(_, frame)| frame.key)
            .map(|(index, _)| index)
            .collect()
    }

    /// 取拼接流里 `[from, from+len)` 区间的字节。
    pub fn read_range(&self, path: &Path, from: u64, len: u64) -> Result<Vec<u8>, AudioError> {
        let to = from.saturating_add(len).min(self.stream_len);
        if to <= from {
            return Ok(Vec::new());
        }
        let mut out = Vec::with_capacity((to - from) as usize);
        let mut file = BufReader::with_capacity(1 << 20, File::open(path)?);
        for chunk in &self.chunks {
            let chunk_start = chunk.stream_offset;
            let chunk_end = chunk_start + chunk.payload as u64;
            if chunk_end <= from || chunk_start >= to {
                continue;
            }
            let payload_start = chunk.file_offset + CHUNK_HEADER as u64;
            let skip = from.saturating_sub(chunk_start);
            let take = (chunk_end.min(to) - chunk_start.max(from)) as usize;
            file.seek(SeekFrom::Start(payload_start + skip))?;
            let mut buf = vec![0u8; take];
            file.read_exact(&mut buf)?;
            out.extend_from_slice(&buf);
        }
        Ok(out)
    }
}

/// 打开并扫描 USM：块表、帧表、SPS 宽高。
pub fn open(path: &Path) -> Result<UsmIndex, AudioError> {
    let mut file = BufReader::with_capacity(1 << 20, File::open(path)?);
    let mut head = [0u8; 0x800];
    let read = file.read(&mut head)?;
    if read < 0x20 || &head[..4] != b"CRID" {
        return Err(AudioError::Malformed("不是 USM（缺少 CRID）".to_string()));
    }
    let header_size = u32_be(&head, 4)
        .map(|value| value as usize + 8)
        .unwrap_or(0);
    if header_size < 0x20 {
        return Err(AudioError::Malformed(format!(
            "USM 头长不成立：{header_size}"
        )));
    }
    let _ = read;

    let file_len = file.seek(SeekFrom::End(0))?;
    file.seek(SeekFrom::Start(header_size as u64))?;

    let mut chunks: Vec<VideoChunk> = Vec::new();
    let mut cursor = header_size as u64;
    let mut stream_offset = 0u64;
    let mut fps_milli = 0u32;
    let mut audio = "none".to_string();

    while cursor + 8 <= file_len {
        file.seek(SeekFrom::Start(cursor))?;
        let mut chunk_head = [0u8; 8];
        file.read_exact(&mut chunk_head)?;
        if chunk_head[0] != b'@' {
            break;
        }
        let size = u32_be(&chunk_head, 4).unwrap_or(0) as u64;
        if size < CHUNK_HEADER as u64 || cursor + size > file_len {
            break;
        }
        // 块头的 0x18 参数里：+0x0C 帧号、+0x10 保留、+0x14 帧率 ×1000。
        let mut param = [0u8; 0x18];
        file.seek(SeekFrom::Start(cursor + 8))?;
        file.read_exact(&mut param)?;
        if fps_milli == 0 {
            let candidate = u32_be(&param, 0x0c).unwrap_or(0);
            if (500..=12_000).contains(&candidate) {
                fps_milli = candidate;
            }
        }
        let frame_no = u32_be(&param, 4).unwrap_or(0);

        match &chunk_head[..4] {
            b"@SFV" => {
                // header / metadata 块（payload 以 `@UTF` 或 `#` 开头）不是码流，
                // 里面的伪起始码会把 SPS 认错。
                let mut probe = [0u8; 4];
                file.seek(SeekFrom::Start(cursor + CHUNK_HEADER as u64))?;
                let probed = file.read(&mut probe).unwrap_or(0);
                let is_table = probed >= 4 && (&probe == b"@UTF" || probe[0] == b'#');
                if is_table {
                    cursor += size.div_ceil(CHUNK_HEADER as u64) * CHUNK_HEADER as u64;
                    continue;
                }
                chunks.push(VideoChunk {
                    file_offset: cursor,
                    payload: (size - CHUNK_HEADER as u64) as u32,
                    stream_offset,
                    frame_no,
                });
                stream_offset += size - CHUNK_HEADER as u64;
            }
            b"@SFA" => {
                if audio == "none" {
                    let mut probe = [0u8; 4];
                    file.seek(SeekFrom::Start(cursor + CHUNK_HEADER as u64))?;
                    let probed = file.read(&mut probe).unwrap_or(0);
                    let is_table = probed >= 4 && (&probe == b"@UTF" || probe[0] == b'#');
                    if is_table {
                        // header / metadata 块，音频数据在后面的块里。
                    } else if probed == 4 {
                        audio = if probe.starts_with(b"HCA\0") {
                            "hca".to_string()
                        } else if probe[0] == 0x80 && probe[1] == 0x00 {
                            "adx".to_string()
                        } else {
                            "unknown".to_string()
                        };
                    }
                }
            }
            b"@END" => break,
            _ => {}
        }

        cursor += size.div_ceil(CHUNK_HEADER as u64) * CHUNK_HEADER as u64;
        if chunks.len() > MAX_CHUNKS {
            return Err(AudioError::Malformed("块数超过上限".to_string()));
        }
    }

    if chunks.is_empty() {
        return Err(AudioError::Malformed("USM 里没有 @SFV 块".to_string()));
    }

    let stream_len = stream_offset;
    let (frames, sps, pps) = scan_frames(&mut file, &chunks, stream_len)?;
    let (width, height, profile, level) = parse_sps(&sps).ok_or_else(|| {
        let head: String = sps
            .iter()
            .take(12)
            .map(|value| format!("{value:02x}"))
            .collect::<Vec<_>>()
            .join(" ");
        AudioError::Malformed(format!("SPS 解析失败（{} B：{head}）", sps.len()))
    })?;
    let codec = format!("avc1.{profile:02x}00{level:02x}");

    Ok(UsmIndex {
        width,
        height,
        codec,
        profile,
        level,
        fps_centi: if fps_milli == 0 { 3_000 } else { fps_milli },
        sps,
        pps,
        chunks,
        frames,
        stream_len,
        audio,
    })
}

/// 顺序扫 `@SFV` 载荷，按 AUD 切帧，并记住首个 SPS / PPS。
fn scan_frames(
    file: &mut BufReader<File>,
    chunks: &[VideoChunk],
    stream_len: u64,
) -> Result<(Vec<VideoFrame>, Vec<u8>, Vec<u8>), AudioError> {
    let mut frames: Vec<VideoFrame> = Vec::new();
    let mut sps: Vec<u8> = Vec::new();
    let mut pps: Vec<u8> = Vec::new();
    // 上一块末尾 3 字节，避免起始码跨块被漏掉。
    let mut carry: Vec<u8> = Vec::new();
    let mut position = 0u64;

    for chunk in chunks {
        file.seek(SeekFrom::Start(chunk.file_offset + CHUNK_HEADER as u64))?;
        let mut body = vec![0u8; chunk.payload as usize];
        file.read_exact(&mut body)?;

        let base = position - carry.len() as u64;
        let mut window = carry.clone();
        window.extend_from_slice(&body);

        // AUD：`00 00 00 01 09`，落在 4 字节起始码上。
        let mut index = 0usize;
        while index + 5 <= window.len() {
            let hit = window[index] == 0
                && window[index + 1] == 0
                && window[index + 2] == 0
                && window[index + 3] == 1
                && (window[index + 4] & 0x1f) == 9;
            if !hit {
                index += 1;
                continue;
            }
            let absolute = base + index as u64;
            if let Some(previous) = frames.last_mut() {
                previous.size = (absolute - previous.stream_offset) as u32;
            }
            // 帧内的第一个 IDR / SPS 决定它是不是关键帧。
            let tail = &window[index..];
            let key = find_nal(tail, 5).is_some() || find_nal(tail, 7).is_some();
            frames.push(VideoFrame {
                stream_offset: absolute,
                size: 0,
                key,
            });
            index += 5;
        }

        if sps.is_empty() {
            if let Some(nal) = find_nal(&window, 7) {
                sps = nal;
            }
            if let Some(nal) = find_nal(&window, 8) {
                pps = nal;
            }
        }

        position += chunk.payload as u64;
        carry = window[window.len().saturating_sub(CARRY)..].to_vec();
        if frames.len() > MAX_FRAMES {
            return Err(AudioError::Malformed("帧数超过上限".to_string()));
        }
    }

    if let Some(last) = frames.last_mut() {
        last.size = (stream_len.saturating_sub(last.stream_offset)) as u32;
    }
    if frames.is_empty() {
        return Err(AudioError::Malformed("流里没有 AUD，无法切帧".to_string()));
    }
    Ok((frames, sps, pps))
}

/// 在窗口里找指定类型的 NAL（3 或 4 字节起始码），返回 NAL 本体（不含起始码）。
fn find_nal(buffer: &[u8], nal_type: u8) -> Option<Vec<u8>> {
    let mut index = 0usize;
    while index + 5 <= buffer.len() {
        let start = if buffer[index] == 0 && buffer[index + 1] == 0 && buffer[index + 2] == 1 {
            Some(index + 3)
        } else if index + 6 <= buffer.len()
            && buffer[index] == 0
            && buffer[index + 1] == 0
            && buffer[index + 2] == 0
            && buffer[index + 3] == 1
        {
            Some(index + 4)
        } else {
            None
        };
        let Some(header_at) = start else {
            index += 1;
            continue;
        };
        if buffer[header_at] & 0x1f != nal_type {
            index = header_at;
            continue;
        }
        // NAL 到下一个起始码为止。
        let mut end = header_at + 1;
        while end + 3 <= buffer.len() {
            if buffer[end] == 0
                && buffer[end + 1] == 0
                && (buffer[end + 2] == 1
                    || (end + 3 < buffer.len() && buffer[end + 2] == 0 && buffer[end + 3] == 1))
            {
                break;
            }
            end += 1;
        }
        return Some(buffer[header_at..end].to_vec());
    }
    None
}

/// SPS 解析（ITU-T H.264 7.3.2.1.1），只取宽高与 profile/level。
fn parse_sps(sps: &[u8]) -> Option<(u32, u32, u8, u8)> {
    if sps.len() < 4 {
        return None;
    }
    let profile = sps[1];
    let level = sps[3];
    let mut reader = BitReader::new(&sps[4..]);
    let _sps_id = reader.ue()?;
    let mut chroma_format_idc = 1u32;
    if matches!(
        profile,
        100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 | 138 | 139 | 134 | 135
    ) {
        chroma_format_idc = reader.ue()?;
        if chroma_format_idc == 3 {
            reader.read_bit()?;
        }
        let _bit_depth_luma = reader.ue()?;
        let _bit_depth_chroma = reader.ue()?;
        reader.read_bit()?;
        if reader.read_bit()? == 1 {
            let count = if chroma_format_idc == 3 { 12 } else { 8 };
            for index in 0..count {
                if reader.read_bit()? == 1 {
                    let size = if index < 6 { 16 } else { 64 };
                    skip_scaling_list(&mut reader, size)?;
                }
            }
        }
    }
    let _log2_max_frame_num = reader.ue()?;
    let pic_order_cnt_type = reader.ue()?;
    if pic_order_cnt_type == 0 {
        let _log2_max_poc = reader.ue()?;
    } else if pic_order_cnt_type == 1 {
        reader.read_bit()?;
        let _offset_non_ref = reader.se()?;
        let _offset_top_bottom = reader.se()?;
        let count = reader.ue()?;
        for _ in 0..count.min(256) {
            let _offset = reader.se()?;
        }
    }
    let _max_num_ref_frames = reader.ue()?;
    reader.read_bit()?;
    let width_mbs = reader.ue()?.checked_add(1)?;
    let height_units = reader.ue()?.checked_add(1)?;
    let frame_mbs_only = reader.read_bit()?;
    if frame_mbs_only == 0 {
        reader.read_bit()?;
    }
    reader.read_bit()?;
    let (mut crop_left, mut crop_right, mut crop_top, mut crop_bottom) = (0u32, 0u32, 0u32, 0u32);
    if reader.read_bit()? == 1 {
        crop_left = reader.ue()?;
        crop_right = reader.ue()?;
        crop_top = reader.ue()?;
        crop_bottom = reader.ue()?;
    }
    let width = width_mbs * 16;
    let height = (2 - frame_mbs_only) * height_units * 16;
    let (sub_width, sub_height) = match chroma_format_idc {
        1 => (2u32, 2u32),
        2 => (2, 1),
        3 => (1, 1),
        _ => (1, 1),
    };
    let crop_unit_x = if chroma_format_idc == 0 { 1 } else { sub_width };
    let crop_unit_y = if chroma_format_idc == 0 {
        2 - frame_mbs_only
    } else {
        sub_height * (2 - frame_mbs_only)
    };
    let width = width.saturating_sub((crop_left + crop_right) * crop_unit_x);
    let height = height.saturating_sub((crop_top + crop_bottom) * crop_unit_y);
    Some((width, height, profile, level))
}

fn skip_scaling_list(reader: &mut BitReader<'_>, size: usize) -> Option<()> {
    let mut last = 8i32;
    let mut next = 8i32;
    for _ in 0..size {
        if next != 0 {
            let delta = reader.se()?;
            next = (last + delta + 256) % 256;
        }
        if next != 0 {
            last = next;
        }
    }
    Some(())
}

struct BitReader<'a> {
    data: &'a [u8],
    bit: usize,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, bit: 0 }
    }

    fn read_bit(&mut self) -> Option<u32> {
        let byte = self.data.get(self.bit / 8)?;
        let value = (byte >> (7 - (self.bit % 8))) & 1;
        self.bit += 1;
        Some(value as u32)
    }

    /// 无符号 Exp-Golomb。
    fn ue(&mut self) -> Option<u32> {
        let mut zeros = 0u32;
        while self.read_bit()? == 0 {
            zeros += 1;
            if zeros > 32 {
                return None;
            }
        }
        let mut value = 0u32;
        for _ in 0..zeros {
            value = (value << 1) | self.read_bit()?;
        }
        Some(((1u64 << zeros) - 1 + value as u64) as u32)
    }

    /// 有符号 Exp-Golomb。
    fn se(&mut self) -> Option<i32> {
        let value = self.ue()? as i64;
        let mapped = if value % 2 == 0 {
            -(value / 2)
        } else {
            (value + 1) / 2
        };
        Some(mapped as i32)
    }
}

fn u32_be(bytes: &[u8], at: usize) -> Option<u32> {
    bytes
        .get(at..at + 4)
        .map(|slice| u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 1920×1080 的一段真实 SPS（profile 100 / level 40，带 crop）。
    const SPS_1080: &[u8] = &[
        0x67, 0x64, 0x00, 0x28, 0xac, 0xb6, 0x03, 0xc0, 0x11, 0x36, 0x02, 0x20, 0x00, 0x00, 0x0c,
        0x80, 0x00, 0x02, 0xed, 0x5c, 0x00, 0x00,
    ];

    #[test]
    fn parses_sps_dimensions() {
        let (width, height, profile, level) = parse_sps(SPS_1080).expect("SPS 应当可解");
        // 编码高度按 16 对齐，1080p 的 SPS 里就是 1088 且没有 crop。
        assert_eq!((width, height), (1920, 1088));
        assert_eq!((profile, level), (100, 40));
    }

    #[test]
    fn finds_nal_after_start_code() {
        let mut buffer = vec![0u8, 0, 0, 1, 0x09, 0x10, 0, 0, 1, 0x67, 0x64, 0x00, 0x28];
        buffer.extend_from_slice(&[0, 0, 1, 0x68, 0xee]);
        let sps = find_nal(&buffer, 7).expect("应当找到 SPS");
        assert_eq!(sps[0] & 0x1f, 7);
        assert_eq!(sps.len(), 4);
    }

    #[test]
    fn reads_exp_golomb() {
        // 0b1 -> 0, 0b010 -> 1, 0b011 -> 2
        let mut reader = BitReader::new(&[0b1_010_011_0]);
        assert_eq!(reader.ue(), Some(0));
        assert_eq!(reader.ue(), Some(1));
        assert_eq!(reader.ue(), Some(2));
    }
}
